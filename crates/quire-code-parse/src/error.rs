//! The one error type this crate returns.
//!
//! Implements [FR-013](../../../spec/functional/FR-013-borrowed-parse-tree-api.md)'s
//! hard-diagnostic rule: a parse failure is a named, `Err`-returned
//! diagnostic naming a file and a line, never an `Ok` value the caller has to
//! inspect for silence. PLAT-14 is the reason this is structural rather than
//! a convention — a hand-rolled scanner desynced on Python scope and returned
//! zero symbols for a whole repository with nothing in its return type saying
//! so.

/// Every variant carries `file` and a one-based `line` (FR-013-AC-3): the two
/// accessors below cannot return "no line" for any *reachable* variant,
/// because every arm supplies one. A variant with no natural line position —
/// there are none of those in this set — would still have to invent one
/// rather than omit it, since `line` has no `Option` to opt out into.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The produced syntax tree contains an error or a missing node.
    ///
    /// This is the common case: malformed or mid-edit source that tree-sitter
    /// still produced *a* tree for, but not a trustworthy one.
    #[error("{file}:{line}: syntax error")]
    Syntax {
        file: String,
        /// One-based line of the first error or missing node.
        line: u32,
        /// Zero-based column of the first error or missing node.
        column: u32,
    },

    /// tree-sitter accepted the requested language but produced no tree at
    /// all for this input.
    ///
    /// Defensive rather than exercised in normal use: `parse_file` sets
    /// neither a timeout nor a cancellation flag, and tree-sitter otherwise
    /// always returns a tree — even a malformed one, via `Syntax` above. This
    /// variant exists so that `parse_file` has no path back to `unwrap`,
    /// `expect` or a panic if that ever stops being true.
    #[error("{file}:{line}: parser produced no syntax tree")]
    NoTree { file: String, line: u32 },
}

impl ParseError {
    /// The file this diagnostic names.
    pub fn file(&self) -> &str {
        match self {
            ParseError::Syntax { file, .. } | ParseError::NoTree { file, .. } => file,
        }
    }

    /// The one-based line this diagnostic names. `1` for a variant with no
    /// single error position — the whole file is the location — never a
    /// sentinel meaning "no line was recorded".
    pub fn line(&self) -> u32 {
        match self {
            ParseError::Syntax { line, .. } | ParseError::NoTree { line, .. } => *line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TC-139, FR-013-AC-3: every variant's accessors return the file and a
    // one-based line, never an absent value.
    #[test]
    fn every_variant_carries_file_and_line() {
        let syntax = ParseError::Syntax {
            file: "src/lib.rs".to_string(),
            line: 12,
            column: 4,
        };
        assert_eq!(syntax.file(), "src/lib.rs");
        assert_eq!(syntax.line(), 12);

        let no_tree = ParseError::NoTree {
            file: "src/lib.rs".to_string(),
            line: 1,
        };
        assert_eq!(no_tree.file(), "src/lib.rs");
        assert_eq!(no_tree.line(), 1);
    }

    // TC-140, FR-013-AC-3: the diagnostic renders the file and line in its
    // message, so it is legible without calling the accessors.
    #[test]
    fn syntax_error_message_names_file_and_line() {
        let err = ParseError::Syntax {
            file: "src/store.rs".to_string(),
            line: 7,
            column: 2,
        };
        assert_eq!(err.to_string(), "src/store.rs:7: syntax error");
    }
}
