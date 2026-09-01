//! Run the governed graph-quality measurement pipeline on local pinned inputs.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use quire_code_rs::measurement::{
    build_observation, build_quoin_collection, canonical_bytes, sha256, CollectionInputs,
    GrammarRevision, Population, PopulationState, Provenance, SCHEMA,
};
use serde_json::{json, Value};

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(decision_passed) => {
            if decision_passed {
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

fn run(args: Vec<String>) -> Result<bool, String> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        eprintln!("{}", usage());
        return Ok(true);
    }
    let parsed = Args::parse(args)?;
    validate_pins(&parsed)?;
    verify_clean_source(&parsed.repo_root, &parsed.source_revision, "quire-code-rs")?;
    verify_clean_source(
        &parsed.corpus,
        &parsed.corpus_source_revision,
        "quire-corpus",
    )?;
    validate_plan(&parsed.quire, &parsed.repo_root)?;

    let population = inspect_population(&parsed.corpus, &parsed.cases)?;
    let raw_relative = "raw/scorer-report.json";
    let raw_path = parsed.output_dir.join(raw_relative);
    fs::create_dir_all(raw_path.parent().expect("raw path has parent"))
        .map_err(|error| format!("{}: {error}", raw_path.display()))?;

    let (report, raw_bytes, scorer_passed) = if population.state == PopulationState::Measured {
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
        (None, reason, false)
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
        executable_digest: sha256(
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
    use std::io::Write;
    std::io::stdout()
        .lock()
        .write_all(&stdout)
        .map_err(|error| format!("stdout: {error}"))?;

    let false_positives = observation
        .pointer("/results/confusion_matrices")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|matrix| matrix.get("false_positive").and_then(Value::as_u64))
        .sum::<u64>();
    Ok(scorer_passed && population.state == PopulationState::Measured && false_positives == 0)
}

fn score(args: &Args) -> Result<(Option<Value>, Vec<u8>, bool), String> {
    let producer = format!(
        "{} --org {{org}} --repo {{repo}} {{input}}",
        shell_quote(&args.extractor.to_string_lossy())
    );
    let mut command = Command::new(&args.python);
    command
        .arg(args.corpus.join("score.py"))
        .arg("--producer")
        .arg(producer)
        .arg("--json");
    for case in &args.cases {
        command.arg("--case").arg(case);
    }
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
    Ok((Some(report), output.stdout, output.status.success()))
}

fn validate_plan(quire: &Path, repo: &Path) -> Result<(), String> {
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

fn inspect_population(corpus: &Path, cases: &[String]) -> Result<Population, String> {
    let roots: Vec<PathBuf> = if cases.is_empty() {
        vec![corpus.join("fixtures")]
    } else {
        cases
            .iter()
            .map(|case| corpus.join("fixtures").join(case))
            .collect()
    };
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
    if !valid_digest(&args.config_digest) {
        return Err("--config-digest must be sha256 plus 64 lowercase hex digits".into());
    }
    if args.grammars.is_empty() {
        return Err("missing --grammar".into());
    }
    for grammar in &args.grammars {
        if !matches!(
            grammar.language.as_str(),
            "rust" | "typescript" | "tsx" | "python" | "mixed"
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
    cases: Vec<String>,
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
            cases: values.get("--case").cloned().unwrap_or_default(),
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
    "usage: measure_graph_quality --repo-root DIR --corpus DIR --extractor FILE --quire FILE --python FILE --output-dir DIR --timestamp RFC3339 --source-revision SHA --corpus-source-revision SHA --scorer-revision SHA --config-digest sha256:HEX --grammar language=crate@revision [--grammar ...] [--case family/case/language ...] --node-version VERSION --rust-version VERSION --python-version VERSION --source-remote URL --corpus-remote URL"
}

#[cfg(test)]
mod tests {
    use super::*;

    // TC-123 / FR-012-AC-6: missing pins name the exact input and emit nothing.
    #[test]
    fn parser_names_the_missing_pin() {
        let error = Args::parse(vec![]).err().unwrap();
        assert!(error.contains("--repo-root"));
    }

    // TC-133, TC-134 / FR-012-CON-2..3: the producer accepts only paths/values and has no URL client.
    #[test]
    fn grammar_parser_is_closed_and_requires_a_pinned_revision() {
        assert!(parse_grammar("rust=tree-sitter-rust@0.24.2".into()).is_ok());
        assert!(parse_grammar("rust=tree-sitter-rust".into()).is_err());
    }
}
