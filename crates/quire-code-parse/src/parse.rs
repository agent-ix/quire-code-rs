//! The one parse entry point this crate has.
//!
//! Implements [FR-013](../../../spec/functional/FR-013-borrowed-parse-tree-api.md).

use tree_sitter::{Node, Parser, Tree};

use crate::error::{Diagnostic, ParseError};
use crate::language::Language;

/// A parsed source file: the syntax tree, and the text it borrows from.
///
/// # What this type does not do
///
/// It does not classify a single node, attribute a declaration to a symbol
/// kind, bind a marker to a declaration, or compute an identity for anything
/// in the tree. Reading `(language, path, qualified_name, kind)` — or any
/// other identity — out of a node's span or byte offsets is the consumer's
/// own construction; this type intentionally exposes no helper that invites
/// deriving one, because the tree it hands back is not itself an identity
/// source (per this crate's own boundary — see the crate-level docs).
///
/// # Thread safety
///
/// `ParsedFile` is both [`Send`] **and** [`Sync`] at the `tree-sitter`
/// version this crate pins (`0.26`) — inherited from
/// [`tree_sitter::Tree`], which declares both. This is verified by a
/// compiled static assertion in `tests/thread_safety.rs`, not asserted from
/// memory: `Tree` was widely known as `Send`-only in older tree-sitter
/// releases, and that folklore does not hold for the pinned version, so it is
/// stated here as a checked fact rather than repeated as received wisdom.
/// Neither `ParsedFile` nor `Language` add a field that would narrow either
/// bound.
///
/// Two fan-out patterns for parsing a repository in parallel are both sound:
///
/// - **Parse per worker thread.** Send file content to each worker and call
///   [`parse_file`] inside it; each worker's `ParsedFile` can be sent back to
///   a collecting thread (`Send`). This is the pattern to reach for when
///   parsing is the parallel work.
/// - **Share one parsed tree for concurrent reads.** Because `ParsedFile`
///   exposes no `&mut` method and is `Sync`, a `ParsedFile` already produced
///   may be put behind an `Arc` and its `&ParsedFile` hand out to a pool of
///   reader threads for concurrent, read-only traversal. This is the pattern
///   when one parse is reused by several consumers of its tree.
///
/// What remains unsound regardless: mutating a tree from one thread while
/// another reads it. This crate exposes no mutation, so that case cannot
/// arise through this API alone — it is only a hazard if a caller reaches
/// [`tree()`](ParsedFile::tree) and calls a mutating `tree-sitter` method
/// (e.g. `Tree::edit`) on a tree another thread is concurrently reading.
pub struct ParsedFile<'src> {
    tree: Tree,
    source: &'src str,
    file: &'src str,
    language: Language,
    diagnostic: Option<Diagnostic>,
}

impl<'src> ParsedFile<'src> {
    /// The parsed syntax tree.
    ///
    /// `tree-sitter` itself is re-exported by this crate (see the crate-level
    /// docs) rather than wrapped, so [`Tree`], [`Node`] and `TreeCursor` are
    /// tree-sitter's own types — this crate does not reimplement traversal.
    ///
    /// The tree is available here regardless of [`diagnostic`](ParsedFile::diagnostic) —
    /// a declaration-structure error is a loud diagnostic attached to this
    /// value, never a reason this crate withholds the tree it already
    /// produced (FR-013-AC-3, FR-013-AC-9).
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// The tree's root node. A convenience for `self.tree().root_node()`.
    pub fn root_node(&self) -> Node<'_> {
        self.tree.root_node()
    }

    /// The source text this tree was parsed from, borrowed for the lifetime
    /// the caller chose when it called [`parse_file`] (FR-013-AC-2).
    pub fn source(&self) -> &'src str {
        self.source
    }

    /// The file identifier this parse was attributed to, borrowed for the
    /// same lifetime as [`source`](ParsedFile::source) rather than owned —
    /// the caller already holds it, so this crate never allocates a copy.
    pub fn file(&self) -> &'src str {
        self.file
    }

    /// The language this tree was parsed as.
    pub fn language(&self) -> Language {
        self.language
    }

    /// `Some` where the tree's declaration structure is unrecoverable — the
    /// root, or a declaration-list-shaped body nested at any depth, has a
    /// direct child that is itself an `ERROR`/`MISSING` node — naming the
    /// one-based line and zero-based column of the first such position
    /// (FR-013-AC-3). `None` for a clean parse, and also for a tree whose
    /// only errors are body-local: an expression or statement inside one
    /// already-identifiable declaration, which does not cost that
    /// declaration its own kind, name or signature (FR-013-AC-10).
    ///
    /// This is the diagnostic [`parse_file`]'s own docs describe as
    /// non-ignorable in shape rather than in enforcement: the same value,
    /// [`ParsedFile`], is returned whether or not this is `Some`, so a
    /// consumer's own scanner walks the same tree either way and meets the
    /// `ERROR`/`MISSING` node in the position the malformed declaration would
    /// have occupied — not an absence indistinguishable from "this file
    /// declares nothing". Checking this accessor is how a consumer attaches
    /// its own file-level diagnostic to that same position without
    /// re-deriving it from the tree by hand.
    pub fn diagnostic(&self) -> Option<Diagnostic> {
        self.diagnostic
    }
}

