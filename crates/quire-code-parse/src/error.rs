//! The one error type this crate returns, and the diagnostic a caller finds
//! attached to `Ok(ParsedFile)` when the tree's declaration structure could
//! not be trusted.
//!
//! Implements [FR-013](../../../spec/functional/FR-013-borrowed-parse-tree-api.md)'s
//! hard-diagnostic rule: a declaration-structure failure is a named position
//! (file, line, column) that a caller cannot get past without seeing, never a
//! result indistinguishable from a clean parse. PLAT-14 is the reason this is
//! structural rather than a convention — a hand-rolled scanner desynced on
//! Python scope and returned zero symbols for a whole repository with nothing
//! in its return type saying so.
//!
//! `ParseError` itself is reserved for the case where no tree exists at all —
//! [`NoTree`](ParseError::NoTree) — and owns its `file` rather than borrowing
//! it, so it is `'static`-constructible: a consumer can `?` it into
//! `anyhow::Result`, box it as `Box<dyn Error + 'static>`, or collect it into
//! a `Vec` that outlives the source buffer a single parse borrowed. Tying
//! `ParseError` to `ParsedFile`'s own `'src` (an earlier shape of this crate,
//! carrying the tree inside `Err`) could not satisfy that: a `ParsedFile<'src>`
//! cannot itself be `'static` (rejected by `E0597` in any but a `'static`
//! source), so an error type that had to *carry* one could not be either. The
//! tree-sitter grammar being requested is a rare, essentially process-startup
//! condition (a missing or ABI-incompatible language, or a parse that
//! tree-sitter's own API contract lets return no tree at all); the ordinary
//! per-file failure this crate exists to make loud is a declaration-structure
//! error, which is common — mid-edit files hit it on every keystroke a
//! consumer like `filament-ide-rs` cares about — so it is `parse_file`'s
//! `Ok` path, attached to the tree it is about, not the rare path.

/// The one error `parse_file` returns: no syntax tree could be produced at
/// all. Reserved for this rare, essentially defensive case rather than for
/// every declaration-structure failure — see the module docs for why: this
/// type owns its `file` and is `'static`, which a type carrying a borrowed
/// [`ParsedFile`](crate::ParsedFile) could not be.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// tree-sitter accepted the requested language but produced no tree at
    /// all for this input, or the language itself could not be linked to the
    /// parser (an ABI mismatch this crate's own pinned grammar crates should
    /// never actually produce, but `Parser::set_language` returns a
    /// `Result`, so this crate has no path back to `unwrap`/`expect`/a panic
    /// if that ever stops being true).
    ///
    /// No tree exists in this case, so none is carried — contrast
    /// [`Diagnostic`], which always accompanies a real tree.
    #[error("{file}:{line}: parser produced no syntax tree")]
    #[non_exhaustive]
    NoTree { file: String, line: u32 },
}

impl ParseError {
    /// The file this diagnostic names.
    pub fn file(&self) -> &str {
        match self {
            ParseError::NoTree { file, .. } => file,
        }
    }

    /// The one-based line this diagnostic names. Never a sentinel meaning "no
    /// line was recorded" — every variant supplies a real one.
    pub fn line(&self) -> u32 {
        match self {
            ParseError::NoTree { line, .. } => *line,
        }
    }
}

