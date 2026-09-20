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
    diagnostic: Option<Diagnostic<'src>>,
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
    #[must_use = "a Some diagnostic means the tree's declaration structure is \
        unrecoverable at the position it names; a consumer that reports \
        symbols from this tree without surfacing this reintroduces a silent \
        zero-symbol result, one layer up (FR-013-AC-3, FR-051-AC-9)"]
    pub fn diagnostic(&self) -> Option<Diagnostic<'src>> {
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
/// # Examples
///
/// ```
/// use quire_code_parse::{parse_file, Language};
///
/// let source = "pub fn broken(x: u32) -> u32 {\n    x +\n";
/// let parsed = parse_file(Language::Rust, "src/broken.rs", source)
///     .expect("tree-sitter still produces a tree even here");
///
/// // The tree is available either way; `diagnostic()` is how a caller
/// // finds out whether its declaration structure can be trusted.
/// if let Some(diagnostic) = parsed.diagnostic() {
///     assert_eq!(diagnostic.file(), "src/broken.rs");
///     assert_eq!(diagnostic.line(), 1);
///     // A consumer's own reporting is obligated to surface this, not
///     // discard it silently — see FR-013's "Consumer obligation" section.
/// } else {
///     unreachable!("this source's declaration structure is unrecoverable");
/// }
/// ```
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
        Some(Diagnostic { file, line, column })
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
/// the tree, not only at the top level: `root` itself, or any node that is
/// not *inside* a declaration's own executable body, has a direct child that
/// is itself an `ERROR` or `MISSING` node, meaning tree-sitter could not
/// resolve even the identity of a declaration there.
///
/// Deliberately narrower than [`Node::has_error`], which is also true for an
/// ordinary, fully-identifiable declaration whose *body* — the executable
/// statements inside a function or method, not its name, parameters, return
/// type, field list or variant list — contains an error several levels down.
///
/// This asks the structural question at *every* nesting level, not only the
/// root's direct children. An earlier, depth-1-only form of this check
/// (PLAT-841 PR #22 review finding FND-001) missed every declaration nested
/// one level inside a `mod`, `impl`, `class` or `namespace` — and since every
/// Python method sits at depth 2 inside `class_definition`, that earlier form
/// structurally could not see a broken Python method at all: the exact
/// PLAT-14 shape ("a genuinely broken file reports nothing") this crate
/// exists to end, reintroduced one layer down. A second, still-too-narrow
/// form (PLAT-841 PR #22 review round 3, findings FND-007/FND-008/FND-009)
/// enumerated declaration-list *container* kinds (`declaration_list`,
/// `class_body`, and so on) rather than asking the question FR-013-AC-10
/// already states — "its own node kind, name **and signature** remain
/// resolvable" — so it missed a struct field, an enum variant and a
/// malformed parameter list, each of which sits inside a declaration's own
/// signature, not its body, and each of which needed its own kind added to
/// the enumeration to be caught (and TypeScript's `class_body` case passed
/// only because that grammar happens to land its `ERROR` there directly, an
/// empirically-fitted rather than principled agreement). [`scan_for_declaration_structure_error`]
/// implements the rule directly instead: is a given node *inside the
/// executable body* of a declaration that has one (a function, a method), or
/// is it anything else — a name, parameters, a return type, a field list, a
/// variant list, a class's or module's own list of further declarations, or
/// a fresh declaration nested inside a body? Only the first is exempt from
/// the direct-child check; everything else gets it, by default, with no
/// further enumeration needed.
fn has_declaration_structure_error(root: Node<'_>) -> bool {
    if root.is_error() || root.is_missing() {
        return true;
    }
    scan_for_declaration_structure_error(root, false)
}

