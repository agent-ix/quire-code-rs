//! External consumer tests.
//!
//! This file compiles as its own crate, linking only `quire_code_parse`'s
//! public API — the same position a real consumer (`quire-rs`,
//! `filament-ide-rs`, a daemon) is in. It never names `quire-code-rs`, the
//! fact model, the type environment, the call resolver or the record
//! emitter, so a passing run of this file is itself evidence for
//! FR-013-AC-1 and FR-013-CON-2, not just an assertion of them.
//!
//! The crate's `unwrap_used`/`expect_used`/`panic` denials apply to this
//! package's every target, including this one; a test asserting on failure
//! is what those macros are for, so they are allowed here explicitly rather
//! than the crate's production-code denial being relaxed.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use quire_code_parse::{parse_file, Language, ParseError};

#[cfg(feature = "rust")]
// TC-135, FR-013-AC-1: a consumer parses Rust source and walks the tree with
// tree-sitter's own Node/TreeCursor API.
#[test]
fn consumer_parses_rust_and_walks_the_tree() {
    let source = "pub struct Store {\n    pub n: u32,\n}\n\nimpl Store {\n    pub fn n(&self) -> u32 { self.n }\n}\n";
    let parsed = parse_file(Language::Rust, "src/store.rs", source).expect("parses cleanly");

    let mut cursor = parsed.root_node().walk();
    let kinds: Vec<&str> = parsed
        .root_node()
        .children(&mut cursor)
        .map(|n| n.kind())
        .collect();
    assert_eq!(kinds, vec!["struct_item", "impl_item"]);
}

#[cfg(feature = "python")]
// TC-146: the Python grammar parses and walks the same way.
#[test]
fn consumer_parses_python_and_walks_the_tree() {
    let source = "def helper(x):\n    return x + 1\n";
    let parsed = parse_file(Language::Python, "tool/main.py", source).expect("parses cleanly");
    assert_eq!(parsed.root_node().kind(), "module");
    let function = parsed
        .root_node()
        .named_child(0)
        .expect("one top-level statement");
    assert_eq!(function.kind(), "function_definition");
}

#[cfg(feature = "typescript")]
// TC-147: the TypeScript and TSX grammars both parse and walk.
#[test]
fn consumer_parses_typescript_and_tsx_and_walks_the_tree() {
    let ts_source = "export function render(node: string): string { return node; }\n";
    let ts = parse_file(Language::TypeScript, "src/render.ts", ts_source).expect("parses cleanly");
    assert_eq!(ts.root_node().kind(), "program");

    let tsx_source = "export function App() { return <div />; }\n";
    let tsx = parse_file(Language::Tsx, "src/App.tsx", tsx_source).expect("parses cleanly");
    assert_eq!(tsx.root_node().kind(), "program");
}

#[cfg(feature = "rust")]
// TC-136, FR-013-AC-3: a syntax-error file returns a named diagnostic with
// `file:line`, never an empty or default `Ok` result.
#[test]
fn syntax_error_file_returns_diagnostic_not_empty_result() {
    let source = "pub fn broken(x: u32) -> u32 {\n    x +\n";
    let result = parse_file(Language::Rust, "src/broken.rs", source);
    match result {
        Err(ParseError::Syntax { file, line, .. }) => {
            assert_eq!(file, "src/broken.rs");
            assert!(line >= 1, "line is one-based and always present");
        }
        Err(other) => panic!("expected ParseError::Syntax, got {other:?}"),
        Ok(_) => panic!("a file with an unbalanced expression must not parse cleanly"),
    }
}

