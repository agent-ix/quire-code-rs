//! Selected-scope extraction against a typed manifest (FR-014).

use quire_code_rs::{
    content_identity, extract_selected, ExtractionScope, Language, SelectedModule,
    SelectionManifest, SourceFile, UnaccountedCause, UnaccountedEntry, UnavailableReason,
};

const A_SRC: &str = "pub fn alpha() {}\npub fn beta() {}\n";
const B_SRC: &str = "pub struct Store;\nimpl Store { pub fn put(&self) {} }\n";

fn file(path: &str, content: &str) -> SourceFile {
    SourceFile::new("acme", "widget", path, Language::Rust, content)
}

fn module(path: &str, content: &str) -> SelectedModule {
    SelectedModule::new("acme", "widget", path, content_identity(content))
}

fn manifest(modules: Vec<SelectedModule>) -> SelectionManifest {
    SelectionManifest::new("selection-1", modules)
}

fn paths_in_nodes(outcome: &quire_code_rs::ExtractionOutcome) -> Vec<String> {
    let mut v: Vec<String> = outcome
        .result
        .nodes
        .iter()
        .map(|n| n.name.clone())
        .filter(|n| !n.contains("::"))
        .collect();
    v.sort();
    v
}

fn two_module_manifest() -> SelectionManifest {
    manifest(vec![module("src/a.rs", A_SRC), module("src/b.rs", B_SRC)])
}

// TC-177 / FR-014-AC-1: Full only from the traversal, with exactly the selected records.
#[test]
fn full_selection_is_full_with_selected_records() {
    let outcome = extract_selected(
        &two_module_manifest(),
        &[file("src/b.rs", B_SRC), file("src/a.rs", A_SRC)],
    );
    assert_eq!(outcome.scope, ExtractionScope::Full);
    assert_eq!(outcome.selection, "selection-1");
    assert!(outcome.extra.is_empty());
    assert_eq!(
        paths_in_nodes(&outcome),
        vec![
            "acme/widget/src/a.rs".to_string(),
            "acme/widget/src/b.rs".to_string()
        ]
    );
}

// TC-178 / FR-014-AC-2
#[test]
fn omitted_module_is_partial_with_it_missing() {
    let outcome = extract_selected(&two_module_manifest(), &[file("src/a.rs", A_SRC)]);
    let ExtractionScope::Partial { missing } = &outcome.scope else {
        panic!("expected Partial, got {:?}", outcome.scope);
    };
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].path, "src/b.rs");
    assert_eq!(missing[0].function, None);
    assert_eq!(missing[0].cause, UnaccountedCause::NotSupplied);
    assert_eq!(
        paths_in_nodes(&outcome),
        vec!["acme/widget/src/a.rs".to_string()]
    );
}

// TC-179 / FR-014-AC-3
#[test]
fn extra_file_is_listed_and_not_included() {
    let outcome = extract_selected(
        &two_module_manifest(),
        &[
            file("src/a.rs", A_SRC),
            file("src/b.rs", B_SRC),
            file("src/c.rs", "pub fn gamma() {}\n"),
        ],
    );
    assert_eq!(outcome.scope, ExtractionScope::Full);
    assert_eq!(outcome.extra.len(), 1);
    assert_eq!(outcome.extra[0].path, "src/c.rs");
    assert!(outcome
        .result
        .nodes
        .iter()
        .all(|n| !n.name.contains("src/c.rs")));
}

// TC-180 / FR-014-AC-4
#[test]
fn identity_mismatch_is_listed_and_not_included() {
    let changed = "pub fn alpha() {}\npub fn beta() {}\npub fn sneaked_in() {}\n";
    let outcome = extract_selected(
        &two_module_manifest(),
        &[file("src/a.rs", changed), file("src/b.rs", B_SRC)],
    );
    let ExtractionScope::Partial { missing } = &outcome.scope else {
        panic!("expected Partial, got {:?}", outcome.scope);
    };
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].path, "src/a.rs");
    assert_eq!(
        missing[0].cause,
        UnaccountedCause::IdentityMismatch {
            expected: content_identity(A_SRC),
            actual: content_identity(changed),
        }
    );
    assert!(outcome
        .result
        .nodes
        .iter()
        .all(|n| !n.name.contains("sneaked_in")));
    assert!(outcome
        .result
        .nodes
        .iter()
        .all(|n| !n.name.contains("src/a.rs")));
}