/// `node.has_error()` is `false` for a subtree with no error anywhere
/// beneath it, so this short-circuits without walking such a subtree at all
/// — the same property [`first_error_position`] already relies on to search
/// to full depth cheaply.
///
/// `inside_executable_body` is `true` while the recursion is anywhere inside
/// a declaration's own executable body — not only at the body node itself,
/// but at every descendant of it, all the way down, since a body-local error
/// is not a direct child of the body node in general (an incomplete
/// expression's `MISSING` token is nested inside the expression, which is
/// nested inside the body). Propagating this flag through the whole
/// subtree — rather than exempting only the body node itself and then
/// re-deciding fresh at each descendant, which is what an earlier form of
/// this function did and which is why it flagged an ordinary body-local
/// error as structural — is what makes that distinction hold at every depth
/// inside the body, not only immediately beneath it.
///
/// The flag resets to a fresh (non-`Some`) decision at a *nested
/// declaration* ([`is_declaration_node`]) even while already inside a body,
/// since Rust, Python and TypeScript all allow a `mod`, `class`, `impl` or
/// function to be declared inside a function body, and that nested
/// declaration's own name, signature and (if it is itself executable) body
/// are each subject to the same rule again, independent of the body they
/// happen to sit inside.
fn scan_for_declaration_structure_error(node: Node<'_>, inside_executable_body: bool) -> bool {
    if !node.has_error() {
        return false;
    }
    let is_declaration = is_declaration_node(node);
    if is_declaration || !inside_executable_body {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_error() || child.is_missing() {
                return true;
            }
        }
    }
    let executable_body = is_declaration
        .then(|| executable_body_field(node))
        .flatten();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let child_inside_body = if is_declaration {
            Some(child) == executable_body
        } else {
            inside_executable_body
        };
        if scan_for_declaration_structure_error(child, child_inside_body) {
            return true;
        }
    }
    false
}

/// Whether `node` is itself a declaration this rule re-evaluates from
/// scratch: a function, method, or a `mod`/`impl`/`trait`/`struct`/`enum`/
/// `class`/`namespace` that holds further declarations. Used only to decide
/// where [`scan_for_declaration_structure_error`] resets its "inside a body"
/// state, not to decide the structural check itself — that check applies to
/// every node by default regardless of this list, matching FR-013-AC-10's
/// rule directly rather than approximating it.
fn is_declaration_node(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_item"
            | "function_signature_item"
            | "mod_item"
            | "impl_item"
            | "trait_item"
            | "struct_item"
            | "enum_item"
            | "union_item"
            | "function_definition"
            | "class_definition"
            | "function_declaration"
            | "function_expression"
            | "generator_function_declaration"
            | "generator_function"
            | "method_definition"
            | "arrow_function"
            | "class_declaration"
            | "class"
            | "internal_module"
    )
}

