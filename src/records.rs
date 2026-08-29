//! Canonical record emission at quire-rs parity.
//!
//! Implements [FR-006](../spec/functional/FR-006-canonical-record-emission.md).
//!
//! Node identity is a pure function of `(object_type, qualified_name)`
//! (FR-006-CON-1). Nothing mutable is hashed — not the line span, not the file
//! content — so moving a declaration within its file preserves its id while
//! updating its span (FR-006-AC-2). That is what makes a consumer's
//! skip-if-unchanged reindex sound.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::edges::{Edge, EdgeType, Reason};
use crate::facts::{CodeFact, LineSpan, ObjectType, Visibility};
use crate::naming::ix_ref;

/// A canonical graph node record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRecord {
    /// Lowercase-hex SHA-256 of `(object_type, qualified_name)`.
    pub id: String,
    /// `ix://{org}/{repo}/{qualified-name}`.
    pub reference: String,
    pub object_type: String,
    pub name: String,
    pub data: NodeData,
}

/// The typed payload carried on a node record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeData {
    /// The concrete declaration form: `struct`, `class`, `method`, …
    pub kind: String,
    pub path: String,
    pub span: LineSpan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// How widely the declaration is visible (FR-009). A consumer tiering an
    /// export-set change reads this to tell a new private helper — which
    /// invalidates nothing — from a new export.
    #[serde(default)]
    pub visibility: Visibility,
    /// Normalized parameter/return summary for a callable (FR-009), absent for
    /// every declaration with no parameter list. A change here is a change to
    /// the calling contract even when the exported *name* set is unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// A canonical graph edge record with its provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeRecord {
    pub source_ref: String,
    pub edge_type: String,
    pub target_ref: String,
    pub confidence: f32,
    pub reason: String,
    pub evidence: Vec<EvidenceRecord>,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub file: String,
    pub line: u32,
}

