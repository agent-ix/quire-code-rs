//! Edge provenance and deduplication.
//!
//! Implements [FR-004](../spec/functional/FR-004-edge-provenance-and-dedupe.md).
//! The `reason` vocabulary and the 20-entry evidence cap are a shared contract
//! with `ix://agent-ix/filament-ide-rs/FR-072` (FR-004-CON-1) — changing either
//! is a coordinated change, not a local one.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Maximum evidence entries retained on a deduplicated edge (FR-004-CON-2).
pub const EVIDENCE_CAP: usize = 20;

/// Why an edge was emitted. Exactly the six values FR-072 consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reason {
    Syntactic,
    PathResolved,
    NameMatch,
    ImportScoped,
    ReceiverTyped,
    ExplicitMention,
}

impl Reason {
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::Syntactic => "syntactic",
            Reason::PathResolved => "path-resolved",
            Reason::NameMatch => "name-match",
            Reason::ImportScoped => "import-scoped",
            Reason::ReceiverTyped => "receiver-typed",
            Reason::ExplicitMention => "explicit-mention",
        }
    }

    /// The confidence this reason carries.
    ///
    /// The three read-directly-from-source reasons are 1.0 by FR-004-AC-6. The
    /// inferred tiers descend with certainty; the spec ranks them but
    /// deliberately leaves the exact floats to implementation (SR-002 FND-004).
    pub fn confidence(self) -> f32 {
        match self {
            Reason::Syntactic | Reason::PathResolved | Reason::ExplicitMention => 1.0,
            Reason::ReceiverTyped => 0.9,
            Reason::ImportScoped => 0.7,
            Reason::NameMatch => 0.5,
        }
    }
}

/// The relationship an edge asserts. Exactly the six types FR-006 emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Contains,
    Imports,
    Calls,
    ImplementsTrait,
    Extends,
    References,
}

impl EdgeType {
    pub fn as_str(self) -> &'static str {
        match self {
            EdgeType::Contains => "contains",
            EdgeType::Imports => "imports",
            EdgeType::Calls => "calls",
            EdgeType::ImplementsTrait => "implements_trait",
            EdgeType::Extends => "extends",
            EdgeType::References => "references",
        }
    }
}

/// One call site or citation that contributed to an edge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Evidence {
    pub file: String,
    pub line: u32,
}

/// A deduplicated edge with its provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub source_ref: String,
    pub edge_type: EdgeType,
    pub target_ref: String,
    pub confidence: f32,
    pub reason: Reason,
    /// At most [`EVIDENCE_CAP`] entries, ordered by file then line.
    pub evidence: Vec<Evidence>,
    /// Total contributing sites, including those beyond the cap.
    pub count: u32,
}

/// Accumulates candidate edges and folds them onto one edge per
/// `(source_ref, edge_type, target_ref)` triple (FR-004).
#[derive(Debug, Default)]
pub struct EdgeAccumulator {
    /// BTreeMap, not HashMap: iteration order is observable in the output, and
    /// NFR-001 forbids anything order-dependent.
    edges: BTreeMap<(String, EdgeType, String), Pending>,
}

#[derive(Debug)]
struct Pending {
    reason: Reason,
    confidence: f32,
    evidence: Vec<Evidence>,
    count: u32,
}

