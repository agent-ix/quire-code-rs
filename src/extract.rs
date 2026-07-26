//! Batch extraction: the library's entry point.
//!
//! Ties parsing (FR-001), naming (FR-002), structural edges (FR-003),
//! provenance (FR-004), mentions (FR-005) and canonical emission (FR-006)
//! into one pass over a batch, and reports what the batch bounded
//! (FR-008-AC-11).
//!
//! Load → resolve → emit → drop: the fact corpus lives for the call and is
//! dropped with it (ADR-002).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::edges::{EdgeAccumulator, EdgeType, Evidence, Reason};
use crate::facts::{CodeFact, Diagnostic, ObjectType};
use crate::imports;
use crate::mentions::{self, Mention};
use crate::parse::{self, ParsedFile, SourceFile};
use crate::records::{self, EdgeRecord, NodeRecord};
use crate::resolve;
use crate::typeenv::{Corpus, TypeEnv};

/// What one extraction produced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub nodes: Vec<NodeRecord>,
    pub edges: Vec<EdgeRecord>,
    pub mentions: Vec<Mention>,
    pub diagnostics: Vec<Diagnostic>,
    pub stats: ExtractionStats,
}

/// What the batch bounded, so an under-supplied batch is visible rather than
/// silent (FR-008-AC-11, SR-003 FND-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExtractionStats {
    pub files: u32,
    pub files_with_errors: u32,
    pub unresolved_calls: u32,
    /// Highest fixpoint iteration count reached by any file's type environment
    /// (NFR-003-AC-4). Measured, not the configured bound.
    pub max_fixpoint_iterations: u32,
    /// Files whose fixpoint hit the iteration bound with bindings still
    /// pending. Non-zero means edges were lost to the bound, not to the source.
    pub files_hitting_iteration_bound: u32,
}