// TC-181 / FR-014-AC-5
#[test]
fn unparsable_selected_file_is_partial_with_diagnostics() {
    let broken = "pub fn broken( {{{";
    let m = manifest(vec![
        module("src/a.rs", A_SRC),
        module("src/bad.rs", broken),
    ]);
    let outcome = extract_selected(&m, &[file("src/a.rs", A_SRC), file("src/bad.rs", broken)]);
    let ExtractionScope::Partial { missing } = &outcome.scope else {
        panic!("expected Partial, got {:?}", outcome.scope);
    };
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].path, "src/bad.rs");
    let UnaccountedCause::Unparsable { diagnostics } = &missing[0].cause else {
        panic!("expected Unparsable, got {:?}", missing[0].cause);
    };
    assert!(diagnostics.iter().all(|d| d.path == "src/bad.rs"));
    assert!(!diagnostics.is_empty());
    assert!(outcome
        .result
        .nodes
        .iter()
        .all(|n| !n.name.contains("src/bad.rs")));
}

// TC-182 / FR-014-AC-6
#[test]
fn degenerate_selections_are_unavailable() {
    // Names a module no supplied file provides.
    let absent = manifest(vec![module("src/a.rs", A_SRC)]);
    let outcome = extract_selected(&absent, &[file("src/other.rs", A_SRC)]);
    let ExtractionScope::Unavailable { reason } = &outcome.scope else {
        panic!("expected Unavailable, got {:?}", outcome.scope);
    };
    let UnavailableReason::NoModuleAccounted { unaccounted } = reason else {
        panic!("expected NoModuleAccounted, got {reason:?}");
    };
    assert_eq!(
        unaccounted,
        &vec![UnaccountedEntry {
            org: "acme".into(),
            repo: "widget".into(),
            path: "src/a.rs".into(),
            function: None,
            cause: UnaccountedCause::NotSupplied,
        }]
    );
    assert_eq!(outcome.extra.len(), 1);
    assert!(outcome.result.nodes.is_empty());

    let empty = extract_selected(&manifest(vec![]), &[file("src/a.rs", A_SRC)]);
    assert_eq!(
        empty.scope,
        ExtractionScope::Unavailable {
            reason: UnavailableReason::EmptySelection
        }
    );

    let dup = manifest(vec![module("src/a.rs", A_SRC), module("./src/a.rs", A_SRC)]);
    let dup_outcome = extract_selected(&dup, &[file("src/a.rs", A_SRC)]);
    assert_eq!(
        dup_outcome.scope,
        ExtractionScope::Unavailable {
            reason: UnavailableReason::DuplicateModule {
                org: "acme".into(),
                repo: "widget".into(),
                path: "src/a.rs".into(),
            }
        }
    );
}

// TC-183 / FR-014-AC-7
#[test]
fn absent_function_is_partial_and_selection_identity_is_preserved() {
    let m = manifest(vec![
        module("src/a.rs", A_SRC).with_functions(["alpha", "missing_fn"])
    ]);
    let outcome = extract_selected(&m, &[file("src/a.rs", A_SRC)]);
    assert_eq!(outcome.selection, "selection-1");
    let ExtractionScope::Partial { missing } = &outcome.scope else {
        panic!("expected Partial, got {:?}", outcome.scope);
    };
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].function.as_deref(), Some("missing_fn"));
    assert_eq!(missing[0].cause, UnaccountedCause::FunctionAbsent);

    let ok = manifest(vec![
        module("src/a.rs", A_SRC).with_functions(["alpha", "beta"])
    ]);
    assert_eq!(
        extract_selected(&ok, &[file("src/a.rs", A_SRC)]).scope,
        ExtractionScope::Full
    );
}
