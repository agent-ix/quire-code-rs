//! End-to-end extraction tests.
//!
//! Covers the acceptance criteria that only hold across a whole batch:
//! determinism (NFR-001), the no-network boundary (NFR-002), golden-file
//! parity (FR-006-AC-6), and the dogfooding check that this repository's own
//! tracking tags are recoverable (FR-005-AC-7).

use std::collections::BTreeSet;

use quire_code_rs::{extract, extract_with, parse_file, Language, MentionKind, SourceFile};

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

// TC-046, FR-001-AC-1, FR-001-AC-2, FR-001-AC-3: a mixed-language batch yields  (structural slice)
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

// TC-038, FR-006-AC-6: extraction of the fixture matches its golden file.
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

// TC-054, NFR-001-AC-1: one hundred repeated extractions are byte-identical.
#[test]
fn a_hundred_extractions_are_byte_identical() {
    let files = fixture();
    let first = serde_json::to_vec(&extract(&files)).expect("serializes");
    for run in 1..100 {
        let again = serde_json::to_vec(&extract(&files)).expect("serializes");
        assert_eq!(first, again, "run {run} diverged");
    }
}

// TC-055, NFR-001-AC-2, StR-001-VC-4: concurrent extractions are byte-identical.
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

// TC-056, NFR-001-AC-3: shuffling the batch order changes nothing.
#[test]
fn shuffled_batch_order_produces_identical_output() {
    let forward = extract(&fixture());
    let mut shuffled = fixture();
    shuffled.rotate_left(2);
    shuffled.reverse();
    assert_eq!(extract(&shuffled), forward);
}

// TC-032, FR-005-AC-7 — the dogfooding check: extracting this repository's own
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

// TC-059, NFR-002-AC-1: no HTTP, RPC or socket client crate in the closure.
#[test]
fn the_dependency_closure_holds_no_network_client() {
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
}

// TC-007, FR-001-AC-7, StR-001-VC-2: a dependency audit confirms no filesystem or network
// crate is reachable from the extraction path.
//
// An allowlist rather than a denylist: a denylist only rejects the crates
// somebody thought to name, and the criterion is about what the extraction path
// *may* reach, not about which of today's crates it happens not to.
#[test]
fn the_declared_dependencies_are_the_audited_set() {
    let root = env!("CARGO_MANIFEST_DIR");
    let manifest = std::fs::read_to_string(format!("{root}/Cargo.toml")).expect("Cargo.toml");
    let section = manifest
        .split("[dependencies]")
        .nth(1)
        .expect("a [dependencies] section")
        .split("\n[")
        .next()
        .expect("the section ends at the next table");

    let declared: BTreeSet<&str> = section
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && l.contains('='))
        .filter_map(|l| l.split('=').next())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect();

    // Parsing, serialization and hashing. Nothing here opens a file, a socket
    // or a process; adding a dependency that could is what this test is for.
    //
    // Re-audited for PLAT-849: `tree-sitter`/`tree-sitter-rust`/
    // `tree-sitter-typescript`/`tree-sitter-python` are no longer declared
    // here directly — they moved behind `quire-code-parse`
    // (`crates/quire-code-parse`, enforced by `tests/dependency_boundary.rs`).
    // `quire-code-parse` is in the audited set in their place: its own crate
    // docs state it performs no I/O ("This crate never reads a path, opens a
    // socket, or touches the filesystem"), so its closure is still within
    // what FR-001-AC-7 allows.
    let audited: BTreeSet<&str> = [
        // Draft-2020-12 validation runs with resolver features disabled; its
        // closure contains no HTTP client and cannot fetch a remote schema.
        "jsonschema",
        "quire-code-parse",
        "regex",
        "serde",
        "serde_json",
        "sha2",
        "thiserror",
    ]
    .into_iter()
    .collect();

    assert_eq!(
        declared, audited,
        "the dependency set changed; re-audit it against FR-001-AC-7 before widening this list"
    );
}

// TC-060, NFR-002-AC-2, FR-003-CON-1: no filesystem, environment or process access in the
// extraction path.
#[test]
fn extraction_touches_no_ambient_state() {
    // Tests are exempt — this very test reads files — so only the extraction
    // sources `for_each_extraction_source` sweeps are audited, with their own
    // test modules stripped.
    for_each_extraction_source(|path, code| {
        for banned in ["std::fs", "std::net", "std::env", "std::process"] {
            assert!(
                !code.contains(banned),
                "{path} uses {banned} outside its test module"
            );
        }
    });
}

