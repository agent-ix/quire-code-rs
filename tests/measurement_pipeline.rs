//! Real ecosystem lane for the governed measurement producer.
//!
//! Run after a clean release build with:
//! `QUIRE_CORPUS_ROOT=../quire-corpus QUOIN_BIN=quoin QUIRE_BIN=quire cargo test --test measurement_pipeline -- --ignored --nocapture`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use quire_code_rs::measurement::sha256;

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

fn reversed_creation_checkout(source: &Path, destination: &Path) {
    output(
        Command::new("git")
            .args(["clone", "--quiet", "--no-hardlinks"])
            .arg(source)
            .arg(destination),
    );
    let tracked = output(
        Command::new("git")
            .arg("-C")
            .arg(destination)
            .args(["ls-files"]),
    );
    let mut files = tracked.lines().map(PathBuf::from).collect::<Vec<_>>();
    let contents = files
        .iter()
        .map(|relative| {
            let path = destination.join(relative);
            (
                relative.clone(),
                fs::read(&path).expect("read tracked file"),
                fs::metadata(path).expect("tracked metadata").permissions(),
            )
        })
        .collect::<Vec<_>>();
    for relative in &files {
        fs::remove_file(destination.join(relative)).expect("remove tracked file");
    }
    files.reverse();
    for relative in files {
        let bytes = contents
            .iter()
            .find(|(candidate, _, _)| candidate == &relative)
            .map(|(_, bytes, _)| bytes)
            .unwrap();
        let path = destination.join(&relative);
        fs::write(&path, bytes).expect("rewrite tracked file");
        let permissions = contents
            .iter()
            .find(|(candidate, _, _)| candidate == &relative)
            .map(|(_, _, permissions)| permissions.clone())
            .unwrap();
        fs::set_permissions(path, permissions).expect("restore tracked permissions");
    }
    assert!(
        output(
            Command::new("git")
                .arg("-C")
                .arg(destination)
                .args(["status", "--porcelain"]),
        )
        .is_empty(),
        "reordered checkout must retain identical tracked bytes"
    );
}