impl std::fmt::Debug for ParsedFile<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParsedFile")
            .field("file", &self.file)
            .field("language", &self.language)
            .field("source_len", &self.source.len())
            .field("root_kind", &self.tree.root_node().kind())
            .field("diagnostic", &self.diagnostic)
            .finish()
    }
}

/// Parse `source` as `language`, attributing any diagnostic to `file`.
///
/// No I/O: the caller supplies bytes already in memory and a [`Language`];
/// this crate never reads a path from disk or the network.
///
/// # Errors
///
/// Returns [`ParseError::NoTree`] only in the defensive case where
/// tree-sitter accepts the language but produces no tree at all — no path
/// this crate's own inputs are known to trigger, kept only because
/// `Parser::set_language`/`Parser::parse` return `Result`/`Option` and this
/// crate has no `unwrap`/`expect`/panic to fall back to if that ever stops
/// being true.
///
/// A declaration-structure failure — the root, or a declaration-list-shaped
/// body nested at any depth, has a direct child that is itself an
/// `ERROR`/`MISSING` node, so tree-sitter could not resolve even the identity
/// of a declaration there (FR-013-AC-3) — is **not** an `Err`. It is `Ok`,
/// with [`ParsedFile::diagnostic`] returning `Some`, naming `file` and the
/// one-based line of the first such position. Carrying the tree inside `Err`
/// (an earlier shape of this crate) could not be made `'static`, so a
/// consumer could not box it, convert it with `anyhow`, or collect it across
/// files into a `Vec` outliving any one source buffer — seeing the
/// declaration-structure diagnostic on `Ok` instead of gating it behind
/// `Err` is what lets [`ParseError`] itself own its `file` and be `'static`
/// (PLAT-841 PR #22 review finding FND-005). The tree is never withheld
/// either way: this crate's hard-diagnostic rule was never really about
/// `Result`'s shape, only about a caller being unable to reach a clean parse
/// and an unrecoverable one through the same code path without a name for
/// the difference — `ParsedFile::diagnostic` supplies that name.
///
/// An error nested inside a declaration's own body — a function whose
/// expression is incomplete, mid-edit — leaves that declaration's own node
/// kind, name and signature fully readable, and tree-sitter recovers it
/// locally. That case returns `Ok` with `diagnostic()` returning `None`, the
/// tree fully walkable, and the error still visible to a caller that
/// inspects [`Node::has_error`] itself on the relevant node — this crate
/// states only the loud, structural case as its own diagnostic; a body-local
/// error is not the failure PLAT-14 named, and treating it as one would fail
/// the exact use case (`filament-ide-rs` editing content mid-keystroke) this
/// API exists to serve.
///
/// # Panics
///
/// Never, for any `source` — arbitrary byte content that is valid UTF-8 (the
/// `&str` type already guarantees that) parses or returns `Err`
/// (FR-013-AC-6).
pub fn parse_file<'src>(
    language: Language,
    file: &'src str,
    source: &'src str,
) -> Result<ParsedFile<'src>, ParseError> {
    let mut parser = Parser::new();
    parser
        .set_language(&language.grammar())
        .map_err(|_| ParseError::NoTree {
            file: file.to_string(),
            line: 1,
        })?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| ParseError::NoTree {
            file: file.to_string(),
            line: 1,
        })?;

    let diagnostic = if has_declaration_structure_error(tree.root_node()) {
        let (line, column) = first_error_position(tree.root_node()).unwrap_or((1, 0));
        Some(Diagnostic { line, column })
    } else {
        None
    };

    Ok(ParsedFile {
        tree,
        source,
        file,
        language,
        diagnostic,
    })
}