/// The stable record id for a node (FR-006-CON-1).
pub fn node_id(object_type: ObjectType, qualified_name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(object_type.as_str().as_bytes());
    // A separator no name can contain, so `(a, bc)` and `(ab, c)` cannot
    // collide.
    hasher.update([0x1f]);
    hasher.update(qualified_name.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Convert a fact into its canonical node record.
pub fn node_record(fact: &CodeFact) -> NodeRecord {
    NodeRecord {
        id: node_id(fact.object_type, &fact.qualified_name),
        reference: ix_ref(&fact.qualified_name),
        object_type: fact.object_type.as_str().to_string(),
        name: fact.qualified_name.clone(),
        data: NodeData {
            kind: fact.kind.to_string(),
            path: fact.path.clone(),
            span: fact.span,
            parent: fact.parent.clone(),
            visibility: fact.visibility,
            signature: fact.signature.clone(),
        },
    }
}

/// Convert an edge into its canonical edge record.
pub fn edge_record(edge: &Edge) -> EdgeRecord {
    EdgeRecord {
        source_ref: edge.source_ref.clone(),
        edge_type: edge.edge_type.as_str().to_string(),
        target_ref: edge.target_ref.clone(),
        confidence: edge.confidence,
        reason: edge.reason.as_str().to_string(),
        evidence: edge
            .evidence
            .iter()
            .map(|e| EvidenceRecord {
                file: e.file.clone(),
                line: e.line,
            })
            .collect(),
        count: edge.count,
    }
}

/// Sort node records into the stable emission order: object type, then name
/// (FR-006-AC-3).
pub fn sort_nodes(nodes: &mut [NodeRecord]) {
    nodes.sort_by(|a, b| {
        a.object_type
            .cmp(&b.object_type)
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// Sort edge records into the stable emission order: source, type, then target.
pub fn sort_edges(edges: &mut [EdgeRecord]) {
    edges.sort_by(|a, b| {
        a.source_ref
            .cmp(&b.source_ref)
            .then_with(|| a.edge_type.cmp(&b.edge_type))
            .then_with(|| a.target_ref.cmp(&b.target_ref))
    });
}

/// Every edge type this library emits (FR-006-AC-4).
pub const EDGE_TYPES: [EdgeType; 6] = [
    EdgeType::Contains,
    EdgeType::Imports,
    EdgeType::Calls,
    EdgeType::ImplementsTrait,
    EdgeType::Extends,
    EdgeType::References,
];

/// Every provenance reason this library emits.
pub const REASONS: [Reason; 6] = [
    Reason::Syntactic,
    Reason::PathResolved,
    Reason::NameMatch,
    Reason::ImportScoped,
    Reason::ReceiverTyped,
    Reason::ExplicitMention,
];

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(name: &str, start: u32) -> CodeFact {
        CodeFact {
            object_type: ObjectType::Function,
            kind: "function",
            qualified_name: name.to_string(),
            simple_name: "f".to_string(),
            path: "src/lib.rs".to_string(),
            span: LineSpan {
                start,
                end: start + 2,
            },
            parent: None,
            visibility: Visibility::Public,
            signature: Some("() -> ()".to_string()),
        }
    }

    // TC-086 — FR-009-AC-9 / FR-009-CON-1: the two fields are additive. A
    // record serialized before FR-009 carries neither, and must still read
    // back — as `public` (the pre-FR-009 assumption that every declaration is
    // an export) with no signature.
    #[test]
    fn records_without_visibility_or_signature_still_deserialize() {
        let legacy = r#"{
            "id": "abc",
            "reference": "ix://agent-ix/repo/src/lib.rs::f",
            "object_type": "code_function",
            "name": "agent-ix/repo/src/lib.rs::f",
            "data": {
                "kind": "function",
                "path": "src/lib.rs",
                "span": { "start": 1, "end": 3 }
            }
        }"#;
        let record: NodeRecord =
            serde_json::from_str(legacy).expect("a pre-FR-009 record still deserializes");
        assert_eq!(record.data.visibility, Visibility::Public);
        assert_eq!(record.data.signature, None);

        // And a record with no signature does not serialize the key at all,
        // so a consumer reading the old shape sees the old shape.
        let round_tripped = serde_json::to_string(&record).expect("serializes");
        assert!(
            !round_tripped.contains("signature"),
            "an absent signature must not add a key: {round_tripped}"
        );
        assert!(round_tripped.contains("\"visibility\":\"public\""));
    }

    // TC-033 — FR-006-AC-1: records carry a hex id, an ix:// ref and a kind.
    #[test]
    fn node_records_carry_id_reference_and_kind() {
        let record = node_record(&fact("agent-ix/repo/src/lib.rs::f", 1));
        assert_eq!(record.id.len(), 64);
        assert!(record
            .id
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        assert_eq!(record.reference, "ix://agent-ix/repo/src/lib.rs::f");
        assert_eq!(record.object_type, "code_function");
        assert_eq!(record.data.kind, "function");
    }

    // TC-034 — FR-006-AC-2: moving a declaration preserves its id.
    #[test]
    fn moving_a_declaration_preserves_its_id() {
        let before = node_record(&fact("agent-ix/repo/src/lib.rs::f", 10));
        let after = node_record(&fact("agent-ix/repo/src/lib.rs::f", 300));
        assert_eq!(before.id, after.id);
        assert_ne!(before.data.span, after.data.span);
    }

    // TC-087 — FR-006-CON-1: node identity is a pure function of
    // `(object_type, qualified_name)`, so the two components cannot alias.
    #[test]
    fn identity_separates_type_from_name() {
        // Without a separator, ("code_type", "x") and ("code", "typex") would
        // hash the same bytes.
        assert_ne!(
            node_id(ObjectType::Type, "x"),
            node_id(ObjectType::Function, "x")
        );
    }

    // TC-035 — FR-006-AC-3: records appear in stable order.
    #[test]
    fn records_sort_into_stable_order() {
        let mut nodes = vec![
            node_record(&fact("agent-ix/repo/src/lib.rs::z", 1)),
            node_record(&fact("agent-ix/repo/src/lib.rs::a", 1)),
        ];
        sort_nodes(&mut nodes);
        assert_eq!(nodes[0].name, "agent-ix/repo/src/lib.rs::a");

        let mut edges = vec![
            EdgeRecord {
                source_ref: "b".into(),
                edge_type: "calls".into(),
                target_ref: "x".into(),
                confidence: 1.0,
                reason: "syntactic".into(),
                evidence: vec![],
                count: 1,
            },
            EdgeRecord {
                source_ref: "a".into(),
                edge_type: "contains".into(),
                target_ref: "y".into(),
                confidence: 1.0,
                reason: "syntactic".into(),
                evidence: vec![],
                count: 1,
            },
        ];
        sort_edges(&mut edges);
        assert_eq!(edges[0].source_ref, "a");
    }

    // TC-036 — FR-006-AC-4: the edge-type set matches the contract exactly.
    #[test]
    fn the_edge_type_set_matches_the_consumer_contract() {
        let names: Vec<_> = EDGE_TYPES.iter().map(|t| t.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "contains",
                "imports",
                "calls",
                "implements_trait",
                "extends",
                "references"
            ]
        );
    }

    // TC-088 — FR-004-CON-1: the `reason` vocabulary matches the set the
    // consumer contract in filament-ide-rs FR-072 reads, value for value.
    #[test]
    fn the_reason_vocabulary_matches_the_consumer_contract() {
        let names: Vec<_> = REASONS.iter().map(|r| r.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "syntactic",
                "path-resolved",
                "name-match",
                "import-scoped",
                "receiver-typed",
                "explicit-mention"
            ]
        );
    }
}
