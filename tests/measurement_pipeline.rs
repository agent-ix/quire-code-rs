//! Real ecosystem lane for the governed measurement producer.
//!
//! Run after a clean release build with:
//! `QUIRE_CORPUS_ROOT=../quire-corpus QUIRE_CORPUS_BIN=<built quire-corpus> QUOIN_BIN=quoin QUIRE_BIN=quire cargo test --test measurement_pipeline -- --ignored --nocapture`.

use ix_trace_rs::trace;
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
        ("--scorer", "quire-corpus"),
        ("--output-dir", "target/measurement"),
        ("--timestamp", "2026-08-31T00:00:00Z"),
        ("--node-version", "24.15.0"),
        ("--rust-version", "1.95.0"),
        ("--source-remote", "local-source"),
        ("--corpus-remote", "local-corpus"),
    ]
    .into_iter()
    .flat_map(|(flag, value)| [flag.to_string(), value.to_string()])
    .collect()
}

// every missing identity fails the real CLI process
// before stdout can contain an observation.
#[trace("TC-123", "FR-012-AC-6")]
#[test]
fn missing_identity_exits_nonzero_with_empty_stdout_and_exact_field() {
    let complete = complete_cli_args();
    for missing in [
        "--timestamp",
        "--node-version",
        "--rust-version",
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

// real extractor + scorer + Quire + Quoin.
#[trace(
    "TC-108",
    "TC-109",
    "TC-110",
    "TC-111",
    "TC-118",
    "TC-119",
    "TC-124",
    "TC-125",
    "TC-127",
    "TC-129",
    "FR-012-AC-1",
    "FR-012-AC-2",
    "FR-012-AC-7",
    "FR-012-AC-8",
    "FR-012-AC-9",
    "FR-012-AC-10",
    "FR-012-AC-11",
    "NFR-005-AC-1",
    "NFR-005-AC-2"
)]
#[test]
#[ignore = "requires a clean quire-corpus checkout and built quire-corpus, installed Quire and Quoin CLIs (QUIRE_CORPUS_ROOT, QUIRE_CORPUS_BIN, QUOIN_BIN, QUIRE_BIN)"]
fn real_corpus_observation_is_retained_and_rendered_by_quoin() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let corpus = PathBuf::from(std::env::var("QUIRE_CORPUS_ROOT").expect("QUIRE_CORPUS_ROOT"));
    let quoin = std::env::var("QUOIN_BIN").expect("QUOIN_BIN");
    let quire = std::env::var("QUIRE_BIN").expect("QUIRE_BIN");
    let scorer = std::env::var("QUIRE_CORPUS_BIN").expect("QUIRE_CORPUS_BIN");
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
    // The scorer must be the checkout's own release build, so the reordered
    // checkout carries a copy of the same binary.
    let reordered_scorer = reordered_corpus.join("target/release/quire-corpus");
    fs::create_dir_all(reordered_scorer.parent().unwrap()).expect("create scorer dir");
    fs::copy(&scorer, &reordered_scorer).expect("copy scorer");
    let reordered_scorer = reordered_scorer.display().to_string();
    fs::create_dir_all(intake.join("spec/assurance")).expect("create intake");
    fs::copy(
        repo.join("spec/assurance/MP-001-graph-quality-observation.md"),
        intake.join("spec/assurance/MP-001-graph-quality-observation.md"),
    )
    .expect("copy plan");
    let plan_path = "spec/assurance/MP-001-graph-quality-observation.md";
    let apparatus = quire_code_rs::measurement::protected_apparatus(
        &fs::read_to_string(repo.join(plan_path)).expect("read plan"),
    )
    .expect("plan apparatus");
    for path in &apparatus {
        let target = intake.join(path);
        fs::create_dir_all(target.parent().expect("apparatus parent")).expect("create parent");
        fs::copy(repo.join(path), target).expect("copy apparatus");
    }

    let first = output(
        Command::new(&release_producer)
            .args(["--repo-root"])
            .arg(&repo)
            .args(["--corpus"])
            .arg(&corpus)
            .args(["--extractor"])
            .arg(&release_extractor)
            .args(["--quire", &quire, "--scorer", &scorer])
            .args(["--output-dir"])
            .arg(&output_dir)
            .args([
                "--timestamp",
                "2026-08-31T00:00:00.000Z",
                "--node-version",
                "24.15.0",
                "--rust-version",
                "1.95.0",
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
            .args(["--quire", &quire, "--scorer", &reordered_scorer])
            .args(["--output-dir"])
            .arg(output_dir.with_file_name("producer-second"))
            .args([
                "--timestamp",
                "2026-08-31T00:00:00.000Z",
                "--node-version",
                "24.15.0",
                "--rust-version",
                "1.95.0",
                "--source-remote",
                &remote(&repo),
                "--corpus-remote",
                &remote(&corpus),
            ]),
    );
    assert_eq!(first, second, "repetitions are byte-identical");
    let parsed: serde_json::Value = serde_json::from_str(&first).expect("collection JSON");
    let scorer = &parsed["rawEvidence"]["scorerReport"];
    assert!(scorer["scored_cases"].as_u64().unwrap_or(0) > 100);
    let observation = &parsed["rawEvidence"]["graphQualityObservation"];
    assert_eq!(observation["population"]["state"], "measured");
    assert_eq!(
        observation["raw_scorer_output"]["path"],
        "raw/scorer-report.json"
    );
    assert_eq!(
        parsed.pointer("/verificationStack/artifacts"),
        Some(&serde_json::json!(apparatus))
    );
    let observations = parsed["observations"].as_array().unwrap();
    assert_eq!(
        observations.len(),
        1,
        "one graph_quality observation per run"
    );
    assert_eq!(
        observations[0].pointer("/dimensions/measure"),
        Some(&serde_json::json!("edge_false_positive"))
    );
    assert_eq!(
        observations[0]["value"],
        observations[0]["population"]["matched"]
    );
    assert_eq!(
        observations[0]["value"],
        scorer["confusion"]["axis_kind"]["edge"]["fp"]
    );

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
    for pointer in [
        "/collectionId",
        "/toolIdentity",
        "/toolVersion",
        "/verificationStack/artifacts",
        "/rawEvidence/graphQualityObservation/observation_id",
        "/rawEvidence/graphQualityObservation/raw_scorer_output/path",
        "/rawEvidence/scorerReport/scored_cases",
    ] {
        assert_eq!(
            retained.pointer(pointer),
            parsed.pointer(pointer),
            "Quoin changed retained field {pointer}"
        );
    }
    assert_eq!(
        retained["observations"].as_array().map(Vec::len),
        parsed["observations"].as_array().map(Vec::len),
        "Quoin changed the typed observation population"
    );

    let report = output(Command::new(&quoin).args(["report", "--repo"]).arg(&intake));
    assert!(report.contains("graph_quality"));
    for visible in [
        "measure=edge_false_positive",
        "population_state=measured",
        "dimension=overall",
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
            && row.pointer("/observation/dimensions/measure")
                == Some(&serde_json::json!("edge_false_positive"))
    }));

    fs::remove_dir_all(&scratch).expect("remove scratch");
}
