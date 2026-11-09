---
id: SR-16312
title: "spec review of MP-001, FR-011, FR-012 and IT-001 edits (PR #54)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-code-rs@530f194837218d3805e0728c18ef1208c38dbd22; spec/assurance/MP-001-graph-quality-observation.md, spec/functional/FR-011-graph-quality-observation-schema.md, spec/functional/FR-012-governed-graph-quality-producer.md, spec/integration/IT-001-corpus-observation-to-quoin.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/MP-001
    type: reviews
---

## Summary

Ticket: PLAT-1010. Spec review (integrity sub-analysis) of the four edited
specs. The MP-001 Decision Rule now names one observation: the `graph_quality`
observation with `dimensions.measure: edge_false_positive`, `dimension:
overall` and `key: overall`, whose `value` and `population.matched` equal the
count. It also states that Quoin applies the rule to every observation sharing
the plan's metric, id and definition version. That matches
`verify::observations_of` on quoin origin/main. With `estimator: count`, Quoin
recomputes the estimate from `matched`, so `eq 0` reads the false-positive
count with lower being better. `definition_version` is v3 in MP-001, FR-011,
the schema, the code and the tests. No `precision_decision` remains in spec/,
schemas/ or tests/. The only matches are the Rust function
`precision_decision_passed` (SR-16310 FND-003). The historical reviews/ and
spec/reviews/ files are untouched.

FR-011 fails `quire validate` (quire 0.36.2) with "required 'id' ... missing"
and "semantic.record-invalid ... fields". Main's FR-011, validated in place,
fails with the same two errors, so the failure predates this PR. MP-001,
FR-012 and IT-001 validate.

## Verdict

**PASS with findings.** The binding is unambiguous and its orientation is
correct. The plan does not state the population fields Quoin needs in order to
judge the rule. That gap is the spec side of SR-16310 FND-001.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | The Decision Rule names the bound observation's dimensions, `value` and `matched`, but not the other population fields Quoin requires before it applies the rule: `population.repetitions` (>= the plan's `repetitions: 2`), `complete: true`, and `examined > 0`. A producer that follows the text (as this one does) has its observation refused, not judged. State them in the rule. Also say that a census with zero heuristic edges (`examined == 0`) is reported as an empty population by Quoin, not as a pass. | spec/assurance/MP-001-graph-quality-observation.md:60-72 |
| FND-002 | low | The Decision Rule says precision "stays in `rawEvidence`", but no precision value is retained anywhere now. The FR-011 record carries confusion matrices, recall, unresolved and ambiguous, from which precision is only derivable. Reword to "confusion matrices (from which precision is derivable)". | spec/assurance/MP-001-graph-quality-observation.md:68-70 |
| FND-003 | low | FR-012-AC-3 still says "passes the precision decision", and FR-012-CON-1 says recall "SHALL remain a reported observation". After this PR, "observation" means two things in FR-012: the FR-011 record and Quoin's typed observations. Recall is in the record only. Name the edge-false-positive decision and qualify where recall is reported. | spec/functional/FR-012-governed-graph-quality-producer.md:84-96 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-004 | low | IT-001-SC-04 now writes a current defect into the scenario ("Currently the ignored pipeline test is known to fail at this step"), and it cites the sub-ticket by description, not as PLAT-1361. A scenario should state the target: Quoin accepts and retains the collection. The known failure belongs in the `#[ignore]` reason and in PLAT-1361. | spec/integration/IT-001-corpus-observation-to-quoin.md:59-62 |

## Dispositions

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
