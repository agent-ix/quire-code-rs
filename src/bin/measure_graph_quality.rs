//! Run the governed graph-quality measurement pipeline on local pinned inputs.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use quire_code_rs::measurement::{
    build_observation, build_quoin_collection, canonical_bytes, precision_decision_passed, sha256,
    CollectionInputs, GrammarRevision, Population, PopulationState, Provenance, DEFINITION_VERSION,
    METRIC, PLAN_ID, SCHEMA,
};
use serde_json::{json, Value};

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(result) => {
            use std::io::Write;
            if let Err(error) = std::io::stdout().lock().write_all(&result.stdout) {
                eprintln!("stdout: {error}");
                return ExitCode::FAILURE;
            }
            if result.decision_passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

struct RunResult {
    stdout: Vec<u8>,
    decision_passed: bool,
}

fn run(args: Vec<String>) -> Result<RunResult, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("could not identify measurement producer: {error}"))?;
    run_with_producer(args, &executable)
}

fn run_with_producer(args: Vec<String>, producer_executable: &Path) -> Result<RunResult, String> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        eprintln!("{}", usage());
        return Ok(RunResult {
            stdout: Vec::new(),
            decision_passed: true,
        });
    }
    let parsed = Args::parse(args)?;
    validate_pins(&parsed)?;
    validate_grammar_pins(&parsed.repo_root, &parsed.grammars)?;
    validate_release_extractor(&parsed.repo_root, &parsed.extractor)?;
    validate_release_producer(&parsed.repo_root, producer_executable)?;
    verify_clean_source(&parsed.repo_root, &parsed.source_revision, "quire-code-rs")?;
    verify_clean_source(
        &parsed.corpus,
        &parsed.corpus_source_revision,
        "quire-corpus",
    )?;
    validate_plan(&parsed.quire, &parsed.repo_root)?;

    let population = inspect_population(&parsed.corpus)?;
    let raw_relative = "raw/scorer-report.json";
    let raw_path = parsed.output_dir.join(raw_relative);
    fs::create_dir_all(raw_path.parent().expect("raw path has parent"))
        .map_err(|error| format!("{}: {error}", raw_path.display()))?;

    let (report, raw_bytes) = if population.state == PopulationState::Measured {
        score(&parsed)?
    } else {
        let reason = serde_json::to_vec(&json!({
            "state": population.state,
            "files_seen": population.files_seen,
            "supported_files": population.supported_files,
            "unreadable_files": population.unreadable_files,
            "unsupported_files": population.unsupported_files
        }))
        .map_err(|error| error.to_string())?;
        (None, reason)
    };
    fs::write(&raw_path, &raw_bytes).map_err(|error| format!("{}: {error}", raw_path.display()))?;
    let raw_digest = sha256(&raw_bytes);

    let corpus_revision = report
        .as_ref()
        .and_then(|value| value.get("corpus_revision"))
        .and_then(Value::as_str)
        .map(normalize_digest)
        .unwrap_or_else(|| sha256(&[]));
    let provenance = Provenance {
        extractor_revision: parsed.source_revision.clone(),
        source_revision: parsed.source_revision.clone(),
        corpus_revision,
        scorer_version: parsed.scorer_revision.clone(),
        configuration_digest: parsed.config_digest.clone(),
        parser_grammars: parsed.grammars.clone(),
    };
    let observation = build_observation(
        report.as_ref(),
        &population,
        &provenance,
        raw_relative,
        &raw_digest,
    )
    .map_err(|error| error.to_string())?;
    let collection_inputs = CollectionInputs {
        timestamp: parsed.timestamp,
        lock_digest: sha256(
            &fs::read(parsed.repo_root.join("Cargo.lock"))
                .map_err(|error| format!("Cargo.lock: {error}"))?,
        ),
        producer_executable_digest: sha256(
            &fs::read(producer_executable)
                .map_err(|error| format!("{}: {error}", producer_executable.display()))?,
        ),
        extractor_executable_digest: sha256(
            &fs::read(&parsed.extractor)
                .map_err(|error| format!("{}: {error}", parsed.extractor.display()))?,
        ),
        schema_digest: sha256(SCHEMA.as_bytes()),
        plan_digest: sha256(
            &fs::read(
                parsed
                    .repo_root
                    .join("spec/assurance/MP-001-graph-quality-observation.md"),
            )
            .map_err(|error| format!("MP-001: {error}"))?,
        ),
        node_version: parsed.node_version,
        rust_version: parsed.rust_version,
        python_version: parsed.python_version,
        source_remote: parsed.source_remote,
        corpus_source_revision: parsed.corpus_source_revision,
        corpus_remote: parsed.corpus_remote,
    };
    let collection = build_quoin_collection(
        &observation,
        report.as_ref(),
        &provenance,
        &collection_inputs,
    )
    .map_err(|error| error.to_string())?;
    let mut stdout = canonical_bytes(&collection).map_err(|error| error.to_string())?;
    stdout.push(b'\n');
    let decision_passed = report
        .as_ref()
        .map(precision_decision_passed)
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or(false);
    Ok(RunResult {
        stdout,
        decision_passed,
    })
}

