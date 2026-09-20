//! The one error type this crate returns.
//!
//! Implements [FR-013](../../../spec/functional/FR-013-borrowed-parse-tree-api.md)'s
//! hard-diagnostic rule: a parse failure is a named, `Err`-returned
//! diagnostic naming a file and a line, never an `Ok` value the caller has to
//! inspect for silence. PLAT-14 is the reason this is structural rather than
//! a convention — a hand-rolled scanner desynced on Python scope and returned
//! zero symbols for a whole repository with nothing in its return type saying
//! so.
//!
//! The tree is never the price of that diagnostic. `ParseError::Syntax`
//! carries the [`ParsedFile`] tree-sitter still produced, so a caller that
//! receives `Err` is never locked out of the tree — only forced to
//! acknowledge, via `Result`, that its declaration structure could not be
//! fully trusted.

use crate::parse::ParsedFile;

/// Every variant carries `file` and a one-based `line` (FR-013-AC-3): the two
/// accessors below cannot return "no line" for any *reachable* variant,
/// because every arm supplies one. A variant with no natural line position —
/// there are none of those in this set — would still have to invent one
/// rather than omit it, since `line` has no `Option` to opt out into.
///
/// `'src` matches [`ParsedFile`]'s own lifetime: this error type borrows
/// exactly as much as the value it sometimes carries, never more, and never
/// falls back to an owned copy to avoid the lifetime parameter.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError<'src> {
    /// The tree's declaration structure is unrecoverable: the root, or one of
    /// its top-level children, is itself an `ERROR` or `MISSING` node — see
    /// [`parse_file`](crate::parse_file) for the full definition and why it
    /// is narrower than "the tree contains an error anywhere".
    ///
    /// The tree tree-sitter still produced is carried in `parsed`, not
    /// discarded: a declaration-structure error is a loud diagnostic, never a
    /// reason to withhold the tree a caller already paid to produce.
    #[error("{file}:{line}: syntax error")]
    #[non_exhaustive]
    Syntax {
        file: &'src str,
        /// One-based line of the first error or missing node.
        line: u32,
        /// Zero-based column of the first error or missing node.
        column: u32,
        /// The tree tree-sitter produced despite the error, fully walkable.
        parsed: ParsedFile<'src>,
    },

    /// tree-sitter accepted the requested language but produced no tree at
    /// all for this input.
    ///
    /// Defensive rather than exercised in normal use: `parse_file` sets
    /// neither a timeout nor a cancellation flag, and tree-sitter otherwise
    /// always returns a tree — even a malformed one, via `Syntax` above. This
    /// variant exists so that `parse_file` has no path back to `unwrap`,
    /// `expect` or a panic if that ever stops being true. No tree exists in
    /// this case, so none is carried.
    #[error("{file}:{line}: parser produced no syntax tree")]
    #[non_exhaustive]
    NoTree { file: &'src str, line: u32 },
}

impl<'src> ParseError<'src> {
    /// The file this diagnostic names.
    pub fn file(&self) -> &str {
        match self {
            ParseError::Syntax { file, .. } | ParseError::NoTree { file, .. } => file,
        }
    }

    /// The one-based line this diagnostic names. Never a sentinel meaning "no
    /// line was recorded" — every variant supplies a real one.
    pub fn line(&self) -> u32 {
        match self {
            ParseError::Syntax { line, .. } | ParseError::NoTree { line, .. } => *line,
        }
    }

    /// The tree tree-sitter produced despite the error, when one exists.
    ///
    /// `Some` for [`Syntax`](ParseError::Syntax) — the whole point of that
    /// variant carrying it — and `None` for [`NoTree`](ParseError::NoTree),
    /// where no tree was ever produced to carry.
    pub fn parsed(&self) -> Option<&ParsedFile<'src>> {
        match self {
            ParseError::Syntax { parsed, .. } => Some(parsed),
            ParseError::NoTree { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_file, Language};

    // TC-139, FR-013-AC-3: every variant's accessors return the file and a
    // one-based line, never an absent value.
    #[cfg(feature = "rust")]
    #[test]
    fn every_variant_carries_file_and_line() {
        let source = "pub fn broken(x: u32) -> u32 {\n    x +\n";
        let err = parse_file(Language::Rust, "src/lib.rs", source)
            .expect_err("truncated declaration is unrecoverable");
        assert_eq!(err.file(), "src/lib.rs");
        assert!(err.line() >= 1);
        assert!(
            err.parsed().is_some(),
            "Syntax carries the tree it names a diagnostic against"
        );

        let no_tree = ParseError::NoTree {
            file: "src/lib.rs",
            line: 1,
        };
        assert_eq!(no_tree.file(), "src/lib.rs");
        assert_eq!(no_tree.line(), 1);
        assert!(no_tree.parsed().is_none(), "no tree was ever produced");
    }

    // TC-140, FR-013-AC-3: the diagnostic renders the file and line in its
    // message, so it is legible without calling the accessors.
    #[cfg(feature = "rust")]
    #[test]
    fn syntax_error_message_names_file_and_line() {
        let source = "pub fn broken(x: u32) -> u32 {\n    x +\n";
        let err = parse_file(Language::Rust, "src/store.rs", source)
            .expect_err("truncated declaration is unrecoverable");
        assert_eq!(
            err.to_string(),
            format!("src/store.rs:{}: syntax error", err.line())
        );
    }
}
