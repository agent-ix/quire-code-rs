---
id: SR-007
title: "gap analysis of Plan-001 governed graph-quality measurements"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-001-governed-graph-quality/, spec/tests.md TC-108..TC-134, StR-003, US-004, FR-011, FR-012, NFR-005, MP-001, IT-001, src/measurement.rs, src/bin/measure_graph_quality.rs, tests/measurement_pipeline.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/Plan-001
    type: reviews
  - target: ix://agent-ix/quire-code-rs/TM-001
    type: references
---

## Summary

All four task files say `done`, and every TC-108..TC-134 identifier appears in
test code. Semantic requirement-to-test-to-code analysis nevertheless finds the
plan incomplete: the claimed complete-census integration is sampled, critical
producer failure paths are represented only by non-executing tags, and the
governed plan is not semantically enforced.

## Verdict

**FAIL** — completed tasks and 27/27 mechanical tracking tags do not overcome
high-severity semantic gaps in population truth, decision logic, plan
enforcement, and test evidence.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|---|---|---|---|---|
| FND-001 | high | Task-004 and IT-001 claim a complete real ecosystem path, but its only test passes `--case` twice and never observes the MeasurementPlan's complete unsampled population. | Task-004, IT-001, MP-001, tests/measurement_pipeline.rs:94-100, tests/measurement_pipeline.rs:141-147, TC-111, TC-118, TC-124, TC-125, TC-129 | implementation-bug-despite-evidence |
| FND-002 | high | FR-012-AC-2 intent does not match code: truth-population node counts and per-dimension unresolved/ambiguous censuses are replaced by producer counts and copied placeholder strata. | FR-012-AC-2, src/measurement.rs:289-320, src/measurement.rs:395-446, TC-108, TC-119 | implementation-bug-despite-evidence |
| FND-003 | high | FR-012-AC-3/AC-4 intent does not match the binary or tests: scorer exit status participates in the decision, while TC-121 contains no positive false-positive case and no process-status assertion. | FR-012-AC-3, FR-012-AC-4, MP-001, src/bin/measure_graph_quality.rs:130-170, src/measurement.rs:714-737, TC-120, TC-121 | implementation-bug-despite-evidence |
| FND-004 | high | FR-012-AC-5/AC-6/AC-8 have tags but no behavioral producer evidence for all non-measured states, each missing pin with empty stdout, or a structurally valid but inactive/drifted MeasurementPlan. | FR-012-AC-5, FR-012-AC-6, FR-012-AC-8, src/measurement.rs:674-689, src/bin/measure_graph_quality.rs:456-468, TC-122, TC-123, TC-125 | correct-requirement-no-evidence |
| FND-005 | high | NFR-005-AC-2 and IT-001 require permuted filesystem creation/enumeration order; the tagged unit test only removes and reinserts one key in a canonical JSON map, and the real lane repeats the same checkout unchanged. | NFR-005-AC-2, IT-001-SC-02, src/measurement.rs:739-765, tests/measurement_pipeline.rs:42-166, TC-124, TC-127 | correct-requirement-no-evidence |
| FND-006 | medium | The public `--case` surface changes the measured population but has no owning requirement that defines subset identity or comparability; it directly conflicts with MP-001's no-sampling rule. | src/bin/measure_graph_quality.rs:151-153, src/bin/measure_graph_quality.rs:295-382, MP-001 | missing-requirement |
| FND-007 | medium | FR-011's validator does not enforce closed language result keys or five-dimension coverage for every result collection, so the engine-agnostic validation surface is weaker than the requirement and its complete matrix row. | FR-011-AC-2, FR-011-AC-6, schemas/graph-quality-observation-v1.schema.json:109-135, src/measurement.rs:190-223, TC-113, TC-117 | implementation-bug-despite-evidence |
| FND-008 | low | Plan-001's four Test Plan and ecosystem checkboxes remain unchecked while all owning tasks and the mapping table say done. | Plan-001:66-82, Task-001, Task-002, Task-003, Task-004 | implementation-bug-despite-evidence |

## Coverage

- Tasks done: 4 / 4 (frontmatter), but semantic completion failed.
- Matrix Test Cases backed by a tracking tag: 27 / 27 for TC-108..TC-134.
- Matrix Test Cases with material semantic false confidence: 10 identified
  directly (TC-111, TC-118, TC-119, TC-121..TC-125, TC-127, TC-129 overlap
  across findings).
- Untraced behaviors: 1 material user-visible surface (`--case`).
- Production stubs: 0.
- Semantic review: ran over StR-003, US-004, FR-011, FR-012, NFR-005, MP-001,
  IT-001, all four tasks, and TC-108..TC-134.

## Execution Evidence

- Ordinary suite: 145 passed, 2 ignored; the issue-7 real integration test is
  excluded from `make test` by default.
- Explicit clean-clone real integration: 1 passed against installed Quire and
  Quoin, proving envelope compatibility but only for its two selected cases.
- Full quire-corpus scorer: 114 cases, 320 TP, 1 pending node FP, 0 FN; this is
  not the population used by the passing integration lane.
- Quire spec/plan structural validation: pass on the current working tree.
- Coverage command: blocked by two installed-catalog archetypes that match no
  document; this does not alter the 27/27 direct tag index above.