#[cfg(feature = "rust")]
// TC-151, FR-013-AC-3: a body-local syntax error — inside one declaration's
// own expression, a sibling declaration intact — returns `Ok`, not `Err`.
// This is the case `filament-ide-rs` needs: a file mid-edit almost always
// carries exactly this shape of error, and the API must still hand back a
// walkable tree, not a hard failure, or live editing gets `Err` on every
// keystroke.
#[test]
fn body_local_syntax_error_still_returns_ok_with_a_walkable_tree() {
    let source = "pub fn broken() -> u32 { 1 + }\npub fn intact() -> u32 { 2 }\n";
    let parsed =
        parse_file(Language::Rust, "src/mid_edit.rs", source).expect("body-local error is Ok");
    assert!(
        parsed.root_node().has_error(),
        "the body-local error is still visible to a caller that looks"
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
        "both declarations keep their own recognizable, walkable kind"
    );
}

#[cfg(feature = "rust")]
// TC-154, FR-013-AC-3: the tree tree-sitter produced is still reachable
// through `Err(ParseError::Syntax { parsed, .. })` — a hard diagnostic is
// never the price of losing the tree.
#[test]
fn syntax_error_still_carries_the_tree_it_names_a_diagnostic_against() {
    let source = "pub fn broken(x: u32) -> u32 {\n    x +\n";
    let result = parse_file(Language::Rust, "src/broken.rs", source);
    match result {
        Err(ParseError::Syntax { parsed, .. }) => {
            assert_eq!(parsed.root_node().kind(), "source_file");
            assert_eq!(parsed.source(), source);
        }
        other => panic!("expected ParseError::Syntax carrying a tree, got {other:?}"),
    }
}

#[cfg(feature = "python")]
// TC-148: the same hard-diagnostic rule holds for Python, not just Rust.
#[test]
fn syntax_error_file_returns_diagnostic_for_python_too() {
    let source = "def broken(\n    pass\n";
    let result = parse_file(Language::Python, "tool/broken.py", source);
    assert!(
        result.is_err(),
        "an unclosed parameter list must not parse cleanly"
    );
    let err = result.expect_err("checked above");
    assert_eq!(err.file(), "tool/broken.py");
}

#[cfg(feature = "rust")]
// TC-149, FR-013-AC-5: parsing the same bytes twice, independently, yields
// the same tree structure both times.
#[test]
fn identical_input_yields_identical_tree_structure_across_independent_parses() {
    let source = "pub struct Store { pub n: u32 }\n\
                  impl Store {\n\
                 \x20   pub fn n(&self) -> u32 { self.n }\n\
                  }\n";

    let first = parse_file(Language::Rust, "fixtures/sample.rs", source).expect("parses cleanly");
    let second = parse_file(Language::Rust, "fixtures/sample.rs", source).expect("parses cleanly");

    assert_eq!(first.root_node().to_sexp(), second.root_node().to_sexp());
}

#[cfg(feature = "rust")]
// TC-150, FR-013-AC-2: the returned value's source outlives the call and
// slices from it stay valid — proven by returning a slice from a helper and
// reading it after the helper's own locals are gone.
#[test]
fn parsed_file_source_outlives_the_call_that_produced_it() {
    fn parse_and_return<'src>(source: &'src str) -> quire_code_parse::ParsedFile<'src> {
        parse_file(Language::Rust, "src/lib.rs", source).expect("parses cleanly")
    }

    let owned = String::from("pub fn kept() -> u32 { 7 }\n");
    let parsed = parse_and_return(&owned);
    assert_eq!(parsed.source(), owned.as_str());
}

#[cfg(feature = "rust")]
// TC-155, FR-013-AC-5: the tree's rendered structure matches a golden
// s-expression committed to the repo, not just another tree produced by the
// same process in the same test run. TC-142/TC-149 only prove determinism
// within one process; a golden file is what would catch drift across a
// tree-sitter version bump or a different machine, since the committed
// fixture is graded against, not regenerated by, the test that reads it.
#[test]
fn tree_structure_matches_a_golden_fixture_committed_across_process_boundaries() {
    let source = include_str!("fixtures/determinism.rs.txt");
    let golden = include_str!("fixtures/determinism.sexp");
    let parsed =
        parse_file(Language::Rust, "fixtures/determinism.rs.txt", source).expect("parses cleanly");
    assert_eq!(parsed.root_node().to_sexp(), golden.trim_end());
}
