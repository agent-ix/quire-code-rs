//! Real ecosystem lane for the governed measurement producer.
//!
//! Run after a clean release build with:
//! `QUIRE_CORPUS_ROOT=../quire-corpus QUOIN_BIN=quoin QUIRE_BIN=quire cargo test --test measurement_pipeline -- --ignored --nocapture`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn output(command: &mut Command) -> String {
    let result = command.output().expect("command starts");
    assert!(
        result.status.success(),
        "command failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).expect("UTF-8 stdout")
}

fn revision(root: &Path) -> String {
    output(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["rev-parse", "HEAD"]),
    )
    .trim()
    .to_string()
}

fn remote(root: &Path) -> String {
    output(
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["remote", "get-url", "origin"]),
    )
    .trim()
    .to_string()
}

// TC-108, TC-109, TC-110, TC-111, TC-118, TC-119, TC-124, TC-125, TC-129 /
// StR-003-VC-1..4, US-004-EX-1..2, FR-012-AC-1..2, FR-012-AC-7..8,
// NFR-005-AC-1, MP-001, IT-001: real extractor + scorer + Quire + Quoin.
#[test]
#[ignore = "requires clean pinned sibling checkouts and installed Quire/Quoin CLIs"]
fn real_corpus_observation_is_accepted_and_rendered_by_quoin() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let corpus = PathBuf::from(std::env::var("QUIRE_CORPUS_ROOT").expect("QUIRE_CORPUS_ROOT"));
    let quoin = std::env::var("QUOIN_BIN").expect("QUOIN_BIN");
    let quire = std::env::var("QUIRE_BIN").expect("QUIRE_BIN");
    let python = std::env::var("PYTHON_BIN").unwrap_or_else(|_| "python3".into());
    let release_extractor = repo.join("target/release/extract_tree");
    assert!(release_extractor.is_file(), "run make build first");

    let scratch =
        std::env::temp_dir().join(format!("quire-code-measurement-{}", std::process::id()));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("remove stale scratch");
    }
    let output_dir = scratch.join("producer");
    let intake = scratch.join("intake");
    fs::create_dir_all(intake.join("spec/assurance")).expect("create intake");
    fs::copy(
        repo.join("spec/assurance/MP-001-graph-quality-observation.md"),
        intake.join("spec/assurance/MP-001-graph-quality-observation.md"),
    )
    .expect("copy plan");

    let source_revision = revision(&repo);
    let corpus_revision = revision(&corpus);
    let config_digest = format!("sha256:{}", "1".repeat(64));
    let first = output(
        Command::new(env!("CARGO_BIN_EXE_measure_graph_quality"))
            .args(["--repo-root"])
            .arg(&repo)
            .args(["--corpus"])
            .arg(&corpus)
            .args(["--extractor"])
            .arg(&release_extractor)
            .args(["--quire", &quire, "--python", &python])
            .args(["--output-dir"])
            .arg(&output_dir)
            .args([
                "--timestamp",
                "2026-08-31T00:00:00.000Z",
                "--source-revision",
                &source_revision,
                "--corpus-source-revision",
                &corpus_revision,
                "--scorer-revision",
                &corpus_revision,
                "--config-digest",
                &config_digest,
                "--grammar",
                "rust=tree-sitter-rust@0.24.2",
                "--grammar",
                "typescript=tree-sitter-typescript@0.23.2",
                "--grammar",
                "tsx=tree-sitter-typescript@0.23.2",
                "--grammar",
                "python=tree-sitter-python@0.25.0",
                "--case",
                "declarations/declaration-forms/rust",
                "--case",
                "relations/receiver-typed-call/rust",
                "--node-version",
                "24.15.0",
                "--rust-version",
                "1.95.0",
                "--python-version",
                "3.14.7",
                "--source-remote",
                &remote(&repo),
                "--corpus-remote",
                &remote(&corpus),
            ]),
    );
    let second = output(
        Command::new(env!("CARGO_BIN_EXE_measure_graph_quality"))
            .args(["--repo-root"])
            .arg(&repo)
            .args(["--corpus"])
            .arg(&corpus)
            .args(["--extractor"])
            .arg(&release_extractor)
            .args(["--quire", &quire, "--python", &python])
            .args(["--output-dir"])
            .arg(output_dir.with_file_name("producer-second"))
            .args([
                "--timestamp",
                "2026-08-31T00:00:00.000Z",
                "--source-revision",
                &source_revision,
                "--corpus-source-revision",
                &corpus_revision,
                "--scorer-revision",
                &corpus_revision,
                "--config-digest",
                &config_digest,
                "--grammar",
                "rust=tree-sitter-rust@0.24.2",
                "--grammar",
                "typescript=tree-sitter-typescript@0.23.2",
                "--grammar",
                "tsx=tree-sitter-typescript@0.23.2",
                "--grammar",
                "python=tree-sitter-python@0.25.0",
                "--case",
                "declarations/declaration-forms/rust",
                "--case",
                "relations/receiver-typed-call/rust",
                "--node-version",
                "24.15.0",
                "--rust-version",
                "1.95.0",
                "--python-version",
                "3.14.7",
                "--source-remote",
                &remote(&repo),
                "--corpus-remote",
                &remote(&corpus),
            ]),
    );
    assert_eq!(first, second, "pinned repetitions are byte-identical");

    let collection = scratch.join("collection.json");
    fs::write(&collection, first).expect("write collection");
    output(
        Command::new(&quoin)
            .args(["measurement", "record", "--repo"])
            .arg(&intake)
            .args(["--input"])
            .arg(&collection),
    );
    let report = output(Command::new(&quoin).args(["report", "--repo"]).arg(&intake));
    assert!(report.contains("graph_quality"));
    assert!(report.contains("measure=recall"));
    assert!(report.contains("measure=false_positive"));

    fs::remove_dir_all(&scratch).expect("remove scratch");
}