/// Whether `root`'s own declaration structure is unrecoverable anywhere in
/// the tree, not only at the top level: `root` itself, or a declaration-list-
/// shaped body nested at any depth — a `mod`/`impl`/`trait` body in Rust, a
/// `class` body in Python, a `class`/`namespace` body in TypeScript — has a
/// direct child that is itself an `ERROR` or `MISSING` node, meaning
/// tree-sitter could not resolve even the identity of a declaration there.
///
/// Deliberately narrower than [`Node::has_error`], which is also true for an
/// ordinary, fully-identifiable declaration whose *body* — the executable
/// statements inside a function or method, not a further list of
/// declarations — contains an error several levels down.
///
/// This asks the structural question at *every* nesting level, not only the
/// root's direct children. An earlier, depth-1-only form of this check
/// (PLAT-841 PR #22 review finding FND-001) missed every declaration nested
/// one level inside a `mod`, `impl`, `class` or `namespace` — and since every
/// Python method sits at depth 2 inside `class_definition`, that earlier form
/// structurally could not see a broken Python method at all: the exact
/// PLAT-14 shape ("a genuinely broken file reports nothing") this crate
/// exists to end, reintroduced one layer down. [`is_declaration_container`]
/// is what makes the question well-posed at every depth: it asks, for the
/// node actually containing the error, whether that node is a
/// declaration-list-shaped body (so an `ERROR`/`MISSING` direct child there
/// means an unresolvable declaration) or an ordinary executable body that
/// merely happens to share a node kind with one (so an error inside it is
/// body-local, and does not count) — matching FR-013-AC-10's semantic rule
/// ("its own node kind, name and signature remain resolvable") rather than
/// the position-only rule the depth-1 form implemented.
fn has_declaration_structure_error(root: Node<'_>) -> bool {
    if root.is_error() || root.is_missing() {
        return true;
    }
    node_has_declaration_structure_error(root)
}

