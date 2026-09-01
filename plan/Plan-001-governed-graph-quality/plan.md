---
id: Plan-001
title: "Governed graph-quality observations"
type: Plan
status: active
relationships:
  - target: ix://agent-ix/quire-code-rs/StR-003
    type: references
  - target: ix://agent-ix/quire-code-rs/US-004
    type: references
  - target: ix://agent-ix/quire-code-rs/FR-011
    type: references
  - target: ix://agent-ix/quire-code-rs/FR-012
    type: references
  - target: ix://agent-ix/quire-code-rs/NFR-005
    type: references
  - target: ix://agent-ix/quire-code-rs/MP-001
    type: references
  - target: ix://agent-ix/quire-code-rs/IT-001
    type: references
---

# Implementation Plan: Governed graph-quality observations

## Requirements Summary

### Stakeholder and user requirements

- [x] **StR-003 / US-004**: turn extractor quality into inspectable governed evidence without converting absence into success.

### Functional requirements

- [x] **FR-011**: ship a strict, versioned, engine-agnostic raw-observation schema.
- [x] **FR-012**: run the real scorer/extractor and emit a Quoin v2 collection with retained raw evidence.

### Non-functional and governance requirements

- [x] **NFR-005**: identical pinned inputs produce identical canonical bytes.
- [x] **MP-001 / IT-001**: apply the zero-wrong-edge decision and prove real Quire/Quoin acceptance.

## Dependency Graph

### Core dependency edges

- `FR-011 -> FR-012` — the producer cannot emit or validate a record before its contract is executable.
- `FR-012 -> NFR-005` — repeatability and failure-state tests exercise the completed producer boundary.
- `FR-011 + FR-012 + NFR-005 + MP-001 -> IT-001` — ecosystem intake is meaningful only after schema, producer, decision, and determinism gates pass.

### Shared dependencies

- Canonical JSON, SHA-256 identity, closed dimension vocabularies, and typed population states are shared by raw evidence, Quoin observations, and repeatability tests.
- The scorer remains the single truth comparator; the producer only validates, preserves, and maps its output.

### Cross-cutting constraints

- Filesystem and child-process access stay in the binary boundary; transformation and validation remain pure library code.
- No network-capable dependency may enter the default feature closure.

### The seams

`src/measurement.rs` owns pure typed transformation and schema validation.
`src/bin/measure_graph_quality.rs` owns local file/process orchestration beside
the existing `extract_tree` binary. Quoin's collection v2 shape is emitted
directly; its rawEvidence retains the custom graph observation and scorer JSON.

## Test Plan

### Contract and transformation tests

- [x] **TC-108..TC-117, TC-130..TC-131**: schema-valid measured records, dimension coverage, closed vocabularies, strict provenance, raw-output references, and extractor-independent validation.

### Producer and failure-domain tests

- [x] **TC-118..TC-123, TC-132..TC-134**: exact mappings, independent recall/precision, wrong-edge gate, non-measured populations, missing pins, filesystem-only boundary, and no network closure.

### Determinism and governance tests

- [x] **TC-124..TC-129**: repeated and permuted inputs, ambient-field exclusion, invalid-plan rejection, and active MP-001 enforcement.

### Ecosystem integration

- [x] **TC-111, TC-118, TC-124, TC-125, TC-129**: real release extractor plus real quire-corpus scorer, Quire validation, Quoin record, and Quoin report.

## Remaining Work

### Track A: Critical Path (serial)

- **A1 = Task-001** Versioned observation contract — medium; exit: malformed or vocabulary-drifted records fail the executable schema and pure validator.
- **A2 = Task-002** Governed producer — hard; exit: the real scorer is preserved in one deterministic Quoin v2 collection.
- **Gate = Task-003** Failure and determinism gates — measures absence honesty, zero wrong edges, and repeatability; pass: all failure states are non-measured, false positives fail, and pinned repetitions are byte-identical.

### Track C: Post-Gate

- **C1 = Task-004** Real ecosystem integration — hard; exit: Quire validates the plan and Quoin records/reports the emitted collection without an adapter or mock.

## Parallel Execution Summary

`Task-001 -> Task-002 -> Task-003 (gate) -> Task-004`

## Task File Mapping

| Task | Track | Owns (references) | Verified by (verifies) | Status |
|---|---|---|---|---|
| Task-001 | A | FR-011 | TC-112..TC-117, TC-130..TC-131 | done |
| Task-002 | A | FR-012 | TC-108..TC-111, TC-118..TC-123, TC-132..TC-134 | done |
| Task-003 | Gate | NFR-005, MP-001 | TC-124..TC-129 | done |
| Task-004 | C | IT-001 | TC-111, TC-118, TC-124, TC-125, TC-129 | done |

## Coordination Rules

The JSON schema is frozen before producer mapping begins. The binary never
reimplements scorer truth logic. Real ecosystem intake starts only after the
local precision and byte-repeatability gate is green.
