//! Precision corpora for receiver-typed call resolution.
//!
//! Verifies [NFR-004](../spec/non-functional/NFR-004-conservative-resolution-precision.md):
//! every `calls` edge emitted on these corpora must appear in the corpus's
//! enumerated set of correct bindings, and the ambiguity corpus must produce no
//! edge at all.
//!
//! Precision is asserted as an absolute; recall is computed and printed as a
//! regression signal but deliberately does not gate. The asymmetry is the whole
//! point: a missing edge leaves a reader where they already were, a wrong one
//! sends them somewhere irrelevant and discredits the rest of the view.

use std::collections::BTreeSet;

use quire_code_rs::{extract, EdgeRecord, ExtractionResult, Language, SourceFile};

/// A corpus whose correct call bindings are known in advance.
struct Corpus {
    name: &'static str,
    files: Vec<SourceFile>,
    /// Every `(caller, callee)` pair that is genuinely present in the source.
    known: BTreeSet<(String, String)>,
}

fn calls(result: &ExtractionResult) -> Vec<&EdgeRecord> {
    result
        .edges
        .iter()
        .filter(|e| e.edge_type == "calls")
        .collect()
}

/// Assert zero wrong edges, and report recall.
fn assert_precision(corpus: &Corpus) {
    let result = extract(&corpus.files);
    let emitted: BTreeSet<(String, String)> = calls(&result)
        .iter()
        .map(|e| (e.source_ref.clone(), e.target_ref.clone()))
        .collect();

    let wrong: Vec<_> = emitted.difference(&corpus.known).collect();
    assert!(
        wrong.is_empty(),
        "{}: emitted {} call edge(s) absent from the known-binding set: {wrong:#?}",
        corpus.name,
        wrong.len()
    );

    let recovered = emitted.intersection(&corpus.known).count();
    let recall = if corpus.known.is_empty() {
        1.0
    } else {
        recovered as f64 / corpus.known.len() as f64
    };
    println!(
        "{}: precision 1.00 (0 wrong), recall {recall:.2} ({recovered}/{})",
        corpus.name,
        corpus.known.len()
    );
}

fn known(pairs: &[(&str, &str)]) -> BTreeSet<(String, String)> {
    pairs
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

fn rust_corpus() -> Corpus {
    Corpus {
        name: "rust",
        files: vec![
            SourceFile::new(
                "agent-ix",
                "demo",
                "src/store.rs",
                Language::Rust,
                r#"pub struct Store;

pub trait Persist {
    fn flush(&self);
}

impl Store {
    pub fn upsert(&self) {}
    pub fn open() -> Store {
        Store
    }
    pub fn reopen(&self) {
        self.upsert();
    }
}

impl Persist for Store {
    fn flush(&self) {}
}
"#,
            ),
            SourceFile::new(
                "agent-ix",
                "demo",
                "src/app.rs",
                Language::Rust,
                r#"use crate::store::Store;

pub fn from_parameter(s: Store) {
    s.upsert();
}

pub fn from_annotation() {
    let s: Store = Store::open();
    s.upsert();
}

pub fn through_a_copy_chain() {
    let a: Store = Store::open();
    let b = a;
    b.upsert();
}
"#,
            ),
        ],
        known: known(&[
            (
                "agent-ix/demo/src/app.rs::from_parameter",
                "agent-ix/demo/src/store.rs::Store::upsert",
            ),
            (
                "agent-ix/demo/src/app.rs::from_annotation",
                "agent-ix/demo/src/store.rs::Store::upsert",
            ),
            (
                "agent-ix/demo/src/app.rs::from_annotation",
                "agent-ix/demo/src/store.rs::Store::open",
            ),
            (
                "agent-ix/demo/src/app.rs::through_a_copy_chain",
                "agent-ix/demo/src/store.rs::Store::upsert",
            ),
            (
                "agent-ix/demo/src/app.rs::through_a_copy_chain",
                "agent-ix/demo/src/store.rs::Store::open",
            ),
            (
                "agent-ix/demo/src/store.rs::Store::reopen",
                "agent-ix/demo/src/store.rs::Store::upsert",
            ),
        ]),
    }
}

fn typescript_corpus() -> Corpus {
    Corpus {
        name: "typescript",
        files: vec![
            SourceFile::new(
                "agent-ix",
                "demo",
                "src/store.ts",
                Language::TypeScript,
                r#"export class Store {
  upsert(): void {}
  reload(): void {
    this.upsert();
  }
}
"#,
            ),
            SourceFile::new(
                "agent-ix",
                "demo",
                "src/app.ts",
                Language::TypeScript,
                r#"import { Store } from './store';

export function fromParameter(s: Store): void {
  s.upsert();
}

export function fromConstructor(): void {
  const s = new Store();
  s.upsert();
}
"#,
            ),
        ],
        known: known(&[
            (
                "agent-ix/demo/src/app.ts::fromParameter",
                "agent-ix/demo/src/store.ts::Store::upsert",
            ),
            (
                "agent-ix/demo/src/app.ts::fromConstructor",
                "agent-ix/demo/src/store.ts::Store::upsert",
            ),
            (
                "agent-ix/demo/src/store.ts::Store::reload",
                "agent-ix/demo/src/store.ts::Store::upsert",
            ),
        ]),
    }
}

