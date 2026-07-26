//! End-to-end extraction tests.
//!
//! Covers the acceptance criteria that only hold across a whole batch:
//! determinism (NFR-001), the no-network boundary (NFR-002), golden-file
//! parity (FR-006-AC-6), and the dogfooding check that this repository's own
//! tracking tags are recoverable (FR-005-AC-7).

use std::collections::BTreeSet;

use quire_code_rs::{extract, Language, MentionKind, SourceFile};

/// A small mixed-language fixture repository.
fn fixture() -> Vec<SourceFile> {
    vec![
        SourceFile::new(
            "agent-ix",
            "demo",
            "src/store.rs",
            Language::Rust,
            r#"//! Storage. Implements FR-002 and see ix://agent-ix/demo/FR-003.

pub struct Store {
    pub count: u32,
}

pub trait Persist {
    fn save(&self);
}

impl Store {
    pub fn upsert(&self) {}
}

impl Persist for Store {
    fn save(&self) {}
}

pub enum Mode {
    On,
    Off,
}
"#,
        ),
        SourceFile::new(
            "agent-ix",
            "demo",
            "src/lib.rs",
            Language::Rust,
            r#"use crate::store::Store;

pub mod inner {
    pub fn helper() {}
}

#[test]
fn exercises_the_store() {
    // TC-001 — the store round-trips.
    let _ = 1;
}
"#,
        ),
        SourceFile::new(
            "agent-ix",
            "demo",
            "ui/store.ts",
            Language::TypeScript,
            r#"export interface Shape {
  n: number;
}

export type Alias = Shape;

export class Store {
  upsert(): void {}
}
"#,
        ),
        SourceFile::new(
            "agent-ix",
            "demo",
            "ui/app.ts",
            Language::TypeScript,
            r#"import { Store } from './store';

export function boot(): void {}
"#,
        ),
        SourceFile::new(
            "agent-ix",
            "demo",
            "tool/main.py",
            Language::Python,
            r#""""Entry point. Implements NFR-001."""


class Runner:
    def run(self):
        pass


def main():
    pass
"#,
        ),
    ]
}

// TC-046 (structural slice) — FR-001-AC-1/2/3: a mixed-language batch yields
// the four node types and structural edges across every language.
#[test]
fn mixed_language_batch_yields_every_node_type() {
    let result = extract(&fixture());

    let types: BTreeSet<&str> = result
        .nodes
        .iter()
        .map(|n| n.object_type.as_str())
        .collect();
    assert!(types.contains("code_file"), "got {types:?}");
    assert!(types.contains("code_module"), "got {types:?}");
    assert!(types.contains("code_function"), "got {types:?}");
    assert!(types.contains("code_type"), "got {types:?}");

    let edge_types: BTreeSet<&str> = result.edges.iter().map(|e| e.edge_type.as_str()).collect();
    assert!(edge_types.contains("contains"), "got {edge_types:?}");
    assert!(edge_types.contains("imports"), "got {edge_types:?}");
}

// TC-038 — FR-006-AC-6: extraction of the fixture matches its golden file.
#[test]
fn fixture_extraction_matches_the_golden_file() {
    let result = extract(&fixture());
    let actual = serde_json::to_string_pretty(&result).expect("serializes");

    let golden_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/fixture.json");
    let golden = std::fs::read_to_string(golden_path).unwrap_or_default();

    if golden.trim() != actual.trim() {
        // Write the actual output beside the golden so a mismatch is easy to
        // inspect and, when intended, to accept.
        let actual_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/golden/fixture.actual.json"
        );
        let _ = std::fs::write(actual_path, &actual);
        panic!(
            "extraction differs from the golden file.\n\
             Inspect tests/golden/fixture.actual.json; if the change is intended, \
             copy it over tests/golden/fixture.json."
        );
    }
}

// TC-054 — NFR-001-AC-1: one hundred repeated extractions are byte-identical.
#[test]
fn a_hundred_extractions_are_byte_identical() {
    let files = fixture();
    let first = serde_json::to_vec(&extract(&files)).expect("serializes");
    for run in 1..100 {
        let again = serde_json::to_vec(&extract(&files)).expect("serializes");
        assert_eq!(first, again, "run {run} diverged");
    }
}

// TC-055 — NFR-001-AC-2: concurrent extractions are byte-identical.
#[test]
fn concurrent_extractions_are_byte_identical() {
    let files = fixture();
    let expected = serde_json::to_vec(&extract(&files)).expect("serializes");

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let files = files.clone();
            std::thread::spawn(move || serde_json::to_vec(&extract(&files)).expect("serializes"))
        })
        .collect();

    for handle in handles {
        assert_eq!(handle.join().expect("thread completes"), expected);
    }
}