// TC-058, NFR-001-AC-5: a static audit finds no clock, randomness, process or
// environment read in extraction paths.
//
// Separate from TC-060 because the two criteria fail for different reasons: an
// ambient *read* breaks the filesystem boundary, while a clock or a random
// source breaks determinism even where no boundary is crossed.
#[test]
fn extraction_reads_no_clock_and_no_randomness() {
    for_each_extraction_source(|path, code| {
        for banned in [
            "SystemTime",
            "Instant::now",
            "thread_rng",
            "rand::",
            "RandomState",
        ] {
            assert!(
                !code.contains(banned),
                "{path} uses {banned} outside its test module; NFR-001 requires \
                 extraction to be a pure function of its input"
            );
        }
    });
}

/// Run `check` over every `*.rs` file under `src/` and, as of PLAT-849,
/// `crates/quire-code-parse/src/` — with comments and test modules stripped
/// — the code that actually runs during extraction.
///
/// PLAT-849 review N2: before this swept `quire-code-parse` too,
/// `the_declared_dependencies_are_the_audited_set` admitted `quire-code-parse`
/// into the audited dependency set on the strength of a sentence in its own
/// crate docs ("This crate never reads a path, opens a socket, or touches the
/// filesystem"), with no compiled check behind it — a stated dependency
/// property needs a compiled assertion here, not a sentence, and this
/// crate's own parsing now runs through `quire-code-parse` on every
/// extraction, so its source is as much "the extraction path" as this
/// package's own `src/`. Widening this one helper closes the same hole in
/// every check built on it: `extraction_touches_no_ambient_state`,
/// `extraction_reads_no_clock_and_no_randomness`, and (via the refactor
/// below) `extraction_paths_use_ordered_collections_only`, which duplicated
/// this walk against `src/` alone before this change.
fn for_each_extraction_source(check: impl Fn(String, &str)) {
    let root = env!("CARGO_MANIFEST_DIR");
    for dir in [
        format!("{root}/src"),
        format!("{root}/crates/quire-code-parse/src"),
    ] {
        for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{dir} is readable: {e}")) {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("source is readable");
            let code = strip_comments(&strip_test_modules(&source));
            check(path.display().to_string(), &code);
        }
    }
}

// TC-057, NFR-001-AC-4: no order-observable hash iteration in extraction.
#[test]
fn extraction_paths_use_ordered_collections_only() {
    for_each_extraction_source(|path, code| {
        // Match real usage — `HashMap<`, `HashMap::`, `use …HashMap` — rather
        // than the bare word, which appears in the comments explaining why
        // these modules use BTree collections instead.
        for banned in ["HashMap", "HashSet"] {
            for form in [format!("{banned}<"), format!("{banned}::")] {
                assert!(
                    !code.contains(&form),
                    "{path} uses {form}; iteration order is observable in the output, \
                     so NFR-001 requires a BTree collection"
                );
            }
        }
    });
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

/// Desc: a consumer that already holds a batch's parses can hand them back
/// instead of having the library parse the corpus again, and the records it
/// gets are the ones a full extraction produces. This is the seam that makes
/// the single-file re-extraction budget reachable from outside: resolution
/// still needs the whole batch, but parsing it is the part a caller who
/// changed one file need not repeat.
/// Assumptions: the fixture is the same mixed-language repository the rest of
/// this suite uses, so the comparison covers every record kind it emits.
/// ACs: NFR-003-AC-5.
/// Trace: TC-077, NFR-003-AC-5
#[test]
fn tc_077_caller_supplied_parses_are_reused_and_change_nothing() {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    let files = fixture();
    let expected = extract(&files);

    // First pass: every file is a miss, and the memo fills.
    let mut memo: BTreeMap<String, Arc<quire_code_rs::ParsedFile>> = BTreeMap::new();
    let mut parses = 0usize;
    let first = extract_with(&files, &mut |file| {
        let key = file.normalized_path();
        memo.entry(key)
            .or_insert_with(|| {
                parses += 1;
                Arc::new(parse_file(file))
            })
            .clone()
    });
    assert_eq!(
        parses,
        files.len(),
        "the first pass parses each file exactly once"
    );

    // Second pass: nothing changed, so the library must ask for parses and
    // accept every one of them without parsing anything itself.
    let mut reparses = 0usize;
    let second = extract_with(&files, &mut |file| {
        let key = file.normalized_path();
        memo.get(&key).cloned().unwrap_or_else(|| {
            reparses += 1;
            Arc::new(parse_file(file))
        })
    });
    assert_eq!(reparses, 0, "an unchanged batch re-parses nothing");

    assert_eq!(
        first.nodes, expected.nodes,
        "supplying parses changes no node record"
    );
    assert_eq!(
        first.edges, expected.edges,
        "supplying parses changes no edge record"
    );
    assert_eq!(second.nodes, expected.nodes, "and is stable across runs");
    assert_eq!(second.edges, expected.edges, "and is stable across runs");
    assert_eq!(second.stats, expected.stats, "including the batch stats");
}