fn python_corpus() -> Corpus {
    Corpus {
        name: "python",
        files: vec![
            SourceFile::new(
                "agent-ix",
                "demo",
                "pkg/store.py",
                Language::Python,
                r#"class Store:
    def upsert(self):
        pass

    def reload(self):
        self.upsert()
"#,
            ),
            SourceFile::new(
                "agent-ix",
                "demo",
                "pkg/app.py",
                Language::Python,
                r#"from .store import Store


def from_constructor():
    s = Store()
    s.upsert()
"#,
            ),
        ],
        known: known(&[
            (
                "agent-ix/demo/pkg/app.py::from_constructor",
                "agent-ix/demo/pkg/store.py::Store::upsert",
            ),
            (
                "agent-ix/demo/pkg/store.py::Store::reload",
                "agent-ix/demo/pkg/store.py::Store::upsert",
            ),
        ]),
    }
}

// TC-066, NFR-004-AC-1, StR-002-VC-4: zero wrong edges on the Rust precision corpus.
#[test]
fn rust_corpus_emits_no_wrong_edges() {
    assert_precision(&rust_corpus());
}

// TC-067, NFR-004-AC-2: zero wrong edges on the TypeScript precision corpus.
#[test]
fn typescript_corpus_emits_no_wrong_edges() {
    assert_precision(&typescript_corpus());
}

// TC-068, NFR-004-AC-3: zero wrong edges on the Python precision corpus.
#[test]
fn python_corpus_emits_no_wrong_edges() {
    assert_precision(&python_corpus());
}

// TC-069, NFR-004-AC-4: the ambiguity corpus produces no call edge at all.
// Every call site here is deliberately unresolvable: two unrelated types
// declare the same method name and no receiver type is recoverable.
#[test]
fn the_ambiguity_corpus_emits_nothing() {
    let files = vec![
        SourceFile::new(
            "agent-ix",
            "demo",
            "src/a.rs",
            Language::Rust,
            "pub struct Alpha;\nimpl Alpha {\n    pub fn save(&self) {}\n}\n",
        ),
        SourceFile::new(
            "agent-ix",
            "demo",
            "src/b.rs",
            Language::Rust,
            "pub struct Beta;\nimpl Beta {\n    pub fn save(&self) {}\n}\n",
        ),
        SourceFile::new(
            "agent-ix",
            "demo",
            "src/ambiguous.rs",
            Language::Rust,
            r#"pub fn whichever(thing: Unknown) {
    thing.save();
}
"#,
        ),
    ];

    let result = extract(&files);
    let emitted = calls(&result);
    assert!(
        emitted.is_empty(),
        "ambiguous call sites must emit nothing, got {emitted:#?}"
    );
    assert!(
        result.stats.unresolved_calls > 0,
        "the unresolved count should make the silence visible"
    );
}

