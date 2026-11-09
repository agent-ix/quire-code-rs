---
id: SR-16311
title: "gap analysis of the single graph_quality observation (PR #54)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-code-rs@530f194837218d3805e0728c18ef1208c38dbd22; src/measurement.rs, src/bin/measure_graph_quality.rs, tests/measurement_pipeline.rs, spec/functional/FR-012-governed-graph-quality-producer.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-012
    type: reviews
---

## Summary

Ticket: PLAT-1010. Planless gap analysis scoped to the PR diff. Plan
completion: not assessed. Two tests back the new single-observation behaviour.
`collection_exposes_one_edge_false_positive_count_and_named_artifacts` checks
the count, `value == matched` and `examined` at fp 0 and at fp 2.
`binary_pipeline_gates_only_on_false_positive_heuristic_edges` checks the count
on the binary path. `real_corpus_observation_is_accepted_and_rendered_by_quoin`
is an ignored test that checks the same thing against the real corpus. All of
these are tagged to existing criteria (FR-012-AC-3/4, StR-003-VC-1..3). None of
them is tagged to a criterion that states the contract the PR introduces.

## Verdict

**PASS with gaps.** The measured path is backed by tests. The not_computed path
and the "exactly one observation" contract have no assertion or no criterion.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | The rewritten `not_computed` observation has no test asserting it (`measure: edge_false_positive`, `unit: count`, `value: null`, exactly one observation). `binary_pipeline_emits_each_non_measured_state_without_results` checks only `rawEvidence`, never `collection.observations`. | src/measurement.rs:466-473, src/bin/measure_graph_quality.rs:600-622 |
| FND-002 | medium | FR-012 Outputs now states that the typed `observations` hold exactly one `graph_quality` observation whose value is the edge false-positive count, but no FR-012 acceptance criterion states it, so the Test Matrix cannot show it as covered. Add an AC: exactly one observation, `value == population.matched ==` the edge FP count, and one `not_computed` observation when the population is not measured. Tag the tests above to it. | spec/functional/FR-012-governed-graph-quality-producer.md:31-35 |
| FND-003 | medium | IT-001-SC-04 (Quoin accepts the record) cannot pass against installed quoin 0.29.1. Besides the missing `population.repetitions` (SR-16310 FND-001), intake rejects the collection envelope: no `configDigest` or `sourceRevision`, `verificationStack.lockDigest`/`executableDigest` are not sha256 values, sources are not full-SHA, and `artifacts` is an array, not an object. The envelope code predates this PR, but the edited ignored IT test still asserts that record succeeds. | src/measurement.rs:210-247, tests/measurement_pipeline.rs:262-320 |

## Dispositions

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | deferred | PLAT-1361 owns the envelope rejections that predate this PR. The repetitions part is fixed (see SR-16310 FND-001). |
