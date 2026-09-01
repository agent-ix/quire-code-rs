---
id: Task-002
title: "Governed graph-quality producer"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: ix://agent-ix/quire-code-rs/Task-001
    type: depends_on
  - target: ix://agent-ix/quire-code-rs/FR-012
    type: references
  - target: ix://agent-ix/quire-code-rs/TC-108
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-118
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-119
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-120
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-121
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-122
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-123
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-132
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-133
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-134
    type: verifies
---

# Task-002: Governed graph-quality producer

## Scope

Implement the filesystem/process binary boundary, scorer-report mapping, raw
retention, and Quoin v2 envelope.

## Subtasks

- [x] Parse and validate every explicit revision, digest, grammar, timestamp, and toolchain pin.
- [x] Validate MP-001 with Quire and invoke real score.py with the release extractor.
- [x] Retain raw bytes and derive typed dimensioned Quoin observations.

## Deliverables

- `src/bin/measure_graph_quality.rs`
- CLI and transformation integration tests

## Notes

- The scorer is authoritative; no truth comparison is duplicated here.