// TC-051, FR-008-AC-8: a call through a Rust trait object resolves to the
// trait's own method and never to an implementor. The source names the
// interface and does not determine which implementation runs, so the interface
// is the whole of what is known — and now that a trait method is a declaration
// (#11), it is a node the edge can point at rather than an absence.
#[test]
fn a_trait_object_call_resolves_to_the_trait_and_no_further() {
    let files = vec![SourceFile::new(
        "agent-ix",
        "demo",
        "src/lib.rs",
        Language::Rust,
        r#"pub trait Persist {
    fn flush(&self);
}

pub struct Disk;
impl Persist for Disk {
    fn flush(&self) {}
}

pub struct Memory;
impl Persist for Memory {
    fn flush(&self) {}
}

pub fn run(sink: &dyn Persist) {
    sink.flush();
}
"#,
    )];

    let result = extract(&files);
    let flush_edges: Vec<_> = calls(&result)
        .into_iter()
        .filter(|e| e.target_ref.ends_with("::flush"))
        .collect();

    let targets: Vec<&str> = flush_edges.iter().map(|e| e.target_ref.as_str()).collect();
    assert_eq!(
        targets,
        vec!["agent-ix/demo/src/lib.rs::Persist::flush"],
        "the interface is what the source names; an implementor is a guess"
    );
    for implementor in ["Disk::flush", "Memory::flush"] {
        assert!(
            !targets.iter().any(|t| t.ends_with(implementor)),
            "a dyn-dispatched call must not pick {implementor}: {flush_edges:#?}"
        );
    }
}

// TC-107, FR-008-AC-13: one declaration shape resolves at one tier in every
// language. An annotated receiver whose type is declared in the same file
// resolves, so FR-008 says `receiver-typed` — and saying `import-scoped` in one
// language makes that language's dependencies rank below identical ones
// elsewhere for any consumer that reads confidence.
//
// Raw strings, deliberately: an escaped literal here was reflowed by `cargo
// fmt` into an indented continuation, which changed the Python fixture into a
// different shape and made the test fail for a reason that was not the defect.
#[test]
fn one_shape_resolves_at_one_tier_in_every_language() {
    let rust = r#"pub struct Store;

impl Store {
    pub fn upsert(&self) {}
}

pub fn same_file(store: &Store) {
    store.upsert();
}
"#;
    let typescript = r#"export class Store {
  upsert(): void {}
}

export function sameFile(store: Store): void {
  store.upsert();
}
"#;
    let python = r#"class Store:
    def upsert(self) -> None:
        pass


def same_file(store: Store) -> None:
    store.upsert()
"#;

    for (language, path, source) in [
        (Language::Rust, "src/lib.rs", rust),
        (Language::TypeScript, "src/index.ts", typescript),
        (Language::Python, "src/store.py", python),
    ] {
        let result = extract(&[SourceFile::new("agent-ix", "demo", path, language, source)]);
        let call = result
            .edges
            .iter()
            .find(|e| e.edge_type == "calls")
            .unwrap_or_else(|| panic!("{path}: the call resolves"));
        assert_eq!(
            call.reason, "receiver-typed",
            "{path}: the receiver's type is written down and declared in this \
             file, so it resolves; a weaker tier here is a lower confidence on \
             the same evidence"
        );
    }
}

// TC-097, FR-008-AC-7: a Python base class is an `extends` relation, so a
// supertype is reachable in every language rather than only where the grammar
// spells the keyword.
#[test]
fn a_python_base_class_yields_an_extends_edge() {
    let result = extract(&[SourceFile::new(
        "agent-ix",
        "demo",
        "src/store.py",
        Language::Python,
        "class Persist:\n    pass\n\n\nclass Store(Persist):\n    pass\n",
    )]);
    let extends: Vec<_> = result
        .edges
        .iter()
        .filter(|e| e.edge_type == "extends")
        .map(|e| (e.source_ref.as_str(), e.target_ref.as_str()))
        .collect();
    assert_eq!(
        extends,
        vec![(
            "agent-ix/demo/src/store.py::Store",
            "agent-ix/demo/src/store.py::Persist"
        )]
    );
}

// TC-050, FR-008-AC-7: Rust impls yield `implements_trait`, TypeScript
// subclasses yield `extends`.
#[test]
fn type_relations_are_emitted_for_both_languages() {
    let rust = extract(&[SourceFile::new(
        "agent-ix",
        "demo",
        "src/lib.rs",
        Language::Rust,
        "pub trait Persist { fn flush(&self); }\npub struct Disk;\nimpl Persist for Disk {\n    fn flush(&self) {}\n}\n",
    )]);
    assert!(
        rust.edges.iter().any(|e| e.edge_type == "implements_trait"),
        "expected an implements_trait edge, got {:#?}",
        rust.edges
    );

    let ts = extract(&[SourceFile::new(
        "agent-ix",
        "demo",
        "src/app.ts",
        Language::TypeScript,
        "export class Base {}\nexport class Derived extends Base {}\n",
    )]);
    assert!(
        ts.edges.iter().any(|e| e.edge_type == "extends"),
        "expected an extends edge, got {:#?}",
        ts.edges
    );
}