/// `node.has_error()` is `false` for a subtree with no error anywhere
/// beneath it, so this short-circuits without walking such a subtree at all
/// — the same property [`first_error_position`] already relies on to search
/// to full depth cheaply.
///
/// Where an error does exist somewhere below `node`: if `node` is itself a
/// declaration container ([`is_declaration_container`]), its direct children
/// are checked for `ERROR`/`MISSING` — the structural question, asked again
/// at this level. Every child is still walked afterward regardless, since a
/// declaration can be nested inside a non-container body too (Rust, Python
/// and TypeScript all allow a `mod`, `class` or function to be defined inside
/// a function), so a nested container several levels down a body that is
/// itself not one must still be found.
fn node_has_declaration_structure_error(node: Node<'_>) -> bool {
    if !node.has_error() {
        return false;
    }
    if is_declaration_container(node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_error() || child.is_missing() {
                return true;
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if node_has_declaration_structure_error(child) {
            return true;
        }
    }
    false
}

/// Whether `node` is itself a declaration-list-shaped body: a container whose
/// direct children are meant to be resolvable declarations, not the
/// executable statements of one declaration's own body.
///
/// Verified empirically against Rust, Python and TypeScript's actual
/// grammars (via `to_sexp()` on nested fixtures, not assumed) — including
/// where tree-sitter's error recovery attaches the `ERROR`/`MISSING` node
/// somewhere other than the naive guess:
///
/// - **Unambiguous by node kind alone.** Every one of the three grammars'
///   own root kinds (`source_file`, `module`, `program`); Rust's
///   `declaration_list`, the body kind `mod_item`, `impl_item` and
///   `trait_item` all share, and `field_declaration_list`, a `struct`'s own
///   field list — a struct has no separate executable body to distinguish
///   from its signature the way a function does, so a field tree-sitter
///   could not resolve is exactly as structural as a top-level item it
///   could not resolve; and TypeScript's `class_body`, a distinct kind from
///   a function's or method's own body kind, unlike Python's class body.
/// - **`class_definition` itself, not only its `block`.** A Python method
///   tree-sitter cannot resolve at all (e.g. missing its own name) does not
///   reliably surface as an `ERROR`/`MISSING` child of the class's `block`;
///   empirically, recovery instead attaches it as a direct child of
///   `class_definition` itself, alongside — not inside — `body`. Checking
///   only `block` would miss exactly the case this exists to catch.
/// - **Ambiguous by node kind — resolved by the parent's kind.** Python
///   also reuses `block` for a `function_definition`'s, `if_statement`'s,
///   etc. own (executable) body; TypeScript reuses `statement_block` for
///   both an `internal_module` (a `namespace`) and a `function_declaration`
///   or `method_definition`. The child node's own kind cannot tell these
///   apart — only the parent's kind can, so this checks that instead, for
///   `block`/`statement_block` specifically, as a defensive second check
///   alongside `class_definition` above rather than a replacement for it.
///   (TypeScript's `declare module "..."` ambient-module spelling is not
///   covered here: only `namespace` was verified against the actual
///   grammar.)
fn is_declaration_container(node: Node<'_>) -> bool {
    match node.kind() {
        "source_file"
        | "module"
        | "program"
        | "declaration_list"
        | "class_body"
        | "field_declaration_list"
        | "class_definition" => true,
        "block" | "statement_block" => node
            .parent()
            .is_some_and(|parent| matches!(parent.kind(), "class_definition" | "internal_module")),
        _ => false,
    }
}

/// One-based line and zero-based column of the first error or missing node in
/// `root`'s subtree, if any — searched to full depth so the reported position
/// is the most precise one available, even though
/// [`has_declaration_structure_error`] only inspects declaration-list-shaped
/// bodies to decide *whether* to report at all. Independent of
/// quire-code-rs's own equivalent (this crate depends on nothing there — see
/// FR-013-CON-2): the two exist in separate crates by design, not by
/// omission.
fn first_error_position(root: Node<'_>) -> Option<(u32, u32)> {
    let mut cursor = root.walk();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            let pos = node.start_position();
            return Some((line_number(pos.row), column_number(pos.column)));
        }
        if node.has_error() {
            let children: Vec<_> = node.children(&mut cursor).collect();
            for child in children.into_iter().rev() {
                stack.push(child);
            }
        }
    }
    None
}

/// A one-based line number from tree-sitter's zero-based `usize` row.
///
/// `try_from` before the `+ 1`, not `as u32` — a bare cast truncates
/// *silently*, so a row at `2^32` would report line 1: a confidently wrong
/// answer, and a worse failure than the panic this guards against, since
/// nothing signals it happened. `u32::MAX` is the reported line for a row
/// this crate cannot represent, rather than a wrapped-around small number
/// that reads as a normal position (FR-013-AC-6: this still never panics).
fn line_number(row: usize) -> u32 {
    u32::try_from(row)
        .map(|row| row.saturating_add(1))
        .unwrap_or(u32::MAX)
}