// TC-056 — NFR-001-AC-3: shuffling the batch order changes nothing.
#[test]
fn shuffled_batch_order_produces_identical_output() {
    let forward = extract(&fixture());
    let mut shuffled = fixture();
    shuffled.rotate_left(2);
    shuffled.reverse();
    assert_eq!(extract(&shuffled), forward);
}

// TC-032 / FR-005-AC-7 — the dogfooding check: extracting this repository's own
// sources recovers the TC tags its tests carry, using the library the tags
// document.
#[test]
fn self_extraction_recovers_this_repositorys_own_tracking_tags() {
    let root = env!("CARGO_MANIFEST_DIR");
    let mut files = Vec::new();
    for relative in [
        "src/lib.rs",
        "src/lang.rs",
        "src/facts.rs",
        "src/naming.rs",
        "src/edges.rs",
        "src/parse.rs",
        "src/imports.rs",
        "src/mentions.rs",
        "src/records.rs",
        "src/extract.rs",
    ] {
        let content = std::fs::read_to_string(format!("{root}/{relative}"))
            .unwrap_or_else(|e| panic!("reading {relative}: {e}"));
        files.push(SourceFile::new(
            "agent-ix",
            "quire-code-rs",
            relative,
            Language::Rust,
            content,
        ));
    }

    let result = extract(&files);

    let tags: BTreeSet<&str> = result
        .mentions
        .iter()
        .filter(|m| m.kind == MentionKind::TrackingTag)
        .map(|m| m.identifier.as_str())
        .collect();

    // The tags carried by the unit tests in the modules above.
    for expected in ["TC-008", "TC-013", "TC-020", "TC-026", "TC-033"] {
        assert!(
            tags.contains(expected),
            "{expected} was not recovered from this repository's own source; got {tags:?}"
        );
    }

    // And the requirement citations in the module docs.
    let citations: BTreeSet<&str> = result
        .mentions
        .iter()
        .filter(|m| m.kind == MentionKind::RequirementCitation)
        .map(|m| m.identifier.as_str())
        .collect();
    assert!(
        citations.contains("FR-001"),
        "module docs cite FR-001; got {citations:?}"
    );

    assert_eq!(result.stats.files_with_errors, 0, "our own sources parse");
}

// TC-059 — NFR-002-AC-1: no HTTP, RPC or socket client crate in the closure.
// TC-060 — NFR-002-AC-2: no filesystem, environment or process access in the
// extraction path.
#[test]
fn the_crate_reaches_nothing_outside_its_inputs() {
    let root = env!("CARGO_MANIFEST_DIR");

    let lock = std::fs::read_to_string(format!("{root}/Cargo.lock")).expect("Cargo.lock");
    for banned in [
        "reqwest",
        "hyper",
        "tonic",
        "curl",
        "ureq",
        "isahc",
        "surf",
        "tungstenite",
        "quinn",
    ] {
        assert!(
            !lock.contains(&format!("name = \"{banned}\"")),
            "{banned} is in the dependency closure; NFR-002 forbids network clients"
        );
    }

    // The extraction path itself touches no ambient state. Tests are exempt —
    // this very test reads files — so only `src/` is audited.
    for entry in std::fs::read_dir(format!("{root}/src")).expect("src is readable") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("source is readable");
        let code = strip_comments(&strip_test_modules(&source));
        for banned in [
            "std::fs",
            "std::net",
            "std::env",
            "std::process",
            "SystemTime",
            "Instant::now",
        ] {
            assert!(
                !code.contains(banned),
                "{} uses {banned} outside its test module",
                path.display()
            );
        }
    }
}

// TC-057 — NFR-001-AC-4: no order-observable hash iteration in extraction.
#[test]
fn extraction_paths_use_ordered_collections_only() {
    let root = env!("CARGO_MANIFEST_DIR");
    for entry in std::fs::read_dir(format!("{root}/src")).expect("src is readable") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("source is readable");
        let code = strip_comments(&strip_test_modules(&source));
        // Match real usage — `HashMap<`, `HashMap::`, `use …HashMap` — rather
        // than the bare word, which appears in the comments explaining why
        // these modules use BTree collections instead.
        for banned in ["HashMap", "HashSet"] {
            for form in [format!("{banned}<"), format!("{banned}::")] {
                assert!(
                    !code.contains(&form),
                    "{} uses {form}; iteration order is observable in the output, \
                     so NFR-001 requires a BTree collection",
                    path.display()
                );
            }
        }
    }
}

/// Drop `#[cfg(test)] mod tests { … }` so audits cover only shipping code.
fn strip_test_modules(source: &str) -> String {
    match source.find("#[cfg(test)]") {
        Some(idx) => source[..idx].to_string(),
        None => source.to_string(),
    }
}

/// Drop line comments so an audit sees code, not prose about the code.
fn strip_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