fn complete_cli_args() -> Vec<String> {
    [
        ("--repo-root", "."),
        ("--corpus", "../quire-corpus"),
        ("--extractor", "target/release/extract_tree"),
        ("--quire", "quire"),
        ("--python", "python3"),
        ("--output-dir", "target/measurement"),
        ("--timestamp", "2026-08-31T00:00:00Z"),
        (
            "--source-revision",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        (
            "--corpus-source-revision",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
        (
            "--scorer-revision",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
        (
            "--config-digest",
            "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        ),
        ("--grammar", "python=tree-sitter-python@0.25.0"),
        ("--grammar", "rust=tree-sitter-rust@0.24.2"),
        ("--grammar", "tsx=tree-sitter-typescript@0.23.2"),
        ("--grammar", "typescript=tree-sitter-typescript@0.23.2"),
        ("--node-version", "24.15.0"),
        ("--rust-version", "1.95.0"),
        ("--python-version", "3.14.7"),
        ("--source-remote", "local-source"),
        ("--corpus-remote", "local-corpus"),
    ]
    .into_iter()
    .flat_map(|(flag, value)| [flag.to_string(), value.to_string()])
    .collect()
}

// TC-123 / FR-012-AC-6: every missing identity fails the real CLI process
// before stdout can contain an observation.
#[test]
fn missing_identity_exits_nonzero_with_empty_stdout_and_exact_field() {
    let complete = complete_cli_args();
    for missing in [
        "--source-revision",
        "--corpus-source-revision",
        "--scorer-revision",
        "--config-digest",
        "--grammar",
        "--timestamp",
        "--node-version",
        "--rust-version",
        "--python-version",
        "--source-remote",
        "--corpus-remote",
    ] {
        let args = complete
            .chunks_exact(2)
            .filter(|pair| pair[0] != missing)
            .flat_map(|pair| pair.iter().cloned())
            .collect::<Vec<_>>();
        let result = Command::new(env!("CARGO_BIN_EXE_measure_graph_quality"))
            .args(args)
            .output()
            .expect("measurement CLI starts");
        assert!(!result.status.success(), "{missing}");
        assert!(result.stdout.is_empty(), "{missing} emitted stdout");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(missing),
            "{missing}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

// TC-108, TC-109, TC-110, TC-111, TC-118, TC-119, TC-124, TC-125, TC-127, TC-129 /
// StR-003-VC-1..4, US-004-EX-1..2, FR-012-AC-1..2, FR-012-AC-7..8,
// NFR-005-AC-1..2, MP-001, IT-001: real extractor + scorer + Quire + Quoin.
#[test]
#[ignore = "requires clean pinned sibling checkouts and installed Quire/Quoin CLIs"]
fn real_corpus_observation_is_accepted_and_rendered_by_quoin() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let corpus = PathBuf::from(std::env::var("QUIRE_CORPUS_ROOT").expect("QUIRE_CORPUS_ROOT"));
    let quoin = std::env::var("QUOIN_BIN").expect("QUOIN_BIN");
    let quire = std::env::var("QUIRE_BIN").expect("QUIRE_BIN");
    let python = std::env::var("PYTHON_BIN").unwrap_or_else(|_| "python3".into());
    let release_extractor = repo.join("target/release/extract_tree");
    let release_producer = repo.join("target/release/measure_graph_quality");
    assert!(release_extractor.is_file(), "run make build first");
    assert!(release_producer.is_file(), "run make build first");

    let scratch =
        std::env::temp_dir().join(format!("quire-code-measurement-{}", std::process::id()));
    if scratch.exists() {
        fs::remove_dir_all(&scratch).expect("remove stale scratch");
    }
    let output_dir = scratch.join("producer");
    let intake = scratch.join("intake");
    let reordered_corpus = scratch.join("corpus-reordered");
    reversed_creation_checkout(&corpus, &reordered_corpus);
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
        Command::new(&release_producer)
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
        Command::new(&release_producer)
            .args(["--repo-root"])
            .arg(&repo)
            .args(["--corpus"])
            .arg(&reordered_corpus)
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
    let parsed: serde_json::Value = serde_json::from_str(&first).expect("collection JSON");
    let scorer = &parsed["rawEvidence"]["scorerReport"];
    assert_eq!(
        scorer["scored_cases"].as_u64(),
        scorer["case_digests"]
            .as_object()
            .map(|items| items.len() as u64),
        "complete-census plan forbids a filtered scorer run"
    );
    assert!(scorer["scored_cases"].as_u64().unwrap_or(0) > 100);
    let observation = &parsed["rawEvidence"]["graphQualityObservation"];
    assert_eq!(observation["population"]["state"], "measured");
    assert_eq!(
        observation["raw_scorer_output"]["path"],
        "raw/scorer-report.json"
    );
    assert_eq!(
        observation["raw_scorer_output"]["digest"],
        sha256(
            &fs::read(output_dir.join("raw/scorer-report.json")).expect("retained scorer bytes")
        )
    );
    assert_eq!(observation["producer"]["source_revision"], source_revision);
    assert_eq!(observation["producer"]["scorer_version"], corpus_revision);
    assert_eq!(
        observation["producer"]["parser_grammars"]
            .as_array()
            .map(Vec::len),
        Some(4)
    );
    assert_eq!(
        parsed["verificationStack"]["sources"]["quire-code-rs"]["revision"],
        source_revision
    );
    assert_eq!(
        parsed["verificationStack"]["sources"]["quire-corpus"]["revision"],
        corpus_revision
    );
    for pointer in [
        "/verificationStack/executableDigest",
        "/verificationStack/artifacts/release-extractor",
        "/verificationStack/artifacts/raw-scorer-output",
    ] {
        let digest = parsed
            .pointer(pointer)
            .and_then(serde_json::Value::as_str)
            .unwrap();
        assert!(
            digest.starts_with("sha256:") && digest.len() == 71,
            "invalid retained digest at {pointer}: {digest}"
        );
    }
    let observations = parsed["observations"].as_array().unwrap();
    let measures = observations
        .iter()
        .filter_map(|item| {
            item.pointer("/dimensions/measure")
                .and_then(serde_json::Value::as_str)
        })
        .collect::<BTreeSet<_>>();
    for required in [
        "precision",
        "precision_decision",
        "recall",
        "unresolved",
        "ambiguous",
        "false_positive",
    ] {
        assert!(measures.contains(required), "missing typed {required}");
    }
    let dimensions = observations
        .iter()
        .filter_map(|item| {
            item.pointer("/dimensions/dimension")
                .and_then(serde_json::Value::as_str)
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        dimensions,
        BTreeSet::from([
            "overall",
            "language",
            "node_kind",
            "relation_kind",
            "resolver_tier"
        ])
    );
    assert!(observations.iter().all(|item| {
        item.pointer("/dimensions/population_state") == Some(&serde_json::json!("measured"))
    }));

    let collection = scratch.join("collection.json");
    fs::write(&collection, first).expect("write collection");
    let retained_path = output(
        Command::new(&quoin)
            .args(["measurement", "record", "--repo"])
            .arg(&intake)
            .args(["--input"])
            .arg(&collection),
    );
    let retained: serde_json::Value =
        serde_json::from_slice(&fs::read(retained_path.trim()).expect("Quoin retained collection"))
            .expect("retained collection JSON");
    assert_eq!(
        retained, parsed,
        "Quoin must retain the complete collection"
    );

    let report = output(Command::new(&quoin).args(["report", "--repo"]).arg(&intake));
    assert!(report.contains("graph_quality"));
    for visible in [
        "measure=precision",
        "measure=precision_decision",
        "measure=recall",
        "measure=unresolved",
        "measure=ambiguous",
        "measure=false_positive",
        "population_state=measured",
        "dimension=overall",
        "dimension=language",
        "dimension=node_kind",
        "dimension=relation_kind",
        "dimension=resolver_tier",
    ] {
        assert!(report.contains(visible), "report omitted {visible}");
    }
    let report_json: serde_json::Value = serde_json::from_str(&output(
        Command::new(&quoin)
            .args(["report", "--format", "json", "--repo"])
            .arg(&intake),
    ))
    .expect("Quoin report JSON");
    let current = report_json["current"].as_array().unwrap();
    assert!(current.iter().any(|row| {
        row.pointer("/collection/toolIdentity")
            == Some(&serde_json::json!(
                "agent-ix/quire-code-rs/measure_graph_quality"
            ))
            && row.pointer("/collection/sourceRevision")
                == Some(&serde_json::json!(source_revision))
            && row.pointer("/observation/dimensions/measure")
                == Some(&serde_json::json!("precision_decision"))
    }));

    fs::remove_dir_all(&scratch).expect("remove scratch");
}