/// A zero-based column number from tree-sitter's `usize` column, with the
/// same truncation guard as [`line_number`].
fn column_number(column: usize) -> u32 {
    u32::try_from(column).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    // FR-013-AC-1 and FR-013-AC-3 are covered end to end, as an external
    // consumer would exercise them, by `tests/integration.rs` (TC-135,
    // TC-136, TC-151) — that file links only this crate's public API, which
    // is the more meaningful proof of the boundary than a same-crate unit
    // test. These unit tests cover what an external test cannot see
    // directly: the borrow's pointer identity and the tree's internal
    // determinism.

    #[cfg(feature = "rust")]
    // TC-141, FR-013-AC-2: the returned value borrows the source for the
    // caller's own lifetime, and its slices are readable after the call.
    #[test]
    fn parsed_file_borrows_source_for_the_callers_lifetime() {
        let source = String::from("pub fn kept() {}\n");
        let parsed = parse_file(Language::Rust, "src/lib.rs", &source).expect("parses cleanly");
        // `source` is still owned by this scope; `parsed.source()` reads the
        // same bytes rather than a copy, proving the borrow rather than an
        // internal clone.
        assert_eq!(parsed.source().as_ptr(), source.as_str().as_ptr());
        assert_eq!(parsed.source(), source.as_str());
    }

    #[cfg(feature = "rust")]
    // TC-142, FR-013-AC-5 / determinism: identical input yields a
    // byte-identical tree on every call, within this process.
    #[test]
    fn identical_input_yields_byte_identical_tree() {
        let source =
            "pub struct Store { pub n: u32 }\nimpl Store { pub fn n(&self) -> u32 { self.n } }\n";
        let first = parse_file(Language::Rust, "src/store.rs", source).expect("parses cleanly");
        let second = parse_file(Language::Rust, "src/store.rs", source).expect("parses cleanly");
        assert_eq!(first.root_node().to_sexp(), second.root_node().to_sexp());
        assert_eq!(
            walk_spans(first.root_node()),
            walk_spans(second.root_node())
        );
    }

    #[cfg(feature = "rust")]
    // TC-143: never panics on adversarial byte content that is still valid
    // UTF-8 — empty input, and input that is only punctuation.
    #[test]
    fn never_panics_on_edge_case_input() {
        for source in ["", "}}}}", "\u{0}\u{0}\u{0}", "// just a comment\n"] {
            let _ = parse_file(Language::Rust, "src/edge.rs", source);
        }
    }

    #[cfg(feature = "rust")]
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(256))]

        // TC-156, FR-013-AC-6: no `&str` a caller can construct causes a
        // panic, checked over generated Unicode input (not only the four
        // fixed cases TC-143 hand-picks) — the smoke test can miss a
        // grammar-specific panic path that a wider search would find
        // (PLAT-841 PR #22 review finding 8).
        #[test]
        fn never_panics_on_arbitrary_str_input(source in ".*") {
            let _ = parse_file(Language::Rust, "src/fuzz.rs", &source);
        }
    }

    #[cfg(feature = "rust")]
    // TC-152, FR-013-AC-10: a body-local error — the declaration's own kind,
    // name and signature are still readable — does not trip
    // `has_declaration_structure_error`, even though `has_error()` is true.
    #[test]
    fn body_local_error_does_not_count_as_a_declaration_structure_error() {
        let source = "pub fn broken() -> u32 { 1 + }\npub fn intact() -> u32 { 2 }\n";
        let parsed = parse_file(Language::Rust, "src/lib.rs", source).expect("parses cleanly");
        assert!(parsed.root_node().has_error(), "the body error is present");
        assert!(
            !has_declaration_structure_error(parsed.root_node()),
            "a body-local error must not read as a declaration-structure error"
        );
        assert!(parsed.diagnostic().is_none());
        let mut cursor = parsed.root_node().walk();
        let kinds: Vec<&str> = parsed
            .root_node()
            .children(&mut cursor)
            .map(|n| n.kind())
            .collect();
        assert_eq!(
            kinds,
            vec!["function_item", "function_item"],
            "both declarations keep their own recognizable kind"
        );
    }

    #[cfg(feature = "rust")]
    // TC-153, FR-013-AC-3: an item tree-sitter could not resolve as a
    // declaration at all (truncated mid-declaration, nothing after it to
    // resync against) trips the structural check.
    #[test]
    fn unresolvable_top_level_item_counts_as_a_declaration_structure_error() {
        let source = "pub fn broken(x: u32) -> u32 {\n    x +\n";
        let parsed =
            parse_file(Language::Rust, "src/broken.rs", source).expect("tree still produced");
        assert!(parsed.diagnostic().is_some());
        let mut cursor = parsed.root_node().walk();
        let kinds: Vec<&str> = parsed
            .root_node()
            .children(&mut cursor)
            .map(|n| n.kind())
            .collect();
        assert_eq!(kinds, vec!["ERROR"]);
    }

    #[cfg(feature = "rust")]
    // TC-157, FR-013-AC-3, FR-013-AC-10 (FND-001): a declaration nested one
    // level inside a `mod` that tree-sitter cannot resolve at all (missing
    // its own name) trips the structural check, exactly as an equivalent
    // top-level item would — the check is not a depth-1 rule.
    #[test]
    fn unresolvable_item_nested_in_a_mod_counts_as_a_declaration_structure_error() {
        let source = "mod m {\n    pub fn ( ) { }\n    pub fn intact() {}\n}\n";
        let parsed =
            parse_file(Language::Rust, "src/mod_broken.rs", source).expect("tree still produced");
        assert!(
            parsed.diagnostic().is_some(),
            "a fn missing its own name, nested inside a mod, must still be caught"
        );
    }

    #[cfg(feature = "rust")]
    // TC-158, FR-013-AC-10 (FND-001): a body-local error nested inside a
    // `mod`'s declaration stays `Ok` with no diagnostic — depth alone does
    // not trip the check, only an unresolvable declaration does.
    #[test]
    fn body_local_error_nested_in_a_mod_stays_ok_with_no_diagnostic() {
        let source =
            "mod m {\n    pub fn broken() -> u32 { 1 + }\n    pub fn intact() -> u32 { 2 }\n}\n";
        let parsed =
            parse_file(Language::Rust, "src/mod_body_local.rs", source).expect("parses cleanly");
        assert!(
            parsed.diagnostic().is_none(),
            "a body-local error nested inside a mod must not trip the structural check"
        );
    }

    #[cfg(feature = "python")]
    // TC-159, FR-013-AC-3, FR-013-AC-10 (FND-001): a Python method tree-sitter
    // could not resolve at all (missing its own name) trips the structural
    // check, even though every Python method sits one level inside
    // `class_definition` — the exact shape PLAT-14 named.
    #[test]
    fn unresolvable_method_in_a_class_counts_as_a_declaration_structure_error() {
        let source =
            "class Foo:\n    def (self):\n        pass\n    def intact(self):\n        pass\n";
        let parsed = parse_file(Language::Python, "x.py", source).expect("tree still produced");
        assert!(
            parsed.diagnostic().is_some(),
            "a method missing its own name, nested inside a class, must still be caught"
        );
    }

    #[cfg(feature = "rust")]
    // TC-162, FR-013-AC-3 (FND-001): a struct field tree-sitter could not
    // resolve at all trips the structural check — a struct has no separate
    // executable body to distinguish its signature from, so a field is
    // exactly as structural as a top-level item.
    #[test]
    fn unresolvable_struct_field_counts_as_a_declaration_structure_error() {
        let source = "mod m {\n    struct S {\n        !!!\n    }\n}\n";
        let parsed =
            parse_file(Language::Rust, "src/mod_struct.rs", source).expect("tree still produced");
        assert!(
            parsed.diagnostic().is_some(),
            "a struct field tree-sitter could not resolve, nested inside a mod, must still be caught"
        );
    }

    #[cfg(feature = "rust")]
    // TC-165, FR-013-AC-3 (FND-001): a broken `impl` method (missing its own
    // name) trips the structural check the same way a broken top-level `fn`
    // would.
    #[test]
    fn unresolvable_impl_method_counts_as_a_declaration_structure_error() {
        let source = "impl Foo {\n    pub fn (&self) { }\n}\n";
        let parsed =
            parse_file(Language::Rust, "src/impl_broken.rs", source).expect("tree still produced");
        assert!(
            parsed.diagnostic().is_some(),
            "a method missing its own name, inside an impl, must still be caught"
        );
    }

    #[cfg(feature = "rust")]
    // TC-166, FR-013-AC-3 (FND-001): garbage two levels deep — a `mod`
    // nested inside a `mod` — trips the structural check; depth alone is not
    // what determines whether this predicate looks, only whether the
    // enclosing node is itself a declaration-list-shaped body, at whatever
    // depth it occurs.
    #[test]
    fn unresolvable_item_nested_two_mods_deep_counts_as_a_declaration_structure_error() {
        let source = "mod a {\n    mod b {\n        bad\n    }\n}\n";
        let parsed =
            parse_file(Language::Rust, "src/nested_mod.rs", source).expect("tree still produced");
        assert!(
            parsed.diagnostic().is_some(),
            "garbage nested two mods deep must still be caught"
        );
    }

    #[cfg(feature = "rust")]
    // TC-167, FR-013-AC-3 (FND-001): a `mod` missing its closing brace — the
    // grammar inserts a `MISSING "}"` as a direct child of the mod's own
    // `declaration_list` — trips the structural check.
    #[test]
    fn mod_missing_closing_brace_counts_as_a_declaration_structure_error() {
        let source = "mod m {\n    pub fn f() {}\n";
        let parsed =
            parse_file(Language::Rust, "src/mod_unclosed.rs", source).expect("tree still produced");
        assert!(
            parsed.diagnostic().is_some(),
            "a mod missing its closing brace must still be caught"
        );
    }

    #[cfg(feature = "python")]
    // TC-163, FR-013-AC-10 (FND-001): a body-local error inside a method
    // nested inside a class stays `Ok` with no diagnostic — nesting alone
    // does not trip the check, only an unresolvable declaration does.
    #[test]
    fn body_local_error_in_a_method_nested_in_a_class_stays_ok_with_no_diagnostic() {
        let source = "class Foo:\n    def broken(self):\n        return 1 +\n    def intact(self):\n        return 2\n";
        let parsed = parse_file(Language::Python, "x.py", source).expect("parses cleanly");
        assert!(
            parsed.diagnostic().is_none(),
            "a body-local error inside a class method must not trip the structural check"
        );
    }

    #[cfg(feature = "typescript")]
    // TC-164, FR-013-AC-10 (FND-001): a body-local error inside a function
    // nested inside a namespace stays `Ok` with no diagnostic.
    #[test]
    fn body_local_error_in_a_namespace_fn_stays_ok_with_no_diagnostic() {
        let source =
            "namespace NS {\n    function broken() { 1 + }\n    function intact() { }\n}\n";
        let parsed = parse_file(Language::TypeScript, "x.ts", source).expect("parses cleanly");
        assert!(
            parsed.diagnostic().is_none(),
            "a body-local error inside a namespace function must not trip the structural check"
        );
    }

    #[cfg(feature = "typescript")]
    // TC-160, FR-013-AC-3, FR-013-AC-10 (FND-001): a TypeScript function
    // tree-sitter could not resolve at all, nested inside a `namespace`,
    // trips the structural check the same way a top-level one would. (An
    // anonymous `function () {}` is not used here — TypeScript's grammar
    // accepts a nameless function as a valid function-expression statement,
    // so it produces no `ERROR`/`MISSING` node at all; a malformed parameter
    // list is what genuinely defeats the grammar.)
    #[test]
    fn unresolvable_function_in_a_namespace_counts_as_a_declaration_structure_error() {
        let source =
            "namespace NS {\n    function f(: void {\n    }\n    function intact() {}\n}\n";
        let parsed = parse_file(Language::TypeScript, "x.ts", source).expect("tree still produced");
        assert!(
            parsed.diagnostic().is_some(),
            "a function with a malformed parameter list, nested inside a namespace, must still be caught"
        );
    }

    /// Every node's kind and byte span, in traversal order — a determinism
    /// fingerprint independent of `to_sexp()`'s formatting.
    #[cfg(feature = "rust")]
    fn walk_spans(root: Node<'_>) -> Vec<(String, usize, usize)> {
        let mut cursor = root.walk();
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            out.push((node.kind().to_string(), node.start_byte(), node.end_byte()));
            let children: Vec<_> = node.children(&mut cursor).collect();
            for child in children.into_iter().rev() {
                stack.push(child);
            }
        }
        out
    }
}
