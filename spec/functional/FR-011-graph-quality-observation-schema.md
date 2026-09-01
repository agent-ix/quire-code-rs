---
id: FR-011
title: "Versioned graph-quality observation schema"
type: FR
object: data_schema
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-004"
    type: "implements"
  - target: "ix://agent-ix/quire-code-rs/FR-010"
    type: "references"
---

# [FR-011] Versioned graph-quality observation schema

## Description

A graph-quality observation SHALL validate against the engine-agnostic JSON
Schema at `schemas/graph-quality-observation-v1.schema.json`. That checked-in
draft-2020-12 file is the single executable schema; this requirement defines
the semantic invariants enforced in addition to its structural rules.

## Schema

The archetype-level schema declaration points to the checked-in executable
artifact rather than restating it:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$ref": "../../../schemas/graph-quality-observation-v1.schema.json"
}
```

## Schema Contract

- `schema_version` is `1` and `record_type` is
  `graph_quality_observation`.
- `observation_id` is the SHA-256 digest of canonical record bytes with the
  identifier field omitted.
- `producer` pins the extractor and source revisions, producer-contract
  version, exactly the Python, Rust, TSX, and TypeScript parser grammar
  identities, configuration digest, corpus revision, and scorer revision.
- `measurement_plan` names `ix://agent-ix/quire-code-rs/MP-001` and definition
  `quire-code.graph-quality-v1`.
- `population` carries state, file counts, and sorted censuses for languages,
  node kinds, relation kinds, and resolver tiers.
- A measured record carries sorted confusion matrices, unresolved and ambiguous
  counts, and recall for the overall population and the language, node-kind,
  relation-kind, and resolver-tier dimensions. The scorer defines no
  true-negative population, so `true_negative` is `null`; the producer never
  invents a zero.
- `raw_scorer_output` contains a relative non-parent-traversing path and the
  SHA-256 digest of the exact retained scorer bytes.
- Every object rejects unknown fields. Language names are closed to `rust`,
  `typescript`, `tsx`, `python`, and `mixed`; dimension and population-state
  names are also closed by the schema.

## Behavior

- A measured record SHALL contain one complete population census and a results
  block.
- An `empty`, `unreadable`, or `unsupported` record SHALL omit results rather
  than encode unmeasured work as a zero.
- Each dimension array SHALL cover all five dimension names, use only the
  closed language vocabulary for a `language` key, and be sorted by dimension
  and key without duplicates.
- The producer SHALL run schema validation before placing the raw observation
  and scorer report inside Quoin MeasurementCollection v2 `rawEvidence`.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-011-CON-1 | The record SHALL remain engine-agnostic JSON data. | Compatibility | Static Test |
| FR-011-CON-2 | The record SHALL require no extractor library to parse or validate it. | Compatibility | Static Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-011-AC-1 | A measured record with a supported file and complete results validates against schema version 1. | Test (TC-112) |
| FR-011-AC-2 | A measured record contains census and result entries for language, node kind, relation kind, and resolver tier plus an overall result. | Test (TC-113) |
| FR-011-AC-3 | `empty`, `unreadable`, and `unsupported` records reject a results block, while `measured` rejects its absence. | Test (TC-114) |
| FR-011-AC-4 | Missing or malformed producer revisions, grammar identities, configuration digest, or measurement-plan identity fail validation. | Test (TC-115) |
| FR-011-AC-5 | Every record retains a relative raw-output path and SHA-256 digest; an absolute or parent-traversing path or malformed digest fails validation. | Test (TC-116) |
| FR-011-AC-6 | Unknown fields, dimension names, population states, or language names fail validation. | Test (TC-117) |

## Dependencies

- **Upstream**: [US-004](../usecase/US-004-assess-versioned-extractor-quality.md)
  and [FR-010](./FR-010-producer-invocation.md).
- **Downstream**: [FR-012](./FR-012-governed-graph-quality-producer.md) emits this
  record inside a Quoin MeasurementCollection v2.