/// A declaration-structure diagnostic attached to a [`ParsedFile`](crate::ParsedFile)
/// that [`ParsedFile::diagnostic`](crate::ParsedFile::diagnostic) hands back —
/// never itself the return type, because withholding the tree is never the
/// price of reporting this (FR-013-AC-3, FR-013-AC-9).
///
/// Carries `file`, not only `line`/`column`: the original hard requirement
/// this crate implements is a diagnostic naming *file and line*, and a
/// `Diagnostic` a caller has copied out of its `ParsedFile` — to log it, to
/// collect it alongside diagnostics from other files — should not have to be
/// re-paired with the file identifier by hand, or worse, paired with the
/// wrong one after being moved out of the loop that produced it (PLAT-841 PR
/// #22 review round 3, finding FND-013). `file: &'src str` costs nothing to
/// add: `Diagnostic` is only ever handed out from a borrowed `ParsedFile`,
/// which already carries the same `'src` borrow, so this adds no new
/// lifetime the type did not already sit behind. It does mean `Diagnostic`
/// is not `'static` — unlike `ParseError`, which is, precisely because it
/// never carries a `Diagnostic` or anything else borrowed (see the module
/// docs); the two types have different constraints for a reason, not by
/// oversight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic<'src> {
    /// The file this diagnostic names, borrowed from the same source the
    /// [`ParsedFile`](crate::ParsedFile) it was read from borrows.
    pub file: &'src str,
    /// One-based line of the first declaration-structure error.
    pub line: u32,
    /// Zero-based column of the first declaration-structure error.
    pub column: u32,
}

impl<'src> Diagnostic<'src> {
    /// The file this diagnostic names.
    pub fn file(&self) -> &'src str {
        self.file
    }

    /// One-based line of the first declaration-structure error.
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Zero-based column of the first declaration-structure error.
    pub fn column(&self) -> u32 {
        self.column
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_file, Language};

    // TC-139, FR-013-AC-11: `ParseError`'s accessors return the file and a
    // one-based line, never an absent value.
    #[test]
    fn every_variant_carries_file_and_line() {
        let no_tree = ParseError::NoTree {
            file: "src/lib.rs".to_string(),
            line: 1,
        };
        assert_eq!(no_tree.file(), "src/lib.rs");
        assert_eq!(no_tree.line(), 1);
    }

    // TC-140, FR-013-AC-11: the diagnostic renders the file and line in its
    // message, so it is legible without calling the accessors.
    #[test]
    fn no_tree_error_message_names_file_and_line() {
        let err = ParseError::NoTree {
            file: "src/store.rs".to_string(),
            line: 7,
        };
        assert_eq!(
            err.to_string(),
            "src/store.rs:7: parser produced no syntax tree"
        );
    }

    // TC-170, FR-013-AC-11: a `ParseError` boxes as a trait object with no
    // lifetime tied to any source it might have been produced alongside —
    // this compiles only because `ParseError` owns its `file` and is
    // `'static` (PLAT-841 PR #22 review finding FND-005). Constructed
    // directly here (allowed only inside this crate, since the variant is
    // `#[non_exhaustive]`) rather than produced through `parse_file`, since
    // `NoTree` is a defensive case this crate's own inputs do not reach.
    #[test]
    fn parse_error_boxes_as_a_static_trait_object() {
        let err = ParseError::NoTree {
            file: "src/lib.rs".to_string(),
            line: 1,
        };
        let boxed: Box<dyn std::error::Error + 'static> = Box::new(err);
        assert!(boxed.to_string().contains("src/lib.rs"));
    }

    // TC-161, FR-013-AC-9, FR-013-AC-12 (FND-013): a declaration-structure
    // error attaches a `Diagnostic` to `Ok(ParsedFile)`, carrying the file
    // identifier alongside the line — the tree and the diagnostic are never
    // in tension, since neither costs the caller the other, and the
    // diagnostic never costs the caller having to re-pair it with the file
    // by hand either.
    #[cfg(feature = "rust")]
    #[test]
    fn syntax_error_file_returns_ok_with_a_diagnostic_naming_the_line() {
        let source = "pub fn broken(x: u32) -> u32 {\n    x +\n";
        let parsed =
            parse_file(Language::Rust, "src/broken.rs", source).expect("tree still produced");
        let diagnostic = parsed
            .diagnostic()
            .expect("declaration structure is unrecoverable");
        assert_eq!(diagnostic.file(), "src/broken.rs");
        assert!(
            diagnostic.line() >= 1,
            "line is one-based and always present"
        );
    }
}
