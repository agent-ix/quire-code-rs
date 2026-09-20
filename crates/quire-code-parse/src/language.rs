//! Language selection.
//!
//! Implements the language half of
//! [FR-013](../../../spec/functional/FR-013-borrowed-parse-tree-api.md).
//! Each variant is reachable only through its own Cargo feature
//! (FR-013-CON-3): a crate compiled with a subset of `rust`/`python`/
//! `typescript` has that subset of variants and no others, so
//! `--no-default-features --features rust` cannot construct, match on, or
//! link the grammar for a language it did not ask for.

/// A source language this crate can parse.
///
/// `#[non_exhaustive]`: a language is an open set this crate does not own the
/// full membership of. Adding a fourth language is additive for every
/// existing consumer's `match`, unlike a closed decision domain this crate
/// *does* own (there is none here — contrast the consumer-owned
/// classification enums this crate deliberately does not define; see the
/// crate-level docs).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    #[cfg(feature = "rust")]
    Rust,
    #[cfg(feature = "python")]
    Python,
    #[cfg(feature = "typescript")]
    TypeScript,
    #[cfg(feature = "typescript")]
    Tsx,
}

impl Language {
    /// The tree-sitter grammar for this language.
    ///
    /// Exhaustive without a wildcard arm: every variant that can exist is
    /// gated by the same feature as its grammar dependency, so there is no
    /// reachable variant this match does not cover.
    pub(crate) fn grammar(self) -> tree_sitter::Language {
        match self {
            #[cfg(feature = "rust")]
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            #[cfg(feature = "python")]
            Language::Python => tree_sitter_python::LANGUAGE.into(),
            #[cfg(feature = "typescript")]
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            #[cfg(feature = "typescript")]
            Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TC-137, FR-013-AC-4: every compiled-in language loads its grammar.
    #[test]
    fn every_compiled_language_loads_its_grammar() {
        let languages: &[Language] = &[
            #[cfg(feature = "rust")]
            Language::Rust,
            #[cfg(feature = "python")]
            Language::Python,
            #[cfg(feature = "typescript")]
            Language::TypeScript,
            #[cfg(feature = "typescript")]
            Language::Tsx,
        ];
        for language in languages {
            let mut parser = tree_sitter::Parser::new();
            let loaded = parser.set_language(&language.grammar());
            assert!(loaded.is_ok(), "grammar failed to load for {language:?}");
        }
    }

    // TC-138, FR-013-CON-3: with only the `rust` feature compiled, `Language`
    // has exactly the one variant that feature enables — checked by
    // compiling only under that exact feature set, not by inspection.
    #[cfg(all(feature = "rust", not(feature = "python"), not(feature = "typescript")))]
    #[test]
    fn rust_only_build_has_one_language_variant() {
        let only = Language::Rust;
        // If `Python`, `TypeScript` or `Tsx` were reachable under this
        // feature set, this match would fail to compile as non-exhaustive —
        // the assertion is the compiler accepting the match, not the runtime
        // value.
        match only {
            Language::Rust => {}
        }
    }
}
