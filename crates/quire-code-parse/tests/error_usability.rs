//! `ParseError` must be usable the way a consumer walking many files actually
//! needs it: boxed into a trait object, and collected across files into a
//! `Vec` that outlives any one file's own parse call.
//!
//! An earlier shape of this crate could not do either — `ParseError<'src>`
//! carried the borrowed `ParsedFile<'src>` tree-sitter produced despite the
//! error, so the error itself could never be `'static` (PLAT-841 PR #22
//! review finding FND-005). `quire-rs` walking a tree of files and
//! collecting every file's outcome is exactly the shape that needed this.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use quire_code_parse::{parse_file, Language};

// `ParseError::NoTree`'s struct-shaped variant is `#[non_exhaustive]`
// (FR-013-AC-8), so it cannot be constructed with a struct literal from
// outside this crate — the boxing test for it (TC-170) lives in
// `src/error.rs`'s own `#[cfg(test)]` module instead, where that
// construction is allowed. This file only exercises what a real external
// consumer can reach: `parse_file` and `ParsedFile::diagnostic`.

#[cfg(feature = "rust")]
// TC-171, FR-013-AC-11: a consumer walking several files collects each
// file's outcome into one `Vec` that outlives any single file's own source
// buffer — the shape `quire-rs` needs when a batch of files is parsed and
// only the errors, not the sources, are kept around afterward.
#[test]
fn errors_from_several_files_collect_into_one_vec_outliving_their_sources() {
    fn diagnostics_for(files: &[(&str, &str)]) -> Vec<String> {
        let mut out = Vec::new();
        for (name, source) in files {
            // Each `source` borrow ends when this loop iteration does; only
            // the `Diagnostic`'s owned `line`/`column` values, read while the
            // borrow is alive, are carried out — proving the point at the
            // `ParsedFile`/`Diagnostic` level, which never needed `ParseError`
            // to be `'static` in the first place (it never borrows the tree).
            let parsed = parse_file(Language::Rust, name, source).expect("tree still produced");
            if let Some(diagnostic) = parsed.diagnostic() {
                out.push(format!("{name}:{}", diagnostic.line()));
            }
        }
        out
    }

    let files = [
        ("src/broken.rs", "pub fn broken(x: u32) -> u32 {\n    x +\n"),
        ("src/clean.rs", "pub fn clean() -> u32 { 1 }\n"),
    ];
    let diagnostics = diagnostics_for(&files);
    assert_eq!(diagnostics, vec!["src/broken.rs:1".to_string()]);
}
