---
id: Task-003
title: "Failure and determinism gates"
type: Task
status: done
track: Gate
priority: P0
relationships:
  - target: ix://agent-ix/quire-code-rs/Task-002
    type: depends_on
  - target: ix://agent-ix/quire-code-rs/NFR-005
    type: references
  - target: ix://agent-ix/quire-code-rs/MP-001
    type: references
  - target: ix://agent-ix/quire-code-rs/TC-124
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-125
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-126
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-127
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-128
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-129
    type: verifies
---

# Task-003: Failure and determinism gates

## Scope

Prove non-measured states, wrong-edge gating, pinned repeatability, and active
plan enforcement before external intake.

## Subtasks

- [x] Exercise empty, unsupported, unreadable, malformed-pin, and invalid-plan cases.
- [x] Prove false positives fail independently from recall.
- [x] Prove reordered identical populations serialize byte-for-byte identically.

## Deliverables

- Property-style regression cases and deterministic golden output

## Notes

- This is the quality gate for Task-004.