/// Extract canonical records from a batch of source files.
///
/// Pure: no filesystem, no network, no clock, no randomness (NFR-002). Two
/// calls with equal input return equal output (NFR-001).
pub fn extract(files: &[SourceFile]) -> ExtractionResult {
    let batch_paths: BTreeSet<String> = files.iter().map(|f| f.normalized_path()).collect();

    let mut all_facts: Vec<CodeFact> = Vec::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut all_mentions: Vec<Mention> = Vec::new();
    let mut accumulator = EdgeAccumulator::new();
    let mut files_with_errors = 0u32;

    // Sort the batch so output does not depend on the order a consumer happens
    // to supply files in (NFR-001-AC-3).
    let mut ordered: Vec<&SourceFile> = files.iter().collect();
    ordered.sort_by(|a, b| {
        (a.org.as_str(), a.repo.as_str(), a.normalized_path()).cmp(&(
            b.org.as_str(),
            b.repo.as_str(),
            b.normalized_path(),
        ))
    });

    // Resolution needs the whole batch before any call can be resolved, so the
    // first pass parses and the second resolves (ADR-002: the corpus lives for
    // the call and is dropped with it).
    let mut parsed_files: Vec<(&SourceFile, String, ParsedFile)> = Vec::new();
    for file in &ordered {
        let path = file.normalized_path();
        let parsed = parse::parse_file(file);
        parsed_files.push((file, path, parsed));
    }

    let corpus = build_corpus(&parsed_files);
    let import_edges = resolve_import_edges(&parsed_files, &batch_paths);
    let types_by_file = types_by_file(&parsed_files);
    let mut unresolved_calls = 0u32;
    let mut max_fixpoint_iterations = 0u32;
    let mut files_hitting_iteration_bound = 0u32;

    for (file, path, parsed) in &parsed_files {
        let (file, path, parsed) = (*file, path.clone(), parsed);

        if parsed.diagnostics.iter().any(|d| d.code == "parse_error") {
            files_with_errors += 1;
        }
        diagnostics.extend(parsed.diagnostics.iter().cloned());

        // Containment (FR-003): every fact with a parent contributes one edge.
        for fact in &parsed.facts {
            if let Some(parent) = &fact.parent {
                accumulator.add(
                    parent.clone(),
                    EdgeType::Contains,
                    fact.qualified_name.clone(),
                    Reason::Syntactic,
                    Evidence {
                        file: path.clone(),
                        line: fact.span.start,
                    },
                );
            }
        }

        // Imports (FR-003). Bare specifiers resolve to nothing and are silent;
        // only an unresolvable *relative* specifier is diagnosed.
        for (specifier, line) in &parsed.imports {
            if !imports::is_relative(specifier, file.language) {
                continue;
            }
            match imports::resolve(specifier, &path, file.language, &batch_paths) {
                Some(target_path) => {
                    let target = crate::naming::file_name(&file.org, &file.repo, &target_path);
                    accumulator.add(
                        parsed.file_qualified_name.clone(),
                        EdgeType::Imports,
                        target,
                        Reason::PathResolved,
                        Evidence {
                            file: path.clone(),
                            line: *line,
                        },
                    );
                }
                None => diagnostics.push(Diagnostic::unresolved_import(&path, specifier, *line)),
            }
        }

        // Mentions (FR-005). The edge targets the identifier as written; the
        // consumer resolves it against its own index.
        let found = mentions::harvest(&parsed.comments, &path);
        for mention in &found {
            accumulator.add(
                mention.source.clone(),
                EdgeType::References,
                mention.identifier.clone(),
                Reason::ExplicitMention,
                Evidence {
                    file: path.clone(),
                    line: mention.line,
                },
            );
        }
        all_mentions.extend(found);

        // Calls and type relations (FR-008). The type environment is built per
        // file and dropped with the iteration.
        let env = TypeEnv::build(&parsed.bindings, &parsed.enclosing_types, &corpus);
        let reachable = resolve::imported_types(&path, &import_edges, &types_by_file);
        let local = local_scope(parsed);
        max_fixpoint_iterations = max_fixpoint_iterations.max(env.iterations as u32);
        if env.hit_iteration_bound {
            files_hitting_iteration_bound += 1;
        }

        for call in &parsed.calls {
            match resolve::resolve_call(call, &env, &corpus, &reachable, &local) {
                Some(resolution) => accumulator.add(
                    call.caller.clone(),
                    EdgeType::Calls,
                    resolution.target,
                    resolution.reason,
                    Evidence {
                        file: path.clone(),
                        line: call.line,
                    },
                ),
                None => unresolved_calls += 1,
            }
        }

        for (child, parent, is_trait) in &parsed.type_relations {
            let Some(child_qualified) = resolve::resolve_type_relation(&corpus, &local, child)
            else {
                continue;
            };
            let Some(parent_qualified) = resolve::resolve_type_relation(&corpus, &local, parent)
            else {
                continue;
            };
            accumulator.add(
                child_qualified,
                if *is_trait {
                    EdgeType::ImplementsTrait
                } else {
                    EdgeType::Extends
                },
                parent_qualified,
                Reason::Syntactic,
                Evidence {
                    file: path.clone(),
                    line: 1,
                },
            );
        }

        all_facts.extend(parsed.facts.iter().cloned());
    }

    let mut nodes: Vec<NodeRecord> = all_facts.iter().map(records::node_record).collect();
    records::sort_nodes(&mut nodes);

    let mut edges: Vec<EdgeRecord> = accumulator
        .finish()
        .iter()
        .map(records::edge_record)
        .collect();
    records::sort_edges(&mut edges);

    all_mentions.sort();
    all_mentions.dedup();

    diagnostics.sort_by(|a, b| {
        (a.path.as_str(), a.line, a.code.as_str()).cmp(&(b.path.as_str(), b.line, b.code.as_str()))
    });

    ExtractionResult {
        nodes,
        edges,
        mentions: all_mentions,
        diagnostics,
        stats: ExtractionStats {
            files: files.len() as u32,
            files_with_errors,
            unresolved_calls,
            max_fixpoint_iterations,
            files_hitting_iteration_bound,
        },
    }
}