/// `node`'s `body` field, when `node` is a declaration kind whose body
/// *executes* — a function's or method's own statements — as opposed to a
/// `mod`/`impl`/`trait`/`struct`/`enum`/`class`/`namespace`'s own list of
/// further declarations. `None` for the latter, so their own `body` gets the
/// structural check like everything else rather than being treated as an
/// opaque zone.
///
/// All three grammars use the *same* node kind, and the *same* field name
/// (`body`), for both a declaration's executable body and (Python,
/// TypeScript) a `class`'s or `namespace`'s own list of further
/// declarations — verified empirically via `to_sexp()` on nested fixtures,
/// not assumed — so the child's kind alone cannot tell them apart; only
/// `node`'s own kind can, hence the explicit list here rather than a check
/// on the field value's kind. (TypeScript's `declare module "..."`
/// ambient-module spelling was not separately verified; see FR-013's "Known
/// limitations" for why this rule is nonetheless expected to cover it, and
/// FND-010 for a shape it does not.)
fn executable_body_field(node: Node<'_>) -> Option<Node<'_>> {
    if !matches!(
        node.kind(),
        "function_item"
            | "function_definition"
            | "function_declaration"
            | "function_expression"
            | "generator_function_declaration"
            | "generator_function"
            | "method_definition"
            | "arrow_function"
    ) {
        return None;
    }
    node.child_by_field_name("body")
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
    // TC-162, FR-013-AC-3 (FND-001, FND-007): a struct field tree-sitter
    // could not resolve — a name given but no type — trips the structural
    // check; a struct has no separate executable body to distinguish its
    // signature from, so a field is exactly as structural as a top-level
    // item. `struct S { a: , }` (a `MISSING type_identifier` inside the
    // `field_declaration` for `a`) is the fixture this row's own text
    // claims; the original fixture (bare `!!!` garbage) landed as a direct
    // `ERROR` child regardless of which check ran, so it never actually
    // exercised the field-declaration case the row describes.
    #[test]
    fn unresolvable_struct_field_counts_as_a_declaration_structure_error() {
        let source = "mod m {\n    struct S {\n        a: ,\n    }\n}\n";
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
    // does not trip the check, only an unresolvable declaration does. An
    // assignment with a dangling operator (`x = 1 +`) is used rather than a
    // dangling `return`: Python's grammar recovers a broken `return`
    // statement by attaching the resulting `ERROR` as an extra sibling of
    // `function_definition` itself (outside its `body` field), which the
    // body-node rule correctly reads as structural — that shape is
    // documented separately (see the `return`-specific note below this
    // test) rather than folded into this one, which exercises the ordinary
    // case: an error genuinely nested inside the body node.
    #[test]
    fn body_local_error_in_a_method_nested_in_a_class_stays_ok_with_no_diagnostic() {
        let source = "class Foo:\n    def broken(self):\n        x = 1 +\n        return x\n    def intact(self):\n        return 2\n";
        let parsed = parse_file(Language::Python, "x.py", source).expect("parses cleanly");
        assert!(
            parsed.diagnostic().is_none(),
            "a body-local error inside a class method must not trip the structural check"
        );
    }

    #[cfg(feature = "python")]
    // TC-174, FR-013-AC-3: a dangling `return` (as opposed to a dangling
    // assignment, TC-163) is a documented case where the body-node rule and
    // intuition disagree. Python's grammar recovers `return 1 +` by
    // attaching the resulting `ERROR` as an extra child of
    // `function_definition` itself — a sibling of `body`, not inside it —
    // even though the function's own kind, name and parameters are fully
    // resolvable. The body-node rule reads this as structural, mechanically
    // and consistently (it is, literally, not inside the body node); this is
    // a documented known limitation, not a bug — FR-013's "Known
    // limitations" section names it, rather than this test asserting the
    // intuitive answer silently.
    #[test]
    fn dangling_return_recovers_outside_the_body_node_and_reads_as_structural() {
        let source = "class Foo:\n    def broken(self):\n        return 1 +\n    def intact(self):\n        return 2\n";
        let parsed = parse_file(Language::Python, "x.py", source).expect("tree still produced");
        assert!(
            parsed.diagnostic().is_some(),
            "documents that this specific recovery shape reads as structural, not a claim that it should"
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

    #[cfg(feature = "rust")]
    // TC-172, FR-013-AC-3 (FND-008): a malformed parameter list — the `ERROR`/
    // `MISSING` sits inside `parameters`, the declaration's own signature,
    // not its executable body — trips the structural check even though the
    // function's own `body` block is syntactically intact. The body-node
    // rule catches this with no enumeration of `parameters` as a special
    // case: `parameters` was never a declaration's executable body, so its
    // direct children get the structural check by default.
    #[test]
    fn malformed_parameter_list_counts_as_a_declaration_structure_error() {
        let source = "pub fn f( -> u32 {}\n";
        let parsed =
            parse_file(Language::Rust, "src/bad_params.rs", source).expect("tree still produced");
        assert!(
            parsed.diagnostic().is_some(),
            "a malformed parameter list must be caught even though the function body is intact"
        );
    }

    #[cfg(feature = "rust")]
    // TC-173, FR-013-AC-3 (FND-009): an enum variant tree-sitter could not
    // resolve — a discriminant value missing after `=` — trips the
    // structural check; an `enum_variant` has no executable body either, so
    // it needs no special case, the same as `field_declaration` above.
    #[test]
    fn unresolvable_enum_variant_counts_as_a_declaration_structure_error() {
        let source = "pub enum E {\n    A = ,\n    B,\n}\n";
        let parsed =
            parse_file(Language::Rust, "src/bad_enum.rs", source).expect("tree still produced");
        assert!(
            parsed.diagnostic().is_some(),
            "an enum variant tree-sitter could not resolve must be caught"
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
