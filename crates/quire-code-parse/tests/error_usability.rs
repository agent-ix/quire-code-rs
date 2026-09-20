//! `ParseError` must be usable the way a consumer walking many files actually
//! needs it: boxed into a trait object, and collected across files into a
//! `Vec` that outlives any one file's own parse call.
//!
//! An earlier shape of this crate could not do either — `ParseError<'src>`
//! carried the borrowed `ParsedFile<'src>` tree-sitter produced despite the
//! error, so the error itself could never be `'static` (PLAT-841 PR #22
//! review finding FND-005). `quire-rs` walking a tree of files and
//! collecting every file's outcome is exactly the shape that needed this.
//!
//! `ParseError`'s `'static`-ness is tested directly in `src/error.rs`
//! (TC-169, a compiled type-level assertion) and `tests/thread_safety.rs`
//! (TC-170, boxing as `Box<dyn Error + 'static>`) — both against
//! `ParseError` itself, constructed directly, since `ParseError::NoTree` is
//! a defensive case this crate's own inputs do not reach through
//! `parse_file` (see `ParseError`'s own docs). This file originally tried to
//! back the same claim (FR-013-AC-11) with a *different* test — collecting
//! per-file outcomes into a `Vec<String>` across a loop — but that test
//! never touched `ParseError` at all: it collected `Diagnostic`s read off
//! `Ok(ParsedFile)`, which were never borrowing anything an earlier,
//! non-`'static` `ParseError` shape would have prevented either (its own
//! comment said so). It would have passed identically against the old
//! shape, so it proved nothing about `'static`-ness and has been retargeted
//! below (PLAT-841 PR #22 review round 3, finding FND-012).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use quire_code_parse::{parse_file, Language};

#[cfg(feature = "rust")]
// TC-171, FR-013-AC-3, FR-013-AC-12: a consumer walking several files reads each file's
// `Diagnostic` — itself borrowed from that file's own `ParsedFile`, per
// FND-013 now carrying `file` alongside `line` — and projects it into an
// owned `String` before the loop moves to the next file, so the borrow never
// has to outlive the iteration that produced it. This is the realistic shape
// `quire-rs` needs (collect outcomes across a batch, not sources), and it is
// `Diagnostic`'s per-file borrow discipline being exercised, not
// `ParseError`'s `'static`-ness — see the module docs for why this row no
// longer cites AC-11.
#[test]
fn diagnostics_from_several_files_project_into_one_vec_outliving_their_sources() {
    fn diagnostics_for(files: &[(&str, &str)]) -> Vec<String> {
        let mut out = Vec::new();
        for (name, source) in files {
            let parsed = parse_file(Language::Rust, name, source).expect("tree still produced");
            if let Some(diagnostic) = parsed.diagnostic() {
                // `diagnostic.file()`, not the loop's own `name` — proving the
                // diagnostic carries its own file identity (FND-013) rather
                // than the caller having to re-pair it from context.
                out.push(format!("{}:{}", diagnostic.file(), diagnostic.line()));
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