/// Build the batch-wide declaration index the solver resolves against.
fn build_corpus(parsed_files: &[(&SourceFile, String, ParsedFile)]) -> Corpus {
    let mut corpus = Corpus::default();

    for (_, _, parsed) in parsed_files {
        for fact in &parsed.facts {
            match fact.object_type {
                ObjectType::Type => {
                    corpus
                        .types
                        .entry(fact.simple_name.clone())
                        .or_default()
                        .insert(fact.qualified_name.clone());
                }
                ObjectType::Function => {
                    // A method's owning type is the segment before its own.
                    match owning_type_segment(&fact.qualified_name) {
                        Some(owner) => {
                            corpus
                                .methods
                                .entry((owner, fact.simple_name.clone()))
                                .or_default()
                                .insert(fact.qualified_name.clone());
                        }
                        None => {
                            corpus
                                .functions
                                .entry(fact.simple_name.clone())
                                .or_default()
                                .insert(fact.qualified_name.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        corpus.return_types.extend(
            parsed
                .return_types
                .iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );
        corpus.field_types.extend(
            parsed
                .field_types
                .iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );
    }

    corpus
}

/// The type segment owning a qualified callable name, when it has one.
///
/// `org/repo/path.rs::Store::upsert` -> `Store`; a free function's name has no
/// segment before its own, so it yields `None`.
fn owning_type_segment(qualified: &str) -> Option<String> {
    let mut segments = qualified.split("::");
    let _path = segments.next()?;
    let parts: Vec<&str> = segments.collect();
    if parts.len() < 2 {
        return None;
    }
    parts
        .get(parts.len() - 2)
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

/// What one file declares itself, so same-file resolution never depends on the
/// rest of the batch (FR-008-AC-9).
fn local_scope(parsed: &ParsedFile) -> resolve::LocalScope {
    let mut local = resolve::LocalScope::default();
    for fact in &parsed.facts {
        match fact.object_type {
            ObjectType::Type => {
                local
                    .types
                    .entry(fact.simple_name.clone())
                    .or_insert_with(|| fact.qualified_name.clone());
            }
            ObjectType::Function => match owning_type_segment(&fact.qualified_name) {
                Some(owner) => {
                    local
                        .methods
                        .entry((owner, fact.simple_name.clone()))
                        .or_insert_with(|| fact.qualified_name.clone());
                }
                None => {
                    local
                        .functions
                        .entry(fact.simple_name.clone())
                        .or_insert_with(|| fact.qualified_name.clone());
                }
            },
            _ => {}
        }
    }
    local
}

/// The import graph, keyed by file path, for tier-2 candidate narrowing.
fn resolve_import_edges(
    parsed_files: &[(&SourceFile, String, ParsedFile)],
    batch_paths: &BTreeSet<String>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (file, path, parsed) in parsed_files {
        for (specifier, _) in &parsed.imports {
            if let Some(target) = imports::resolve(specifier, path, file.language, batch_paths) {
                out.entry(path.clone()).or_default().insert(target);
            }
        }
    }
    out
}

/// Simple type names declared per file, for tier-2 candidate narrowing.
fn types_by_file(
    parsed_files: &[(&SourceFile, String, ParsedFile)],
) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (_, path, parsed) in parsed_files {
        let declared: BTreeSet<String> = parsed
            .facts
            .iter()
            .filter(|f| f.object_type == ObjectType::Type)
            .map(|f| f.simple_name.clone())
            .collect();
        out.insert(path.clone(), declared);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::Language;

    fn batch() -> Vec<SourceFile> {
        vec![
            SourceFile::new(
                "agent-ix",
                "demo",
                "src/store.rs",
                Language::Rust,
                "//! Implements FR-002.\npub struct Store;\nimpl Store {\n    pub fn upsert(&self) {}\n}\n",
            ),
            SourceFile::new(
                "agent-ix",
                "demo",
                "src/lib.rs",
                Language::Rust,
                "use crate::store::Store;\n\n#[test]\nfn checks_it() {\n    // TC-001\n}\n",
            ),
        ]
    }

    // TC-015 — FR-003-AC-1: containment forms a tree rooted at the file.
    #[test]
    fn containment_forms_a_tree_rooted_at_the_file() {
        let result = extract(&batch());
        let contains: Vec<_> = result
            .edges
            .iter()
            .filter(|e| e.edge_type == "contains")
            .collect();
        assert!(!contains.is_empty());
        // Every contained node has exactly one parent.
        let mut targets: Vec<_> = contains.iter().map(|e| e.target_ref.as_str()).collect();
        let before = targets.len();
        targets.sort_unstable();
        targets.dedup();
        assert_eq!(before, targets.len(), "a node has two parents");
    }

    // TC-019 — FR-003-AC-5: structural edges carry confidence 1.0.
    #[test]
    fn structural_edges_are_fully_confident() {
        let result = extract(&batch());
        for edge in result
            .edges
            .iter()
            .filter(|e| e.edge_type == "contains" || e.edge_type == "imports")
        {
            assert_eq!(edge.confidence, 1.0, "{edge:?}");
        }
    }

    #[test]
    fn a_resolved_relative_import_yields_an_edge() {
        let result = extract(&batch());
        let import = result
            .edges
            .iter()
            .find(|e| e.edge_type == "imports")
            .expect("an imports edge");
        assert_eq!(import.target_ref, "agent-ix/demo/src/store.rs");
        assert_eq!(import.reason, "path-resolved");
    }

    // TC-017 — FR-003-AC-3: a bare specifier is silent; a broken relative one
    // is diagnosed.
    #[test]
    fn bare_imports_are_silent_and_broken_relative_imports_are_diagnosed() {
        let quiet = extract(&[SourceFile::new(
            "agent-ix",
            "demo",
            "src/lib.rs",
            Language::Rust,
            "use std::collections::BTreeMap;\n",
        )]);
        assert!(
            quiet
                .diagnostics
                .iter()
                .all(|d| d.code != "unresolved_import"),
            "got {:?}",
            quiet.diagnostics
        );

        let noisy = extract(&[SourceFile::new(
            "agent-ix",
            "demo",
            "src/app.ts",
            Language::TypeScript,
            "import { X } from './missing';\n",
        )]);
        assert!(
            noisy
                .diagnostics
                .iter()
                .any(|d| d.code == "unresolved_import"),
            "got {:?}",
            noisy.diagnostics
        );
    }

    #[test]
    fn mentions_become_reference_edges_addressed_as_written() {
        let result = extract(&batch());
        let mention_edge = result
            .edges
            .iter()
            .find(|e| e.target_ref == "TC-001")
            .expect("a mention edge for TC-001");
        assert_eq!(mention_edge.edge_type, "references");
        assert_eq!(mention_edge.reason, "explicit-mention");
        assert_eq!(mention_edge.confidence, 1.0);
    }

    // TC-037 — FR-006-AC-5: no timestamp, absolute path, hostname or pid.
    #[test]
    fn serialized_output_carries_nothing_environmental() {
        let result = extract(&batch());
        let json = serde_json::to_string(&result).expect("serializes");
        assert!(!json.contains("/Users/"), "absolute path leaked");
        assert!(!json.contains("/home/"), "absolute path leaked");
        for suspicious in ["timestamp", "created_at", "hostname", "pid"] {
            assert!(!json.contains(suspicious), "{suspicious} leaked");
        }
    }

    // TC-042 — FR-007-AC-4: healthy files are unaffected by a malformed sibling.
    #[test]
    fn a_malformed_file_does_not_change_its_siblings_records() {
        let healthy = extract(&batch());

        let mut with_broken = batch();
        with_broken.push(SourceFile::new(
            "agent-ix",
            "demo",
            "src/broken.rs",
            Language::Rust,
            "fn oops( {\n",
        ));
        let mixed = extract(&with_broken);

        let healthy_names: Vec<_> = healthy.nodes.iter().map(|n| n.name.as_str()).collect();
        let mixed_names: Vec<_> = mixed
            .nodes
            .iter()
            .map(|n| n.name.as_str())
            .filter(|n| !n.contains("broken.rs"))
            .collect();
        assert_eq!(healthy_names, mixed_names);
        assert_eq!(mixed.stats.files_with_errors, 1);
    }

    // TC-056 — NFR-001-AC-3: shuffling the batch changes nothing.
    #[test]
    fn batch_order_does_not_affect_output() {
        let forward = extract(&batch());
        let mut reversed = batch();
        reversed.reverse();
        let backward = extract(&reversed);
        assert_eq!(forward, backward);
    }

    // TC-054 — NFR-001-AC-1: repeated extractions are identical.
    #[test]
    fn repeated_extractions_are_identical() {
        let files = batch();
        let first = serde_json::to_string(&extract(&files)).unwrap();
        for _ in 0..25 {
            assert_eq!(serde_json::to_string(&extract(&files)).unwrap(), first);
        }
    }

    #[test]
    fn stats_report_what_the_batch_bounded() {
        let result = extract(&batch());
        assert_eq!(result.stats.files, 2);
        assert_eq!(result.stats.files_with_errors, 0);
    }
}
