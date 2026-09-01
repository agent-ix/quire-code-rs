---
id: Task-004
title: "Real ecosystem integration"
type: Task
status: done
track: C
priority: P0
relationships:
  - target: ix://agent-ix/quire-code-rs/Task-003
    type: depends_on
  - target: ix://agent-ix/quire-code-rs/IT-001
    type: references
  - target: ix://agent-ix/quire-code-rs/TC-111
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-118
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-124
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-125
    type: verifies
  - target: ix://agent-ix/quire-code-rs/TC-129
    type: verifies
---

# Task-004: Real ecosystem integration

## Scope

Run the release extractor through the real corpus scorer, validate the plan with
Quire, and record/report the result with a real Quoin CLI and temporary store.

## Subtasks

- [x] Add a gated real-dependency integration harness with no mocked I/O or subprocess.
- [x] Prove Quoin retains and renders every dimension plus raw evidence identity.

## Deliverables

- Real three-project integration test and reproducible invocation documentation

## Notes

- Local paths are test inputs only and never enter emitted evidence.