impl EdgeAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one contributing site for an edge.
    ///
    /// Repeated calls for the same triple fold together: `count` accumulates,
    /// evidence collects, and the highest-confidence reason wins so a
    /// well-resolved site is not degraded by a weaker one (FR-004-AC-3).
    pub fn add(
        &mut self,
        source_ref: impl Into<String>,
        edge_type: EdgeType,
        target_ref: impl Into<String>,
        reason: Reason,
        evidence: Evidence,
    ) {
        let key = (source_ref.into(), edge_type, target_ref.into());

        // A self-edge is dropped at the one place every edge passes through
        // (FR-006-AC-7). It carries no traversal information — an impact
        // closure that reaches the node already has it — and the consumer's
        // graph rejects one outright, so emitting it fails a whole reindex run
        // rather than adding a wrong row (filament-ide-rs FR-072).
        //
        // Two sources produce them. A genuinely recursive call, which is what
        // this drop is for. And a type carrying both an inherent and a trait
        // `impl` of one method name, whose two declarations share a qualified
        // name under FR-002-AC-2 — there the self-edge is a *symptom* of an
        // identity collision, and dropping it hides rather than fixes it
        // (agent-ix/quire-code-rs#18).
        if key.0 == key.2 {
            return;
        }

        let confidence = reason.confidence();
        match self.edges.get_mut(&key) {
            Some(pending) => {
                pending.count = pending.count.saturating_add(1);
                pending.evidence.push(evidence);
                if confidence > pending.confidence {
                    pending.confidence = confidence;
                    pending.reason = reason;
                }
            }
            None => {
                self.edges.insert(
                    key,
                    Pending {
                        reason,
                        confidence,
                        evidence: vec![evidence],
                        count: 1,
                    },
                );
            }
        }
    }

    /// Finish, producing edges in stable order with evidence sorted and capped.
    pub fn finish(self) -> Vec<Edge> {
        self.edges
            .into_iter()
            .map(|((source_ref, edge_type, target_ref), mut pending)| {
                // Sort before truncating so the retained 20 are the first 20 in
                // source order, not the first 20 encountered (FR-004-AC-5).
                pending.evidence.sort();
                pending.evidence.dedup();
                pending.evidence.truncate(EVIDENCE_CAP);
                Edge {
                    source_ref,
                    edge_type,
                    target_ref,
                    confidence: pending.confidence,
                    reason: pending.reason,
                    evidence: pending.evidence,
                    count: pending.count,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(file: &str, line: u32) -> Evidence {
        Evidence {
            file: file.to_string(),
            line,
        }
    }

    // TC-106, FR-006-AC-7: an edge whose source and target are equal is not
    // emitted, whatever produced it.
    #[test]
    fn a_self_edge_is_never_emitted() {
        let mut acc = EdgeAccumulator::new();
        acc.add(
            "org/repo/src/lib.rs::recurse",
            EdgeType::Calls,
            "org/repo/src/lib.rs::recurse",
            Reason::ReceiverTyped,
            ev("src/lib.rs", 3),
        );
        acc.add(
            "org/repo/src/lib.rs::caller",
            EdgeType::Calls,
            "org/repo/src/lib.rs::callee",
            Reason::ReceiverTyped,
            ev("src/lib.rs", 9),
        );
        let edges = acc.finish();
        assert_eq!(
            edges
                .iter()
                .map(|e| (e.source_ref.as_str(), e.target_ref.as_str()))
                .collect::<Vec<_>>(),
            vec![("org/repo/src/lib.rs::caller", "org/repo/src/lib.rs::callee")],
            "the self-edge is dropped and the ordinary edge survives"
        );
    }

    // TC-020, FR-004-AC-1: two sites on one triple fold to one edge, count 2.
    #[test]
    fn repeated_sites_fold_onto_one_edge() {
        let mut acc = EdgeAccumulator::new();
        acc.add(
            "a",
            EdgeType::Calls,
            "b",
            Reason::ReceiverTyped,
            ev("f.rs", 10),
        );
        acc.add(
            "a",
            EdgeType::Calls,
            "b",
            Reason::ReceiverTyped,
            ev("f.rs", 20),
        );
        let edges = acc.finish();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].count, 2);
        assert_eq!(edges[0].evidence.len(), 2);
    }

    // TC-021, FR-004-AC-2, FR-004-CON-2: thirty sites yield count 30 and 20 evidence entries.
    #[test]
    fn evidence_is_capped_but_count_is_not() {
        let mut acc = EdgeAccumulator::new();
        for line in 1..=30 {
            acc.add(
                "a",
                EdgeType::Calls,
                "b",
                Reason::ReceiverTyped,
                ev("f.rs", line),
            );
        }
        let edges = acc.finish();
        assert_eq!(edges[0].count, 30);
        assert_eq!(edges[0].evidence.len(), EVIDENCE_CAP);
        // The retained entries are the first twenty in source order.
        assert_eq!(edges[0].evidence[0].line, 1);
        assert_eq!(edges[0].evidence[EVIDENCE_CAP - 1].line, 20);
    }

    // TC-022, FR-004-AC-3: highest confidence and its reason win.
    #[test]
    fn stronger_reason_wins_regardless_of_arrival_order() {
        let mut weak_first = EdgeAccumulator::new();
        weak_first.add("a", EdgeType::Calls, "b", Reason::NameMatch, ev("f.rs", 1));
        weak_first.add(
            "a",
            EdgeType::Calls,
            "b",
            Reason::ReceiverTyped,
            ev("f.rs", 2),
        );

        let mut strong_first = EdgeAccumulator::new();
        strong_first.add(
            "a",
            EdgeType::Calls,
            "b",
            Reason::ReceiverTyped,
            ev("f.rs", 2),
        );
        strong_first.add("a", EdgeType::Calls, "b", Reason::NameMatch, ev("f.rs", 1));

        for acc in [weak_first, strong_first] {
            let edges = acc.finish();
            assert_eq!(edges[0].reason, Reason::ReceiverTyped);
            assert_eq!(edges[0].confidence, Reason::ReceiverTyped.confidence());
        }
    }

    // TC-023, FR-004-AC-4: every reason is in the enum and confidence in [0,1].
    #[test]
    fn every_reason_carries_a_confidence_within_the_unit_interval() {
        for reason in [
            Reason::Syntactic,
            Reason::PathResolved,
            Reason::NameMatch,
            Reason::ImportScoped,
            Reason::ReceiverTyped,
            Reason::ExplicitMention,
        ] {
            let c = reason.confidence();
            assert!((0.0..=1.0).contains(&c), "{reason:?} -> {c}");
        }
    }

    // TC-024, FR-004-AC-5: evidence is ordered by file then line.
    #[test]
    fn evidence_is_ordered_by_file_then_line() {
        let mut acc = EdgeAccumulator::new();
        acc.add("a", EdgeType::Calls, "b", Reason::NameMatch, ev("z.rs", 5));
        acc.add("a", EdgeType::Calls, "b", Reason::NameMatch, ev("a.rs", 9));
        acc.add("a", EdgeType::Calls, "b", Reason::NameMatch, ev("a.rs", 2));
        let edges = acc.finish();
        let got: Vec<_> = edges[0]
            .evidence
            .iter()
            .map(|e| (e.file.as_str(), e.line))
            .collect();
        assert_eq!(got, vec![("a.rs", 2), ("a.rs", 9), ("z.rs", 5)]);
    }

    // TC-025, FR-004-AC-6: syntactic reasons carry confidence 1.0.
    #[test]
    fn source_read_reasons_are_fully_confident() {
        assert_eq!(Reason::Syntactic.confidence(), 1.0);
        assert_eq!(Reason::PathResolved.confidence(), 1.0);
        assert_eq!(Reason::ExplicitMention.confidence(), 1.0);
    }

    // TC-053, FR-008-AC-10: the resolution tiers rank as the contract states.
    #[test]
    fn resolution_tiers_rank_receiver_over_import_over_name() {
        assert!(Reason::ReceiverTyped.confidence() > Reason::ImportScoped.confidence());
        assert!(Reason::ImportScoped.confidence() > Reason::NameMatch.confidence());
    }

    #[test]
    fn edges_come_out_in_stable_order() {
        let mut acc = EdgeAccumulator::new();
        acc.add("z", EdgeType::Calls, "a", Reason::NameMatch, ev("f.rs", 1));
        acc.add(
            "a",
            EdgeType::Imports,
            "b",
            Reason::PathResolved,
            ev("f.rs", 1),
        );
        acc.add(
            "a",
            EdgeType::Contains,
            "c",
            Reason::Syntactic,
            ev("f.rs", 1),
        );
        let edges = acc.finish();
        let keys: Vec<_> = edges
            .iter()
            .map(|e| (e.source_ref.as_str(), e.edge_type.as_str()))
            .collect();
        assert_eq!(
            keys,
            vec![("a", "contains"), ("a", "imports"), ("z", "calls")]
        );
    }
}
