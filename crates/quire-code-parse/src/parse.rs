//! The one parse entry point this crate has.
//!
//! Implements [FR-013](../../../spec/functional/FR-013-borrowed-parse-tree-api.md).

use tree_sitter::{Node, Parser, Tree};

use crate::error::ParseError;
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
}

impl<'src> ParsedFile<'src> {
    /// The parsed syntax tree.
    ///
    /// `tree-sitter` itself is re-exported by this crate (see the crate-level
    /// docs) rather than wrapped, so [`Tree`], [`Node`] and `TreeCursor` are
    /// tree-sitter's own types — this crate does not reimplement traversal.
    ///
    /// The tree is available here whether [`parse_file`] returned `Ok` or
    /// [`Err(ParseError::Syntax)`](ParseError::Syntax) — a declaration-
    /// structure error is a loud diagnostic, never a reason this crate
    /// withholds the tree it already produced (FR-013-AC-3).
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
}

impl std::fmt::Debug for ParsedFile<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParsedFile")
            .field("file", &self.file)
            .field("language", &self.language)
            .field("source_len", &self.source.len())
            .field("root_kind", &self.tree.root_node().kind())
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
/// Returns [`ParseError::Syntax`] — naming `file` and the one-based line of
/// the first error or missing node, and carrying the produced tree — when the
/// tree's *declaration structure* is unrecoverable: the root, or one of its
/// top-level children, is itself an `ERROR` or `MISSING` node, so tree-sitter
/// could not resolve even the identity of a top-level item there
/// (FR-013-AC-3). This is deliberately narrower than "the tree contains an
/// error anywhere": an error nested inside a declaration's own body — a
/// function whose expression is incomplete, mid-edit — leaves that
/// declaration's own node kind, name and signature fully readable, and
/// tree-sitter recovers it locally (verified empirically across all three
/// grammars this crate loads). That case returns `Ok`, with the tree fully
/// walkable and the error still visible to a caller that inspects
/// [`Node::has_error`] itself — this crate states only the loud, structural
/// case as its own diagnostic; a body-local error is not the failure PLAT-14
/// named, and treating it as one would fail the exact use case
/// (`filament-ide-rs` editing content mid-keystroke) this API exists to
/// serve.
///
/// Returns [`ParseError::NoTree`] in the defensive case where tree-sitter
/// accepts the language but produces no tree at all.
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
) -> Result<ParsedFile<'src>, ParseError<'src>> {
    let mut parser = Parser::new();
    parser
        .set_language(&language.grammar())
        .map_err(|_| ParseError::NoTree { file, line: 1 })?;

    let tree = parser
        .parse(source, None)
        .ok_or(ParseError::NoTree { file, line: 1 })?;

    let parsed = ParsedFile {
        tree,
        source,
        file,
        language,
    };

    if has_declaration_structure_error(parsed.root_node()) {
        let (line, column) = first_error_position(parsed.root_node()).unwrap_or((1, 0));
        return Err(ParseError::Syntax {
            file,
            line,
            column,
            parsed,
        });
    }

    Ok(parsed)
}

/// Whether `root`'s own declaration structure is unrecoverable: `root`
/// itself, or one of its direct (top-level) children, is an `ERROR` or
/// `MISSING` node — meaning tree-sitter could not resolve even the identity
/// of a top-level item there.
///
/// Deliberately narrower than [`Node::has_error`], which is also true for an
/// ordinary, fully-identifiable declaration whose *body* contains an error
/// several levels down. Empirically, across Rust, Python and TypeScript, a
/// body-local error leaves the enclosing declaration's own node kind (e.g.
/// `function_item`) intact with `is_error() == false`, even though
/// `has_error()` is `true`; only a top-level item tree-sitter could not
/// resolve *as a declaration at all* becomes a top-level `ERROR` node itself.
/// Checking only top-level child kinds, rather than walking for any error
/// anywhere, is what keeps this a purely structural check — it never asks
/// what kind of declaration a node is, only what kind of node it is.
fn has_declaration_structure_error(root: Node<'_>) -> bool {
    if root.is_error() || root.is_missing() {
        return true;
    }
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.is_error() || child.is_missing() {
            return true;
        }
    }
    false
}

/// One-based line and zero-based column of the first error or missing node in
/// `root`'s subtree, if any — searched to full depth so the reported position
/// is the most precise one available, even though
/// [`has_declaration_structure_error`] only inspects the top level to decide
/// *whether* to report at all. Independent of quire-code-rs's own equivalent
/// (this crate depends on nothing there — see FR-013-CON-2): the two exist in
/// separate crates by design, not by omission.
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
    // TC-152, FR-013-AC-3: a body-local error — the declaration's own kind,
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
        let parsed_err = match parse_file(Language::Rust, "src/broken.rs", source) {
            Err(ParseError::Syntax { parsed, .. }) => parsed,
            other => panic!("expected ParseError::Syntax, got {other:?}"),
        };
        let mut cursor = parsed_err.root_node().walk();
        let kinds: Vec<&str> = parsed_err
            .root_node()
            .children(&mut cursor)
            .map(|n| n.kind())
            .collect();
        assert_eq!(kinds, vec!["ERROR"]);
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
