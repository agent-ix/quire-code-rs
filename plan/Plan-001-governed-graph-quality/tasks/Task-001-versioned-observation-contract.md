---
id: Task-001
title: "Versioned observation contract"
type: Task
status: in_progress
track: A
priority: P0
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-011
    type: references
  - target: ix://agent-ix/quire-code-rs/TC-112
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-113
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-114
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-115
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-116
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-117
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-130
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-131
    type: verifies
---

# Task-001: Versioned observation contract

## Scope

Add the executable draft-2020-12 schema, typed records, canonical serialization,
content identity, and validation needed before any process orchestration.

## Subtasks

- [ ] Check in and compile the schema without HTTP/file resolvers.
- [ ] Define closed population, dimension, provenance, and result types.
- [ ] Validate semantic invariants JSON Schema cannot express alone.

## Deliverables

- `schemas/graph-quality-observation-v1.schema.json`
- `src/measurement.rs`
- Contract-focused tests with matrix tracking tags

## Notes

- True-negative remains null when the scorer owns no negative population.

