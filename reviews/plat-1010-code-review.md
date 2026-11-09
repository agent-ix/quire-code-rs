---
id: SR-16310
title: "code review of the single graph_quality observation (PR #54)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-code-rs@530f194837218d3805e0728c18ef1208c38dbd22; src/measurement.rs, src/bin/measure_graph_quality.rs, tests/measurement_pipeline.rs, schemas/graph-quality-observation-v1.schema.json"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-012
    type: reviews
---

## Summary

Ticket: PLAT-1010. Code review, with the rust-review lane, of
`git diff origin/main...origin/plat-1010-graph-quality`. `quoin_observations`
now returns exactly one `graph_quality` observation per run. On a measured run,
`value` and `population.matched` are both the scorer's
`/confusion/axis_kind/edge/fp`. On a non-measured run it is one `not_computed`
observation with the same `edge_false_positive` dimensions. A missing edge
`tp` or `fp` count is now a typed error instead of a silent zero. The
definition version is `quire-code.graph-quality-v3` in the constant, the
schema `const`, MP-001, FR-011 and the plan-drift test. Focused tests pass
(6 lib measurement tests, 4 producer binary tests), and clippy with
`--features measurement --all-targets -D warnings` is clean.

The orientation against Quoin's checker is correct. Quoin origin/main
`verify::observations_of` selects every observation matching
`planId`/`metric`/`definitionVersion`. `rows::assess` with estimator `count`
recomputes the estimate as `matched` and requires `value` to agree with it.
`eq 0` then holds only for a zero false-positive count, the lower-is-better
reading that `objective.direction: zero` states. But the observation omits
`population.repetitions`, and MP-001 states `repetitions: 2`. Quoin refuses it:
see FND-001.

## Verdict

**FAIL.** FND-001 means Quoin refuses the observation that the decision rule
binds to. `quoin measurement record` (0.29.1) was run on a collection this
producer emitted, and it failed with `QM-POPULATION-UNSTATED ... plan MP-001
requires 2 repetitions`. Quoin's `rows::assess` returns `PopulationUnstated`
before the rule is applied, so the PR's goal, Quoin judging the false-positive
count, is not reached. The other findings do not block.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | high | The single measured observation omits `population.repetitions`, but MP-001 states `statistical_design.repetitions: 2`, so Quoin refuses it before applying the decision rule (`rows::assess` -> `PopulationUnstated`; reproduced with `quoin measurement record`: QM-POPULATION-UNSTATED, plan MP-001 requires 2 repetitions). The producer does run the scorer twice, so it should state `"repetitions": 2`. | src/measurement.rs:490-494 |
| FND-002 | low | `edge_tp + edge_fp` adds two u64 values that come from scorer output without a checked add, so it panics in debug and wraps in release on overflow. Use `checked_add` and return an `InvalidScorerReport` error. | src/measurement.rs:492 |
| FND-003 | low | The edge false-positive count is read twice, by `precision_decision_passed` (exit status) and by `quoin_observations`, with different error text. The exit decision and the typed value could drift apart. Derive both from one reader. The name `precision_decision_passed` also keeps the retired `precision_decision` vocabulary. | src/measurement.rs:411-421, src/measurement.rs:481-492 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-004 | medium | `population.repetitions` is the constant `REPETITIONS = 2`, and its doc says it is "declared, not counted": one binary invocation scores once. Quoin's repetitions check therefore passes whether or not a second run happened, so it guards nothing. FND-001's fix advice rested on a wrong premise: the binary does not score twice. Only the ignored IT test runs it twice and compares the output byte for byte. Two honest fixes: set MP-001 `repetitions: 1` and drop the constant, since determinism is FR-012-AC-7's separate check; or have the producer perform both runs and compare them before stating 2. | src/measurement.rs:21-24, src/measurement.rs:498 |
| FND-005 | low | The doc comment on `edge_false_positive_count` still reads "Evaluate only the governed precision rule". The function now returns the count, not a decision. | src/measurement.rs:413-414 |

## Dispositions

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | Reproduced at 624528f: `quoin measurement record` no longer reports QM-POPULATION-UNSTATED. Only PLAT-1361's envelope rejections remain. How the value is sourced is a new finding, FND-004. |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
