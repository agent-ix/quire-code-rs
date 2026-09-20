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
    file: String,
    language: Language,
}

impl<'src> ParsedFile<'src> {
    /// The parsed syntax tree.
    ///
    /// `tree-sitter` itself is re-exported by this crate (see the crate-level
    /// docs) rather than wrapped, so [`Tree`], [`Node`] and `TreeCursor` are
    /// tree-sitter's own types — this crate does not reimplement traversal.
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

    /// The file identifier this parse was attributed to.
    pub fn file(&self) -> &str {
        &self.file
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
/// Returns [`ParseError::Syntax`] naming `file` and the one-based line of the
/// first error or missing node when the produced tree contains one, rather
/// than an `Ok` value the caller has to inspect for silence (FR-013-AC-3).
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
    file: &str,
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

    let root = tree.root_node();
    if root.has_error() {
        let (line, column) = first_error_position(root).unwrap_or((1, 0));
        return Err(ParseError::Syntax {
            file: file.to_string(),
            line,
            column,
        });
    }

    Ok(ParsedFile {
        tree,
        source,
        file: file.to_string(),
        language,
    })
}

/// One-based line and zero-based column of the first error or missing node in
/// `root`'s subtree, if any. Independent of quire-code-rs's own equivalent
/// (this crate depends on nothing there — see FR-013-CON-2): the two exist in
/// separate crates by design, not by omission.
fn first_error_position(root: Node<'_>) -> Option<(u32, u32)> {
    let mut cursor = root.walk();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            let pos = node.start_position();
            // `saturating_add`, not `+`: this crate promises never to panic
            // on any input (FR-013-AC-6), and `row` is tree-sitter's `usize`
            // — a `+ 1` that overflowed `u32` in debug would be exactly the
            // panic that promise forbids, for input no more adversarial than
            // an unreasonably large file.
            return Some(((pos.row as u32).saturating_add(1), pos.column as u32));
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

#[cfg(test)]
mod tests {
    use super::*;

    // FR-013-AC-1 and FR-013-AC-3 are covered end to end, as an external
    // consumer would exercise them, by `tests/integration.rs` (TC-135,
    // TC-136) — that file links only this crate's public API, which is the
    // more meaningful proof of the boundary than a same-crate unit test.
    // These unit tests cover what an external test cannot see directly: the
    // borrow's pointer identity and the tree's internal determinism.

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
    // byte-identical tree on every call.
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