fn score(args: &Args) -> Result<(Option<Value>, Vec<u8>), String> {
    let producer = format!(
        "{} --org '{{org}}' --repo '{{repo}}' '{{input}}'",
        shell_quote(&args.extractor.to_string_lossy())
    );
    let mut command = Command::new(&args.python);
    command
        .arg(args.corpus.join("score.py"))
        .arg("--producer")
        .arg(producer)
        .arg("--json");
    command.current_dir(&args.corpus);
    let output = command
        .output()
        .map_err(|error| format!("could not run corpus scorer: {error}"))?;
    if output.stdout.is_empty() {
        return Err(format!(
            "corpus scorer emitted no JSON: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let report: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        format!(
            "corpus scorer output is not JSON: {error}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })?;
    validate_complete_report(&report)?;
    Ok((Some(report), output.stdout))
}

fn validate_complete_report(report: &Value) -> Result<(), String> {
    let scored = report
        .get("scored_cases")
        .and_then(Value::as_u64)
        .ok_or_else(|| "scorer report has no scored_cases".to_string())?;
    let declared = report
        .get("case_digests")
        .and_then(Value::as_object)
        .ok_or_else(|| "scorer report has no case_digests".to_string())?
        .len() as u64;
    if scored != declared {
        return Err(format!(
            "scorer report is a partial population: scored {scored} of {declared} declared cases"
        ));
    }
    Ok(())
}

fn validate_plan(quire: &Path, repo: &Path) -> Result<(), String> {
    let path = repo.join("spec/assurance/MP-001-graph-quality-observation.md");
    let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    validate_plan_semantics(&text)?;
    let output = Command::new(quire)
        .arg("validate")
        .arg("--scope")
        .arg(repo)
        .arg("spec/assurance/MP-001-graph-quality-observation.md")
        .output()
        .map_err(|error| format!("could not run Quire: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "MP-001 failed Quire validation: {}",
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn validate_plan_semantics(text: &str) -> Result<(), String> {
    let frontmatter = text
        .strip_prefix("---\n")
        .and_then(|rest| {
            rest.split_once("\n---\n")
                .map(|(frontmatter, _)| frontmatter)
        })
        .ok_or_else(|| "MP-001 has no YAML frontmatter".to_string())?;
    for expected in [
        format!("id: {PLAN_ID}"),
        "type: MeasurementPlan".into(),
        "status: active".into(),
        format!("metric: {METRIC}"),
        format!("definition_version: {DEFINITION_VERSION}"),
        "  sampling: complete census with no sampling; preserve language, node-kind, relation-kind, and resolver-tier strata".into(),
        "  decision_rule: reject a measured run with any wrong heuristic edge; report recall independently; treat non-measured states as no decision".into(),
    ] {
        if !frontmatter.lines().any(|line| line == expected) {
            return Err(format!("MP-001 semantic mismatch: expected `{expected}`"));
        }
    }
    Ok(())
}

fn verify_clean_source(root: &Path, expected: &str, name: &str) -> Result<(), String> {
    let head = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| format!("could not inspect {name} revision: {error}"))?;
    let actual = String::from_utf8_lossy(&head.stdout).trim().to_string();
    if !head.status.success() || actual != expected {
        return Err(format!(
            "{name} revision mismatch: expected {expected}, found {actual}"
        ));
    }
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain"])
        .output()
        .map_err(|error| format!("could not inspect {name} source state: {error}"))?;
    if !status.status.success() || !status.stdout.is_empty() {
        return Err(format!("{name} source state is not clean"));
    }
    Ok(())
}

fn inspect_population(corpus: &Path) -> Result<Population, String> {
    let roots = vec![corpus.join("fixtures")];
    let mut files_seen = 0;
    let mut supported_files = 0;
    let mut unreadable_files = 0;
    let mut unsupported_files = 0;
    let mut pending = roots;
    pending.sort();
    while let Some(path) = pending.pop() {
        let entries =
            fs::read_dir(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let mut entries = entries
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        entries.sort_by_key(|entry| entry.path());
        for entry in entries.into_iter().rev() {
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| format!("{}: {error}", path.display()))?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }
            if !inside_source_tree(&path) {
                continue;
            }
            files_seen += 1;
            if supported(&path) {
                supported_files += 1;
                if fs::read_to_string(&path).is_err() {
                    unreadable_files += 1;
                }
            } else {
                unsupported_files += 1;
            }
        }
    }
    let state = if files_seen == 0 {
        PopulationState::Empty
    } else if unreadable_files > 0 {
        PopulationState::Unreadable
    } else if supported_files == 0 {
        PopulationState::Unsupported
    } else {
        PopulationState::Measured
    };
    Ok(Population {
        state,
        files_seen,
        supported_files,
        unreadable_files,
        unsupported_files,
    })
}

fn inside_source_tree(path: &Path) -> bool {
    path.ancestors().any(|parent| {
        parent
            .file_name()
            .is_some_and(|name| name == "input" || name == "variant")
    })
}

fn supported(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("rs" | "ts" | "tsx" | "py")
    )
}

fn normalize_digest(value: &str) -> String {
    if value.starts_with("sha256:") {
        value.to_string()
    } else {
        format!("sha256:{value}")
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn validate_pins(args: &Args) -> Result<(), String> {
    for (name, value) in [
        ("source-revision", &args.source_revision),
        ("corpus-source-revision", &args.corpus_source_revision),
        ("scorer-revision", &args.scorer_revision),
    ] {
        if value.len() != 40
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(format!("--{name} must be a full lowercase 40-hex revision"));
        }
    }
    if args.scorer_revision != args.corpus_source_revision {
        return Err(
            "--scorer-revision must equal --corpus-source-revision because score.py is loaded from that checkout"
                .into(),
        );
    }
    if !valid_digest(&args.config_digest) {
        return Err("--config-digest must be sha256 plus 64 lowercase hex digits".into());
    }
    if args.grammars.is_empty() {
        return Err("missing --grammar".into());
    }
    for grammar in &args.grammars {
        if !matches!(
            grammar.language.as_str(),
            "rust" | "typescript" | "tsx" | "python"
        ) {
            return Err(format!("unknown grammar language {}", grammar.language));
        }
    }
    for (name, value) in [
        ("timestamp", &args.timestamp),
        ("node-version", &args.node_version),
        ("rust-version", &args.rust_version),
        ("python-version", &args.python_version),
        ("source-remote", &args.source_remote),
        ("corpus-remote", &args.corpus_remote),
    ] {
        if value.is_empty() {
            return Err(format!("missing --{name}"));
        }
    }
    Ok(())
}

fn validate_release_extractor(repo: &Path, extractor: &Path) -> Result<(), String> {
    let expected_extractor = repo.join("target/release/extract_tree");
    let actual =
        fs::canonicalize(extractor).map_err(|error| format!("{}: {error}", extractor.display()))?;
    let expected = fs::canonicalize(&expected_extractor)
        .map_err(|error| format!("{}: {error}", expected_extractor.display()))?;
    if actual != expected {
        return Err(format!(
            "--extractor must be the release binary {}",
            expected.display()
        ));
    }
    Ok(())
}

fn validate_release_producer(repo: &Path, producer: &Path) -> Result<(), String> {
    let expected_producer = repo.join("target/release/measure_graph_quality");
    let actual =
        fs::canonicalize(producer).map_err(|error| format!("{}: {error}", producer.display()))?;
    let expected = fs::canonicalize(&expected_producer)
        .map_err(|error| format!("{}: {error}", expected_producer.display()))?;
    if actual != expected {
        return Err(format!(
            "measurement producer must be the release binary {}",
            expected.display()
        ));
    }
    Ok(())
}

fn validate_grammar_pins(repo: &Path, supplied: &[GrammarRevision]) -> Result<(), String> {
    let lock_path = repo.join("Cargo.lock");
    let lock = fs::read_to_string(&lock_path)
        .map_err(|error| format!("{}: {error}", lock_path.display()))?;
    let expected = BTreeMap::from([
        (
            "python".to_string(),
            (
                "tree-sitter-python".to_string(),
                locked_version(&lock, "tree-sitter-python")?,
            ),
        ),
        (
            "rust".to_string(),
            (
                "tree-sitter-rust".to_string(),
                locked_version(&lock, "tree-sitter-rust")?,
            ),
        ),
        (
            "tsx".to_string(),
            (
                "tree-sitter-typescript".to_string(),
                locked_version(&lock, "tree-sitter-typescript")?,
            ),
        ),
        (
            "typescript".to_string(),
            (
                "tree-sitter-typescript".to_string(),
                locked_version(&lock, "tree-sitter-typescript")?,
            ),
        ),
    ]);
    let mut actual = BTreeMap::new();
    for grammar in supplied {
        if actual
            .insert(
                grammar.language.clone(),
                (grammar.grammar.clone(), grammar.revision.clone()),
            )
            .is_some()
        {
            return Err(format!("duplicate --grammar for {}", grammar.language));
        }
    }
    if actual != expected {
        return Err(format!(
            "--grammar pins must exactly match Cargo.lock: expected {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn locked_version(lock: &str, package: &str) -> Result<String, String> {
    for block in lock.split("[[package]]").skip(1) {
        let mut name = None;
        let mut version = None;
        for line in block.lines() {
            if let Some(value) = line
                .strip_prefix("name = \"")
                .and_then(|v| v.strip_suffix('"'))
            {
                name = Some(value);
            }
            if let Some(value) = line
                .strip_prefix("version = \"")
                .and_then(|v| v.strip_suffix('"'))
            {
                version = Some(value);
            }
        }
        if name == Some(package) {
            return version
                .map(str::to_string)
                .ok_or_else(|| format!("Cargo.lock package {package} has no version"));
        }
    }
    Err(format!("Cargo.lock has no package {package}"))
}

fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

struct Args {
    repo_root: PathBuf,
    corpus: PathBuf,
    extractor: PathBuf,
    quire: PathBuf,
    python: PathBuf,
    output_dir: PathBuf,
    timestamp: String,
    source_revision: String,
    corpus_source_revision: String,
    scorer_revision: String,
    config_digest: String,
    grammars: Vec<GrammarRevision>,
    node_version: String,
    rust_version: String,
    python_version: String,
    source_remote: String,
    corpus_remote: String,
}

impl Args {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        let mut values = BTreeMap::<String, Vec<String>>::new();
        let mut iter = args.into_iter();
        while let Some(flag) = iter.next() {
            if !flag.starts_with("--") {
                return Err(format!("unexpected argument {flag}\n{}", usage()));
            }
            let value = iter
                .next()
                .ok_or_else(|| format!("{flag} requires a value"))?;
            if !matches!(
                flag.as_str(),
                "--repo-root"
                    | "--corpus"
                    | "--extractor"
                    | "--quire"
                    | "--python"
                    | "--output-dir"
                    | "--timestamp"
                    | "--source-revision"
                    | "--corpus-source-revision"
                    | "--scorer-revision"
                    | "--config-digest"
                    | "--grammar"
                    | "--node-version"
                    | "--rust-version"
                    | "--python-version"
                    | "--source-remote"
                    | "--corpus-remote"
            ) {
                return Err(format!("unknown option {flag}"));
            }
            if flag != "--grammar" && values.contains_key(&flag) {
                return Err(format!("duplicate option {flag}"));
            }
            values.entry(flag).or_default().push(value);
        }
        let one = |flag: &str| -> Result<String, String> {
            values
                .get(flag)
                .and_then(|v| v.last())
                .cloned()
                .ok_or_else(|| format!("missing {flag}"))
        };
        let grammars = values
            .get("--grammar")
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(parse_grammar)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            repo_root: PathBuf::from(one("--repo-root")?),
            corpus: PathBuf::from(one("--corpus")?),
            extractor: PathBuf::from(one("--extractor")?),
            quire: PathBuf::from(one("--quire")?),
            python: PathBuf::from(one("--python")?),
            output_dir: PathBuf::from(one("--output-dir")?),
            timestamp: one("--timestamp")?,
            source_revision: one("--source-revision")?,
            corpus_source_revision: one("--corpus-source-revision")?,
            scorer_revision: one("--scorer-revision")?,
            config_digest: one("--config-digest")?,
            grammars,
            node_version: one("--node-version")?,
            rust_version: one("--rust-version")?,
            python_version: one("--python-version")?,
            source_remote: one("--source-remote")?,
            corpus_remote: one("--corpus-remote")?,
        })
    }
}

fn parse_grammar(value: String) -> Result<GrammarRevision, String> {
    let (language, rest) = value
        .split_once('=')
        .ok_or_else(|| format!("invalid --grammar {value}; expected language=grammar@revision"))?;
    let (grammar, revision) = rest
        .rsplit_once('@')
        .ok_or_else(|| format!("invalid --grammar {value}; expected language=grammar@revision"))?;
    if language.is_empty() || grammar.is_empty() || revision.is_empty() {
        return Err(format!("invalid --grammar {value}"));
    }
    Ok(GrammarRevision {
        language: language.into(),
        grammar: grammar.into(),
        revision: revision.into(),
    })
}

fn usage() -> &'static str {
    "usage: measure_graph_quality --repo-root DIR --corpus DIR --extractor FILE --quire FILE --python FILE --output-dir DIR --timestamp RFC3339 --source-revision SHA --corpus-source-revision SHA --scorer-revision SHA --config-digest sha256:HEX --grammar language=crate@revision [--grammar ...] --node-version VERSION --rust-version VERSION --python-version VERSION --source-remote URL --corpus-remote URL"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_args() -> Vec<String> {
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

    fn without_flag(args: &[String], missing: &str) -> Vec<String> {
        args.chunks_exact(2)
            .filter(|pair| pair[0] != missing)
            .flat_map(|pair| pair.iter().cloned())
            .collect()
    }

    fn git(root: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .expect("git starts");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    fn commit_repo(root: &Path) -> String {
        git(root, &["init", "-q"]);
        git(
            root,
            &["config", "user.email", "measurement@example.invalid"],
        );
        git(root, &["config", "user.name", "Measurement Test"]);
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "fixture"]);
        git(root, &["rev-parse", "HEAD"])
    }

    fn non_measured_args(repo: &Path, corpus: &Path, output: &Path) -> Vec<String> {
        let source_revision = git(repo, &["rev-parse", "HEAD"]);
        let corpus_revision = git(corpus, &["rev-parse", "HEAD"]);
        let replacements = [
            ("--repo-root", repo.display().to_string()),
            ("--corpus", corpus.display().to_string()),
            (
                "--extractor",
                repo.join("target/release/extract_tree")
                    .display()
                    .to_string(),
            ),
            ("--quire", "/usr/bin/true".into()),
            ("--python", "/usr/bin/true".into()),
            ("--output-dir", output.display().to_string()),
            ("--source-revision", source_revision),
            ("--corpus-source-revision", corpus_revision.clone()),
            ("--scorer-revision", corpus_revision),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>();
        let mut args = valid_args();
        for pair in args.chunks_exact_mut(2) {
            if let Some(value) = replacements.get(pair[0].as_str()) {
                pair[1] = value.clone();
            }
        }
        args
    }

    fn make_source_repo(root: &Path) {
        fs::create_dir_all(root.join("spec/assurance")).unwrap();
        fs::create_dir_all(root.join("target/release")).unwrap();
        fs::write(root.join("Cargo.lock"), include_str!("../../Cargo.lock")).unwrap();
        fs::write(
            root.join("spec/assurance/MP-001-graph-quality-observation.md"),
            include_str!("../../spec/assurance/MP-001-graph-quality-observation.md"),
        )
        .unwrap();
        fs::write(root.join("target/release/extract_tree"), b"release").unwrap();
        fs::write(
            root.join("target/release/measure_graph_quality"),
            b"release producer",
        )
        .unwrap();
        commit_repo(root);
    }

    fn scorer_report(edge_false_positives: u64) -> Value {
        json!({
            "schema_version": 1,
            "corpus_revision": format!("sha256:{}", "c".repeat(64)),
            "case_digests": {"relations/ambiguous/rust": "digest"},
            "scored_cases": 1,
            "confusion": {
                "total": {"total": {"tp": 8, "fp": edge_false_positives + 1, "fn": 2}},
                "language": {"rust": {"tp": 8, "fp": edge_false_positives + 1, "fn": 2}},
                "object_type": {"code_function": {"tp": 4, "fp": 1, "fn": 1}},
                "relation": {"calls": {"tp": 3, "fp": edge_false_positives, "fn": 1}},
                "tier": {"receiver-typed": {"tp": 3, "fp": edge_false_positives, "fn": 1}},
                "axis_kind": {
                    "edge": {"tp": 3, "fp": edge_false_positives, "fn": 1},
                    "node": {"tp": 5, "fp": 1, "fn": 1}
                }
            },
            "cases": {
                "relations/ambiguous/rust": {
                    "census": {"ambiguous_call_sites": {"reported": 1, "expected": 1}}
                }
            }
        })
    }

    fn make_measured_corpus(root: &Path, report: &Value, scorer_exit: i32) {
        fs::create_dir_all(root.join("fixtures/relations/ambiguous/rust/input")).unwrap();
        fs::write(
            root.join("fixtures/relations/ambiguous/rust/input/lib.rs"),
            b"fn measured() {}\n",
        )
        .unwrap();
        let payload = serde_json::to_string(report).unwrap();
        assert!(!payload.contains('\''));
        fs::write(
            root.join("score.py"),
            format!("printf '%s\\n' '{payload}'\nexit {scorer_exit}\n"),
        )
        .unwrap();
        commit_repo(root);
    }

    // TC-123 / FR-012-AC-6: missing pins name the exact input and emit nothing.
    #[test]
    fn parser_names_every_missing_identity_before_any_output_exists() {
        let args = valid_args();
        for missing in [
            "--source-revision",
            "--corpus-source-revision",
            "--scorer-revision",
            "--config-digest",
            "--timestamp",
            "--node-version",
            "--rust-version",
            "--python-version",
            "--source-remote",
            "--corpus-remote",
        ] {
            let error = Args::parse(without_flag(&args, missing)).err().unwrap();
            assert!(error.contains(missing), "{missing}: {error}");
        }

        let parsed = Args::parse(without_flag(&args, "--grammar")).unwrap();
        assert_eq!(validate_pins(&parsed).unwrap_err(), "missing --grammar");
        assert!(
            run(Vec::new()).is_err(),
            "failed runs produce no stdout payload"
        );
    }

    // TC-133, TC-134 / FR-012-CON-2..3: the producer accepts only paths/values and has no URL client.
    #[test]
    fn grammar_parser_is_closed_and_requires_a_pinned_revision() {
        assert!(parse_grammar("rust=tree-sitter-rust@0.24.2".into()).is_ok());
        assert!(parse_grammar("rust=tree-sitter-rust".into()).is_err());
        let parsed = Args::parse(valid_args()).unwrap();
        validate_grammar_pins(Path::new(env!("CARGO_MANIFEST_DIR")), &parsed.grammars).unwrap();
        let mut bad = parsed.grammars;
        bad[0].revision = "0.0.0".into();
        assert!(validate_grammar_pins(Path::new(env!("CARGO_MANIFEST_DIR")), &bad).is_err());
    }

    // TC-125 / FR-012-AC-8: structural Quire validity cannot mask semantic drift.
    #[test]
    fn measurement_plan_must_remain_active_and_match_the_governed_decision() {
        let good = include_str!("../../spec/assurance/MP-001-graph-quality-observation.md");
        validate_plan_semantics(good).unwrap();
        for (from, to) in [
            ("status: active", "status: retired"),
            ("metric: graph_quality", "metric: other"),
            (
                "definition_version: quire-code.graph-quality-v1",
                "definition_version: drifted",
            ),
            (
                "decision_rule: reject a measured run with any wrong heuristic edge",
                "decision_rule: accept all measured runs",
            ),
        ] {
            let bad = good.replace(from, to);
            assert!(validate_plan_semantics(&bad).is_err(), "accepted {to}");
        }
    }

    #[test]
    fn scorer_report_must_cover_every_declared_case() {
        let complete = json!({"scored_cases": 2, "case_digests": {"a": "x", "b": "y"}});
        validate_complete_report(&complete).unwrap();
        let partial = json!({"scored_cases": 1, "case_digests": {"a": "x", "b": "y"}});
        assert!(validate_complete_report(&partial).is_err());
    }

    // TC-122 / FR-012-AC-5: all three absence states emit evidence but no
    // measured result and map to a non-zero main-process decision.
    #[test]
    fn binary_pipeline_emits_each_non_measured_state_without_results() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let scratch = std::env::temp_dir().join(format!(
            "quire-code-non-measured-{}-{unique}",
            std::process::id()
        ));
        let repo = scratch.join("source");
        fs::create_dir_all(&repo).unwrap();
        make_source_repo(&repo);

        for (state, relative, bytes) in [
            ("empty", ".keep", b"".as_slice()),
            ("unsupported", "case/input/readme.txt", b"text".as_slice()),
            ("unreadable", "case/input/broken.rs", &[0xff, 0xfe][..]),
        ] {
            let corpus = scratch.join(format!("corpus-{state}"));
            let fixture = corpus.join("fixtures").join(relative);
            fs::create_dir_all(fixture.parent().unwrap()).unwrap();
            fs::write(&fixture, bytes).unwrap();
            commit_repo(&corpus);
            let result = run_with_producer(
                non_measured_args(&repo, &corpus, &scratch.join(format!("out-{state}"))),
                &repo.join("target/release/measure_graph_quality"),
            )
            .unwrap();
            assert!(!result.decision_passed, "{state} must exit non-zero");
            let collection: Value = serde_json::from_slice(&result.stdout).unwrap();
            let observation = &collection["rawEvidence"]["graphQualityObservation"];
            assert_eq!(observation["population"]["state"], state);
            assert!(observation.get("results").is_none());
        }

        fs::remove_dir_all(scratch).unwrap();
    }

    // TC-120, TC-121 / FR-012-AC-3..4: child-process status, recall, and
    // non-edge findings do not control the decision; a heuristic-edge FP does.
    #[test]
    fn binary_pipeline_gates_only_on_false_positive_heuristic_edges() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let scratch = std::env::temp_dir().join(format!(
            "quire-code-precision-gate-{}-{unique}",
            std::process::id()
        ));
        let repo = scratch.join("source");
        fs::create_dir_all(&repo).unwrap();
        make_source_repo(&repo);

        for (name, edge_fp, scorer_exit, expected) in
            [("recall-only", 0, 1, true), ("wrong-edge", 1, 0, false)]
        {
            let corpus = scratch.join(format!("corpus-{name}"));
            make_measured_corpus(&corpus, &scorer_report(edge_fp), scorer_exit);
            let mut args = non_measured_args(&repo, &corpus, &scratch.join(format!("out-{name}")));
            for pair in args.chunks_exact_mut(2) {
                if pair[0] == "--python" {
                    pair[1] = "/bin/sh".into();
                }
            }
            let result =
                run_with_producer(args, &repo.join("target/release/measure_graph_quality"))
                    .unwrap();
            assert_eq!(result.decision_passed, expected, "{name}");
            let collection: Value = serde_json::from_slice(&result.stdout).unwrap();
            assert!(collection["observations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item.pointer("/dimensions/measure") == Some(&json!("recall"))));
            assert_eq!(
                collection["rawEvidence"]["scorerReport"]["confusion"]["axis_kind"]["edge"]["fp"],
                edge_fp
            );
        }

        fs::remove_dir_all(scratch).unwrap();
    }
}