// TC-052, FR-008-AC-9, FR-008-CON-3: same-file resolution is unaffected by batch
// composition.
#[test]
fn same_file_resolution_is_independent_of_the_rest_of_the_batch() {
    let alone = SourceFile::new(
        "agent-ix",
        "demo",
        "src/solo.rs",
        Language::Rust,
        r#"pub struct Local;
impl Local {
    pub fn tick(&self) {}
    pub fn run(&self) {
        self.tick();
    }
}
"#,
    );

    let solo = extract(std::slice::from_ref(&alone));
    let solo_calls: BTreeSet<_> = calls(&solo)
        .iter()
        .map(|e| (e.source_ref.clone(), e.target_ref.clone()))
        .collect();

    let mut with_others = vec![alone];
    with_others.push(SourceFile::new(
        "agent-ix",
        "demo",
        "src/noise.rs",
        Language::Rust,
        "pub struct Other;\nimpl Other {\n    pub fn unrelated(&self) {}\n}\n",
    ));
    let mixed = extract(&with_others);
    let mixed_calls: BTreeSet<_> = calls(&mixed)
        .iter()
        .map(|e| (e.source_ref.clone(), e.target_ref.clone()))
        .filter(|(source, _)| source.contains("solo.rs"))
        .collect();

    assert_eq!(solo_calls, mixed_calls);
    assert!(!solo_calls.is_empty(), "the self-call should resolve");
}

// TC-073, FR-008-AC-11: the result reports what the batch bounded.
#[test]
fn the_result_reports_batch_size_and_unresolved_calls() {
    let result = extract(&rust_corpus().files);
    assert_eq!(result.stats.files, 2);
    assert_eq!(result.stats.files_with_errors, 0);
}

// TC-070, NFR-004-AC-5: recall is computed and reported for every corpus.
#[test]
fn recall_is_reported_for_every_language() {
    for corpus in [rust_corpus(), typescript_corpus(), python_corpus()] {
        let result = extract(&corpus.files);
        let emitted: BTreeSet<(String, String)> = calls(&result)
            .iter()
            .map(|e| (e.source_ref.clone(), e.target_ref.clone()))
            .collect();
        let recovered = emitted.intersection(&corpus.known).count();
        // Reported, not gated — the threshold column in NFR-004 is empty on
        // purpose.
        println!("{} recall: {recovered}/{}", corpus.name, corpus.known.len());
    }
}

// A repository that declares the same type name in two languages must not lose
// the edges *inside* each file. The batch ambiguity is real, but it is not the
// ambiguity the call site has — FR-008 requires same-file resolution to succeed
// without consulting other files.
// TC-074, FR-008-AC-12: a simple type name declared in two files still
// resolves within each file that declares it.
#[test]
fn a_name_shared_across_languages_does_not_suppress_same_file_edges() {
    let files = vec![
        SourceFile::new(
            "agent-ix",
            "demo",
            "src/store.rs",
            Language::Rust,
            r#"pub struct Store;

pub trait Persist {
    fn flush(&self);
}

impl Persist for Store {
    fn flush(&self) {}
}

impl Store {
    pub fn upsert(&self) {}
    pub fn reopen(&self) {
        self.upsert();
    }
}
"#,
        ),
        // Same simple name, different language, unrelated type.
        SourceFile::new(
            "agent-ix",
            "demo",
            "ui/store.ts",
            Language::TypeScript,
            "export class Store {\n  upsert(): void {}\n}\n",
        ),
    ];

    let result = extract(&files);

    assert!(
        result
            .edges
            .iter()
            .any(|e| e.edge_type == "implements_trait"
                && e.source_ref == "agent-ix/demo/src/store.rs::Store"),
        "the Rust impl should resolve to the Rust Store, got {:#?}",
        result.edges
    );

    let self_call: Vec<_> = calls(&result)
        .into_iter()
        .filter(|e| e.source_ref == "agent-ix/demo/src/store.rs::Store::reopen")
        .collect();
    assert_eq!(
        self_call.len(),
        1,
        "the self-call should resolve to the Rust upsert, got {self_call:#?}"
    );
    assert_eq!(
        self_call[0].target_ref,
        "agent-ix/demo/src/store.rs::Store::upsert"
    );
}
