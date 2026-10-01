//! Governed graph-quality observations and Quoin collection mapping.
//!
//! This module is deliberately pure. The binary owns filesystem and child
//! processes; this module owns the versioned record, canonical identity, schema
//! validation, and lossless mapping of the corpus scorer report (FR-011,
//! FR-012).

use std::collections::{BTreeMap, BTreeSet};

use jsonschema::{Draft, JSONSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const SCHEMA: &str = include_str!("../schemas/graph-quality-observation-v1.schema.json");
pub const PLAN_ID: &str = "MP-001";
pub const PLAN_REF: &str = "ix://agent-ix/quire-code-rs/MP-001";
pub const DEFINITION_VERSION: &str = "quire-code.graph-quality-v2";
pub const METRIC: &str = "graph_quality";

type LanguageUnresolvedCounts = BTreeMap<String, (u64, u64)>;
type UnresolvedCounts = (u64, u64, LanguageUnresolvedCounts);

const GRAMMAR_LANGUAGES: [&str; 4] = ["python", "rust", "tsx", "typescript"];
const RESULT_DIMENSIONS: [&str; 5] = [
    "overall",
    "language",
    "node_kind",
    "relation_kind",
    "resolver_tier",
];

#[derive(Debug, Error)]
pub enum MeasurementError {
    #[error("schema is invalid: {0}")]
    InvalidSchema(String),
    #[error("observation failed validation: {0}")]
    InvalidObservation(String),
    #[error("scorer report is invalid: {0}")]
    InvalidScorerReport(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarRevision {
    pub language: String,
    pub grammar: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub extractor_revision: String,
    pub source_revision: String,
    pub corpus_revision: String,
    pub scorer_version: String,
    pub configuration_digest: String,
    pub parser_grammars: Vec<GrammarRevision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PopulationState {
    Measured,
    Empty,
    Unreadable,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Population {
    pub state: PopulationState,
    pub files_seen: u64,
    pub supported_files: u64,
    pub unreadable_files: u64,
    pub unsupported_files: u64,
}

#[derive(Debug, Clone)]
pub struct CollectionInputs {
    pub timestamp: String,
    pub lock_digest: String,
    pub producer_executable_digest: String,
    pub extractor_executable_digest: String,
    pub schema_digest: String,
    pub plan_digest: String,
    pub node_version: String,
    pub rust_version: String,
    pub python_version: String,
    pub source_remote: String,
    pub corpus_source_revision: String,
    pub corpus_remote: String,
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn canonical_bytes(value: &Value) -> Result<Vec<u8>, MeasurementError> {
    serde_json::to_vec(value)
        .map_err(|error| MeasurementError::InvalidObservation(error.to_string()))
}

/// Build one raw graph observation from the exact scorer report.
pub fn build_observation(
    report: Option<&Value>,
    population: &Population,
    provenance: &Provenance,
    raw_path: &str,
    raw_digest: &str,
) -> Result<Value, MeasurementError> {
    let mut grammars = provenance.parser_grammars.clone();
    grammars.sort_by(|a, b| {
        (&a.language, &a.grammar, &a.revision).cmp(&(&b.language, &b.grammar, &b.revision))
    });

    let (census, results) = match population.state {
        PopulationState::Measured => {
            let report = report.ok_or_else(|| {
                MeasurementError::InvalidScorerReport(
                    "measured population requires a scorer report".into(),
                )
            })?;
            let census = census_from(report)?;
            let results = results_from(report)?;
            (census, Some(results))
        }
        _ => (empty_census(), None),
    };

    let mut observation = json!({
        "schema_version": 1,
        "record_type": "graph_quality_observation",
        "observation_id": format!("sha256:{}", "0".repeat(64)),
        "producer": {
            "extractor_revision": provenance.extractor_revision,
            "producer_contract_version": 1,
            "parser_grammars": grammars,
            "configuration_digest": provenance.configuration_digest,
            "source_revision": provenance.source_revision,
            "corpus_revision": provenance.corpus_revision,
            "scorer_version": provenance.scorer_version
        },
        "measurement_plan": {
            "ref": PLAN_REF,
            "definition_version": DEFINITION_VERSION
        },
        "population": {
            "state": population.state,
            "files_seen": population.files_seen,
            "supported_files": population.supported_files,
            "unreadable_files": population.unreadable_files,
            "unsupported_files": population.unsupported_files,
            "census": census
        },
        "raw_scorer_output": { "path": raw_path, "digest": raw_digest }
    });
    if let Some(results) = results {
        observation["results"] = results;
    }
    let id = observation_id(&observation)?;
    observation["observation_id"] = Value::String(id);
    validate_observation(&observation)?;
    Ok(observation)
}

/// Validate both draft-2020-12 shape and invariants requiring whole-record context.
pub fn validate_observation(value: &Value) -> Result<(), MeasurementError> {
    let schema: Value = serde_json::from_str(SCHEMA)
        .map_err(|error| MeasurementError::InvalidSchema(error.to_string()))?;
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema)
        .map_err(|error| MeasurementError::InvalidSchema(error.to_string()))?;
    if let Err(errors) = compiled.validate(value) {
        let detail = errors
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(MeasurementError::InvalidObservation(detail));
    }

    let supplied = value
        .get("observation_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expected = observation_id(value)?;
    if supplied != expected {
        return Err(MeasurementError::InvalidObservation(format!(
            "observation_id mismatch: expected {expected}"
        )));
    }
    let grammar_languages = value
        .pointer("/producer/parser_grammars")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|grammar| grammar.get("language").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    if grammar_languages != BTreeSet::from(GRAMMAR_LANGUAGES) {
        return Err(MeasurementError::InvalidObservation(
            "parser_grammars must pin exactly python, rust, tsx, and typescript".into(),
        ));
    }
    for pointer in [
        "/population/census/languages",
        "/population/census/node_kinds",
        "/population/census/relation_kinds",
        "/population/census/resolver_tiers",
        "/results/confusion_matrices",
        "/results/unresolved",
        "/results/ambiguous",
        "/results/recall",
    ] {
        if let Some(items) = value.pointer(pointer).and_then(Value::as_array) {
            require_sorted_unique(items, pointer)?;
        }
    }
    if value.pointer("/population/state") == Some(&Value::String("measured".into())) {
        for (name, pointer) in [
            ("confusion matrices", "/results/confusion_matrices"),
            ("unresolved counts", "/results/unresolved"),
            ("ambiguous counts", "/results/ambiguous"),
            ("recall", "/results/recall"),
        ] {
            let items = value
                .pointer(pointer)
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    MeasurementError::InvalidObservation(format!("measured record has no {name}"))
                })?;
            let dimensions: BTreeSet<_> = items
                .iter()
                .filter_map(|item| item.get("dimension").and_then(Value::as_str))
                .collect();
            for required in RESULT_DIMENSIONS {
                if !dimensions.contains(required) {
                    return Err(MeasurementError::InvalidObservation(format!(
                        "{name} omit {required}"
                    )));
                }
            }
            for item in items {
                if item.get("dimension").and_then(Value::as_str) == Some("language")
                    && !item.get("key").and_then(Value::as_str).is_some_and(|key| {
                        matches!(key, "rust" | "typescript" | "tsx" | "python" | "mixed")
                    })
                {
                    return Err(MeasurementError::InvalidObservation(format!(
                        "{name} contain an unknown language key"
                    )));
                }
            }
        }
    }
    Ok(())
}

pub fn build_quoin_collection(
    observation: &Value,
    scorer_report: Option<&Value>,
    provenance: &Provenance,
    inputs: &CollectionInputs,
) -> Result<Value, MeasurementError> {
    validate_observation(observation)?;
    let observations = quoin_observations(observation, scorer_report)?;
    let collection_id = observation["observation_id"]
        .as_str()
        .unwrap_or_default()
        .trim_start_matches("sha256:");
    Ok(json!({
        "schemaVersion": 2,
        "collectionId": collection_id,
        "subject": "agent-ix/quire-code-rs graph extraction",
        "scope": observation["population"].clone(),
        "toolIdentity": "agent-ix/quire-code-rs/measure_graph_quality",
        "toolVersion": format!("{} ({})", env!("CARGO_PKG_VERSION"), provenance.extractor_revision),
        "configDigest": provenance.configuration_digest,
        "timestamp": inputs.timestamp,
        "sourceRevision": provenance.source_revision,
        "corpusRevision": provenance.corpus_revision,
        "environment": {
            "node": inputs.node_version,
            "rust": inputs.rust_version,
            "python": inputs.python_version
        },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "lockDigest": inputs.lock_digest,
            "executableDigest": inputs.producer_executable_digest,
            "buildProfile": "release",
            "toolchains": { "node": inputs.node_version, "rust": inputs.rust_version, "python": inputs.python_version },
            "sources": {
                "quire-code-rs": { "revision": provenance.source_revision, "sourceState": "clean", "remote": inputs.source_remote },
                "quire-corpus": { "revision": inputs.corpus_source_revision, "sourceState": "clean", "remote": inputs.corpus_remote }
            },
            "capabilities": ["graph-quality-observation-v1", "quire-corpus-scorer-v1"],
            "artifacts": {
                "configuration": provenance.configuration_digest,
                "release-extractor": inputs.extractor_executable_digest,
                "measurement-plan": inputs.plan_digest,
                "observation-schema": inputs.schema_digest,
                "raw-scorer-output": observation["raw_scorer_output"]["digest"]
            }
        },
        "observations": observations,
        "rawEvidence": { "graphQualityObservation": observation, "scorerReport": scorer_report }
    }))
}

fn observation_id(value: &Value) -> Result<String, MeasurementError> {
    let mut content = value.clone();
    content
        .as_object_mut()
        .ok_or_else(|| {
            MeasurementError::InvalidObservation("observation must be an object".into())
        })?
        .remove("observation_id");
    Ok(sha256(&canonical_bytes(&content)?))
}

fn census_from(report: &Value) -> Result<Value, MeasurementError> {
    let confusion = report
        .get("confusion")
        .and_then(Value::as_object)
        .ok_or_else(|| MeasurementError::InvalidScorerReport("missing confusion object".into()))?;
    let languages = truth_axis_census(confusion.get("language"));
    let node_kinds = truth_axis_census(confusion.get("object_type"));
    let relation = truth_axis_census(confusion.get("relation"));
    let tier = truth_axis_census(confusion.get("tier"));
    Ok(json!({
        "languages": census_items(languages),
        "node_kinds": census_items(node_kinds),
        "relation_kinds": census_items(relation),
        "resolver_tiers": census_items(tier)
    }))
}

fn truth_axis_census(axis: Option<&Value>) -> BTreeMap<String, u64> {
    axis.and_then(Value::as_object)
        .map(|entries| {
            entries
                .iter()
                .map(|(key, value)| {
                    // The population is the declared truth set. Producer-only
                    // false positives are results, never population members.
                    let count = ["tp", "fn"]
                        .iter()
                        .filter_map(|field| value.get(field).and_then(Value::as_u64))
                        .sum();
                    (key.clone(), count)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn census_items(values: BTreeMap<String, u64>) -> Vec<Value> {
    values
        .into_iter()
        .map(|(key, count)| json!({"key": key, "count": count}))
        .collect()
}

fn empty_census() -> Value {
    json!({"languages": [], "node_kinds": [], "relation_kinds": [], "resolver_tiers": []})
}

fn results_from(report: &Value) -> Result<Value, MeasurementError> {
    let confusion = report
        .get("confusion")
        .and_then(Value::as_object)
        .ok_or_else(|| MeasurementError::InvalidScorerReport("missing confusion object".into()))?;
    let mut matrices = Vec::new();
    for (axis, dimension) in [
        ("total", "overall"),
        ("language", "language"),
        ("object_type", "node_kind"),
        ("relation", "relation_kind"),
        ("tier", "resolver_tier"),
    ] {
        let entries = confusion.get(axis).and_then(Value::as_object);
        if dimension == "overall" && entries.is_none() {
            return Err(MeasurementError::InvalidScorerReport(
                "missing total confusion".into(),
            ));
        }
        for (key, counts) in entries.into_iter().flat_map(|value| value.iter()) {
            matrices.push(json!({
                "dimension": dimension,
                "key": if dimension == "overall" { "overall" } else { key },
                "true_positive": counts.get("tp").and_then(Value::as_u64).unwrap_or(0),
                "false_positive": counts.get("fp").and_then(Value::as_u64).unwrap_or(0),
                "false_negative": counts.get("fn").and_then(Value::as_u64).unwrap_or(0),
                "true_negative": Value::Null
            }));
        }
    }
    matrices.sort_by_key(dimension_sort_key);
    let recall: Vec<Value> = matrices
        .iter()
        .filter_map(|matrix| {
            let tp = matrix["true_positive"].as_u64()?;
            let expected = tp + matrix["false_negative"].as_u64()?;
            (expected > 0).then(|| {
                json!({
                    "dimension": matrix["dimension"], "key": matrix["key"],
                    "recovered": tp, "expected": expected, "ratio": tp as f64 / expected as f64
                })
            })
        })
        .collect();
    let (unresolved_total, ambiguous_total, by_language) = unresolved_counts(report)?;
    let unresolved = dimension_counts(unresolved_total, &by_language, "reported");
    let ambiguous = dimension_counts(ambiguous_total, &by_language, "expected");
    Ok(
        json!({"confusion_matrices": matrices, "unresolved": unresolved, "ambiguous": ambiguous, "recall": recall}),
    )
}

fn unresolved_counts(report: &Value) -> Result<UnresolvedCounts, MeasurementError> {
    let cases = report
        .get("cases")
        .and_then(Value::as_object)
        .ok_or_else(|| MeasurementError::InvalidScorerReport("missing cases object".into()))?;
    let mut by_language = BTreeMap::<String, (u64, u64)>::new();
    for (name, case) in cases {
        let language = name.rsplit('/').next().unwrap_or("mixed").to_string();
        let census = case.pointer("/census/ambiguous_call_sites");
        let reported = census
            .and_then(|v| v.get("reported"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let expected = census
            .and_then(|v| v.get("expected"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let entry = by_language.entry(language).or_default();
        entry.0 += reported;
        entry.1 += expected;
    }
    Ok((
        by_language.values().map(|x| x.0).sum(),
        by_language.values().map(|x| x.1).sum(),
        by_language,
    ))
}

fn dimension_counts(
    total: u64,
    languages: &BTreeMap<String, (u64, u64)>,
    which: &str,
) -> Vec<Value> {
    let mut out = vec![
        json!({"dimension":"overall", "key":"overall", "count":total}),
        // `ambiguous_call_sites` is a homogeneous truth population: each item
        // is a call site, concerns a `calls` relation, and was left unresolved.
        // These are exact marginal strata, not inferred producer node counts.
        json!({"dimension":"node_kind", "key":"call_site", "count":total}),
        json!({"dimension":"relation_kind", "key":"calls", "count":total}),
        json!({"dimension":"resolver_tier", "key":"unresolved", "count":total}),
    ];
    for (language, counts) in languages {
        out.push(json!({"dimension":"language", "key":language, "count": if which == "reported" { counts.0 } else { counts.1 }}));
    }
    out.sort_by_key(dimension_sort_key);
    out
}

/// Evaluate only the governed precision rule. Scorer process status and recall
/// are deliberately excluded because the plan gives recall no pass threshold.
pub fn precision_decision_passed(report: &Value) -> Result<bool, MeasurementError> {
    let false_positives = report
        .pointer("/confusion/axis_kind/edge/fp")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            MeasurementError::InvalidScorerReport(
                "missing edge false-positive count for precision decision".into(),
            )
        })?;
    Ok(false_positives == 0)
}

fn dimension_sort_key(value: &Value) -> (String, String) {
    (
        value
            .get("dimension")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        value
            .get("key")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    )
}

fn require_sorted_unique(items: &[Value], pointer: &str) -> Result<(), MeasurementError> {
    let keys: Vec<_> = items.iter().map(dimension_sort_key).collect();
    let sorted: Vec<_> = keys
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if keys != sorted {
        return Err(MeasurementError::InvalidObservation(format!(
            "{pointer} must be sorted without duplicates"
        )));
    }
    Ok(())
}

fn quoin_observations(
    observation: &Value,
    scorer_report: Option<&Value>,
) -> Result<Vec<Value>, MeasurementError> {
    let state = observation
        .pointer("/population/state")
        .and_then(Value::as_str)
        .unwrap_or("unsupported");
    if state != "measured" {
        return Ok(vec![json!({
            "metric": METRIC, "planId": PLAN_ID, "definitionVersion": DEFINITION_VERSION,
            "state": "not_computed", "value": Value::Null, "unit": "decision", "shape": "scalar",
            "dimensions": {"measure":"precision_decision", "dimension":"overall", "key":"overall", "population_state":state},
            "reason": format!("population state is {state}")
        })]);
    }
    let matrices = observation
        .pointer("/results/confusion_matrices")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            MeasurementError::InvalidObservation("results omit confusion matrices".into())
        })?;
    let recalls = observation
        .pointer("/results/recall")
        .and_then(Value::as_array)
        .unwrap_or(&Vec::new())
        .to_vec();
    let mut out = Vec::new();
    for matrix in matrices {
        for (measure, field) in [
            ("true_positive", "true_positive"),
            ("false_positive", "false_positive"),
            ("false_negative", "false_negative"),
        ] {
            out.push(quoin_value(
                measure,
                matrix,
                matrix[field].clone(),
                "count",
                "count",
            ));
        }
        let tp = matrix["true_positive"].as_u64().unwrap_or(0);
        let fp = matrix["false_positive"].as_u64().unwrap_or(0);
        if tp + fp > 0 {
            out.push(quoin_value(
                "precision",
                matrix,
                json!(tp as f64 / (tp + fp) as f64),
                "fraction",
                "ratio",
            ));
        }
    }
    let scorer_report = scorer_report.ok_or_else(|| {
        MeasurementError::InvalidScorerReport("measured collection has no scorer report".into())
    })?;
    let decision = precision_decision_passed(scorer_report)?;
    let edge_tp = scorer_report
        .pointer("/confusion/axis_kind/edge/tp")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let edge_fp = scorer_report
        .pointer("/confusion/axis_kind/edge/fp")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    out.push(json!({
        "metric": METRIC, "planId": PLAN_ID, "definitionVersion": DEFINITION_VERSION,
        "state": "measured", "value": if decision { 1 } else { 0 }, "unit": "decision", "shape": "scalar",
        "population": {"examined": edge_tp + edge_fp, "matched": edge_tp, "complete": true, "identity": {"dimension":"overall", "key":"heuristic_edges"}},
        "dimensions": {"measure":"precision_decision", "dimension":"overall", "key":"overall", "population_state":"measured"}
    }));
    for recall in &recalls {
        out.push(quoin_value(
            "recall",
            recall,
            recall["ratio"].clone(),
            "fraction",
            "ratio",
        ));
    }
    for (measure, pointer) in [
        ("unresolved", "/results/unresolved"),
        ("ambiguous", "/results/ambiguous"),
    ] {
        for item in observation
            .pointer(pointer)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            out.push(quoin_value(
                measure,
                item,
                item["count"].clone(),
                "count",
                "count",
            ));
        }
    }
    out.sort_by(|a, b| {
        canonical_bytes(a)
            .unwrap_or_default()
            .cmp(&canonical_bytes(b).unwrap_or_default())
    });
    Ok(out)
}

fn quoin_value(measure: &str, item: &Value, value: Value, unit: &str, shape: &str) -> Value {
    let tp = item.get("true_positive").and_then(Value::as_u64);
    let fp = item.get("false_positive").and_then(Value::as_u64);
    let fn_count = item.get("false_negative").and_then(Value::as_u64);
    let recovered = item.get("recovered").and_then(Value::as_u64);
    let expected = item.get("expected").and_then(Value::as_u64);
    let (examined, matched) = match measure {
        "precision" => (tp.zip(fp).map(|(a, b)| a + b), tp),
        "recall" => (expected, recovered),
        _ => (tp.zip(fn_count).map(|(a, b)| a + b), tp),
    };
    json!({
        "metric": METRIC, "planId": PLAN_ID, "definitionVersion": DEFINITION_VERSION,
        "state": "measured", "value": value, "unit": unit, "shape": shape,
        "population": {"examined": examined, "matched": matched, "complete": true, "identity": observation_population_identity(item)},
        "dimensions": {"measure":measure, "dimension":item["dimension"], "key":item["key"], "population_state":"measured"}
    })
}

fn observation_population_identity(item: &Value) -> Value {
    json!({"dimension":item["dimension"], "key":item["key"]})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> Value {
        json!({
            "schema_version":1, "corpus_revision":format!("sha256:{}", "c".repeat(64)), "scored_cases":2,
            "confusion": {
                "total":{"total":{"tp":8,"fp":0,"fn":2}},
                "language":{"rust":{"tp":4,"fp":0,"fn":1},"python":{"tp":4,"fp":0,"fn":1}},
                "object_type":{"code_function":{"tp":2,"fp":0,"fn":1}},
                "relation":{"calls":{"tp":3,"fp":0,"fn":1}},
                "tier":{"receiver-typed":{"tp":3,"fp":0,"fn":1}},
                "axis_kind":{"edge":{"tp":3,"fp":0,"fn":1},"node":{"tp":5,"fp":0,"fn":1}}
            },
            "cases": {
                "relations/a/python":{"kind_census":{"code_function":{"function":2}},"census":{"ambiguous_call_sites":{"reported":1,"expected":1}}},
                "relations/a/rust":{"kind_census":{"code_function":{"function":3}},"census":{}}
            }
        })
    }

    fn provenance() -> Provenance {
        Provenance {
            extractor_revision: "a".repeat(40),
            source_revision: "a".repeat(40),
            corpus_revision: format!("sha256:{}", "c".repeat(64)),
            scorer_version: "b".repeat(40),
            configuration_digest: format!("sha256:{}", "d".repeat(64)),
            parser_grammars: vec![
                GrammarRevision {
                    language: "python".into(),
                    grammar: "tree-sitter-python".into(),
                    revision: "0.25.0".into(),
                },
                GrammarRevision {
                    language: "rust".into(),
                    grammar: "tree-sitter-rust".into(),
                    revision: "0.24.2".into(),
                },
                GrammarRevision {
                    language: "tsx".into(),
                    grammar: "tree-sitter-typescript".into(),
                    revision: "0.23.2".into(),
                },
                GrammarRevision {
                    language: "typescript".into(),
                    grammar: "tree-sitter-typescript".into(),
                    revision: "0.23.2".into(),
                },
            ],
        }
    }

    fn population(state: PopulationState) -> Population {
        Population {
            state,
            files_seen: if state == PopulationState::Empty {
                0
            } else {
                2
            },
            supported_files: if state == PopulationState::Measured {
                2
            } else {
                0
            },
            unreadable_files: if state == PopulationState::Unreadable {
                1
            } else {
                0
            },
            unsupported_files: if state == PopulationState::Unsupported {
                2
            } else {
                0
            },
        }
    }

    fn collection_inputs() -> CollectionInputs {
        CollectionInputs {
            timestamp: "2026-08-31T00:00:00Z".into(),
            lock_digest: format!("sha256:{}", "1".repeat(64)),
            producer_executable_digest: format!("sha256:{}", "2".repeat(64)),
            extractor_executable_digest: format!("sha256:{}", "3".repeat(64)),
            schema_digest: format!("sha256:{}", "4".repeat(64)),
            plan_digest: format!("sha256:{}", "5".repeat(64)),
            node_version: "24.15.0".into(),
            rust_version: "1.95.0".into(),
            python_version: "3.14.7".into(),
            source_remote: "local-source".into(),
            corpus_source_revision: "b".repeat(40),
            corpus_remote: "local-corpus".into(),
        }
    }

    fn reseal(value: &mut Value) {
        value["observation_id"] = Value::String(observation_id(value).unwrap());
    }

    // TC-112, TC-113, TC-130, TC-131 / FR-011-AC-1..2, FR-011-CON-1..2.
    #[test]
    fn measured_observation_is_schema_valid_and_dimension_complete() {
        let value = build_observation(
            Some(&report()),
            &population(PopulationState::Measured),
            &provenance(),
            "raw/scorer.json",
            &format!("sha256:{}", "e".repeat(64)),
        )
        .unwrap();
        validate_observation(&value).unwrap();
        let dimensions: BTreeSet<_> = value
            .pointer("/results/confusion_matrices")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["dimension"].as_str().unwrap())
            .collect();
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
        assert!(value
            .pointer("/results/confusion_matrices/0/true_negative")
            .unwrap()
            .is_null());
        assert_eq!(
            value.pointer("/population/census/node_kinds/0/count"),
            Some(&json!(3)),
            "truth census excludes producer-only false positives"
        );
        for pointer in ["/results/unresolved", "/results/ambiguous"] {
            let items = value.pointer(pointer).unwrap().as_array().unwrap();
            let dimensions = items
                .iter()
                .map(|item| item["dimension"].as_str().unwrap())
                .collect::<BTreeSet<_>>();
            assert_eq!(dimensions, BTreeSet::from(RESULT_DIMENSIONS));
            for (dimension, key, count) in [
                ("overall", "overall", 1),
                ("language", "python", 1),
                ("language", "rust", 0),
                ("node_kind", "call_site", 1),
                ("relation_kind", "calls", 1),
                ("resolver_tier", "unresolved", 1),
            ] {
                assert!(items.iter().any(|item| {
                    item["dimension"] == dimension && item["key"] == key && item["count"] == count
                }));
            }
        }
    }

    // TC-114, TC-122 / FR-011-AC-3, FR-012-AC-5.
    #[test]
    fn absence_states_never_acquire_results_or_measured_zero() {
        for state in [
            PopulationState::Empty,
            PopulationState::Unreadable,
            PopulationState::Unsupported,
        ] {
            let value = build_observation(
                None,
                &population(state),
                &provenance(),
                "raw/scorer.json",
                &format!("sha256:{}", "e".repeat(64)),
            )
            .unwrap();
            assert!(value.get("results").is_none());
            validate_observation(&value).unwrap();
        }
    }

    // TC-115, TC-116, TC-117 / FR-011-AC-4..6.
    #[test]
    fn schema_rejects_bad_provenance_paths_vocabularies_and_unknown_fields() {
        let good = build_observation(
            Some(&report()),
            &population(PopulationState::Measured),
            &provenance(),
            "raw/scorer.json",
            &format!("sha256:{}", "e".repeat(64)),
        )
        .unwrap();
        for mutate in [
            |v: &mut Value| v["producer"]["source_revision"] = json!("short"),
            |v: &mut Value| v["raw_scorer_output"]["path"] = json!("/tmp/raw.json"),
            |v: &mut Value| v["raw_scorer_output"]["path"] = json!("../raw.json"),
            |v: &mut Value| v["raw_scorer_output"]["path"] = json!(r"\raw.json"),
            |v: &mut Value| v["raw_scorer_output"]["path"] = json!(r"\\server\share\raw.json"),
            |v: &mut Value| v["raw_scorer_output"]["path"] = json!(r"..\raw.json"),
            |v: &mut Value| v["raw_scorer_output"]["path"] = json!(r"C:raw.json"),
            |v: &mut Value| v["producer"]["parser_grammars"][0]["language"] = json!("java"),
            |v: &mut Value| {
                v["measurement_plan"]["ref"] = json!("ix://agent-ix/quire-code-rs/MP-999")
            },
            |v: &mut Value| v["surprise"] = json!(true),
        ] {
            let mut bad = good.clone();
            mutate(&mut bad);
            reseal(&mut bad);
            assert!(validate_observation(&bad).is_err());
        }

        let mut bad_language = good.clone();
        let language = bad_language["results"]["recall"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["dimension"] == "language")
            .unwrap();
        language["key"] = json!("java");
        reseal(&mut bad_language);
        assert!(validate_observation(&bad_language).is_err());

        let mut missing_dimension = good.clone();
        missing_dimension["results"]["ambiguous"]
            .as_array_mut()
            .unwrap()
            .retain(|item| item["dimension"] != "resolver_tier");
        reseal(&mut missing_dimension);
        assert!(validate_observation(&missing_dimension).is_err());
    }

    // TC-120, TC-121, TC-128, TC-132 / FR-012-AC-3..4, NFR-005-AC-3,
    // FR-012-CON-1: recall is reported but never changes the false-positive gate.
    #[test]
    fn recall_and_wrong_edges_stay_independent_and_no_ambient_identity_is_added() {
        let value = build_observation(
            Some(&report()),
            &population(PopulationState::Measured),
            &provenance(),
            "raw/scorer.json",
            &format!("sha256:{}", "e".repeat(64)),
        )
        .unwrap();
        assert!(precision_decision_passed(&report()).unwrap());
        let mut wrong_edge = report();
        wrong_edge["confusion"]["axis_kind"]["edge"]["fp"] = json!(1);
        assert!(!precision_decision_passed(&wrong_edge).unwrap());
        assert_eq!(value.pointer("/results/recall/0/ratio"), Some(&json!(0.8)));
        let text = String::from_utf8(canonical_bytes(&value).unwrap()).unwrap();
        for forbidden in ["hostname", "process_id", "/Users/", "timestamp"] {
            assert!(!text.contains(forbidden));
        }
    }

    // TC-108, TC-110, TC-120 / StR-003-VC-1..3, FR-012-AC-3:
    // Quoin receives typed precision and the producer/extractor attestations
    // retain their distinct executable identities.
    #[test]
    fn collection_exposes_precision_decision_and_both_executable_digests() {
        let scorer = report();
        let observation = build_observation(
            Some(&scorer),
            &population(PopulationState::Measured),
            &provenance(),
            "raw/scorer.json",
            &format!("sha256:{}", "e".repeat(64)),
        )
        .unwrap();
        let collection = build_quoin_collection(
            &observation,
            Some(&scorer),
            &provenance(),
            &collection_inputs(),
        )
        .unwrap();
        assert_eq!(
            collection.pointer("/verificationStack/executableDigest"),
            Some(&json!(format!("sha256:{}", "2".repeat(64))))
        );
        assert_eq!(
            collection.pointer("/verificationStack/artifacts/release-extractor"),
            Some(&json!(format!("sha256:{}", "3".repeat(64))))
        );
        let measures = collection["observations"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item.pointer("/dimensions/measure").and_then(Value::as_str))
            .collect::<BTreeSet<_>>();
        assert!(measures.contains("precision"));
        assert!(measures.contains("precision_decision"));
    }

    // TC-124, TC-126 / FR-012-AC-7, NFR-005-AC-1.
    #[test]
    fn pinned_inputs_are_byte_identical_when_report_maps_arrive_reordered() {
        let first = build_observation(
            Some(&report()),
            &population(PopulationState::Measured),
            &provenance(),
            "raw/scorer.json",
            &format!("sha256:{}", "e".repeat(64)),
        )
        .unwrap();
        let mut second_report = report();
        let cases = second_report["cases"].as_object_mut().unwrap();
        let rust = cases.remove("relations/a/rust").unwrap();
        cases.insert("relations/a/rust".into(), rust);
        let second = build_observation(
            Some(&second_report),
            &population(PopulationState::Measured),
            &provenance(),
            "raw/scorer.json",
            &format!("sha256:{}", "e".repeat(64)),
        )
        .unwrap();
        assert_eq!(
            canonical_bytes(&first).unwrap(),
            canonical_bytes(&second).unwrap()
        );
    }
}
