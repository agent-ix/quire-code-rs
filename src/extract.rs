//! Batch extraction: the library's entry point.
//!
//! Ties parsing (FR-001), naming (FR-002), structural edges (FR-003),
//! provenance (FR-004), mentions (FR-005) and canonical emission (FR-006)
//! into one pass over a batch, and reports what the batch bounded
//! (FR-008-AC-11).
//!
//! Load → resolve → emit → drop: the fact corpus lives for the call and is
//! dropped with it (ADR-002).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::edges::{EdgeAccumulator, EdgeType, Evidence, Reason};
use crate::facts::{CodeFact, Diagnostic};
use crate::imports;
use crate::mentions::{self, Mention};
use crate::parse::{self, SourceFile};
use crate::records::{self, EdgeRecord, NodeRecord};

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

    for file in ordered {
        let path = file.normalized_path();
        let parsed = parse::parse_file(file);

        if parsed.diagnostics.iter().any(|d| d.code == "parse_error") {
            files_with_errors += 1;
        }
        diagnostics.extend(parsed.diagnostics);

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

        all_facts.extend(parsed.facts);
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
            unresolved_calls: 0,
        },
    }
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
