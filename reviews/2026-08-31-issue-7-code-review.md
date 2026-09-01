---
id: SR-006
title: "code review of issue 7 governed graph-quality measurements"
type: SpecReview
analysis: code-review
scope: "origin/main...working tree: FR-011, FR-012, NFR-005, MP-001, IT-001, src/measurement.rs, src/bin/measure_graph_quality.rs, schemas/, tests/measurement_pipeline.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/Plan-001
    type: reviews
  - target: ix://agent-ix/quire-code-rs/TM-001
    type: references
---

## Summary

The Quoin MeasurementCollection v2 envelope is accepted by the real installed
Quoin CLI, and the ordinary Rust tests are green. The implementation is not
merge-ready: its passing ecosystem lane samples two cases while claiming a
complete population, its unresolved/ambiguous strata are synthesized rather
than measured, and its decision/test gates do not enforce the authored plan.

## Verdict

**FAIL** — five high-severity correctness and false-confidence findings violate
the population, decision, governance, and evidence requirements.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|---|---|---|---|---|
| FND-001 | high | The real integration lane scores only two selected Rust cases but records every observation population as complete under a MeasurementPlan that prohibits sampling. The scorer report still identifies the whole corpus revision and all case digests, so the retained evidence can be mistaken for the declared 114-case population. | MP-001:11-17, MP-001:38-45, tests/measurement_pipeline.rs:94-100, tests/measurement_pipeline.rs:141-147, src/measurement.rs:542-548, TC-108, TC-118, TC-119, TC-129 | implementation-bug-despite-evidence |
| FND-002 | high | Population/result censuses are not exact: node census counts produced nodes, both unresolved and ambiguous come from `ambiguous_call_sites`, and node/relation/tier unresolved strata copy the same total into invented `unattributed`, `calls`, and `unresolved` keys. | FR-012-AC-2, src/measurement.rs:289-320, src/measurement.rs:395-446, TC-108, TC-119 | implementation-bug-despite-evidence |
| FND-003 | high | The process decision is not the zero-wrong-heuristic-edge rule. It requires the scorer process to pass (which fails on non-pending false negatives) and sums every false positive, including node/corpus findings, so recall below one can fail and non-heuristic findings can trip the heuristic-edge gate. | MP-001:17, MP-001:29-32, FR-012-AC-3, FR-012-AC-4, src/bin/measure_graph_quality.rs:51-52, src/bin/measure_graph_quality.rs:130-170, TC-120, TC-121 | implementation-bug-despite-evidence |
| FND-004 | high | MeasurementPlan validation is structural only. A Quire-valid retired plan, changed metric, changed definition version, or changed decision rule reaches stdout; the producer never parses or matches those governed fields before offering the record to Quoin. | FR-012:43-48, FR-012-AC-8, src/bin/measure_graph_quality.rs:43, src/bin/measure_graph_quality.rs:173-187, TC-125, TC-129 | implementation-bug-despite-evidence |
| FND-005 | high | Matrix rows marked complete are backed by tags but not by the promised behavior: TC-121 never constructs an FP or runs the exit gate; TC-122 never invokes the binary or checks status/stdout; TC-123 checks only the first missing generic CLI argument; TC-125 supplies no invalid plan; TC-127 reorders a JSON map rather than filesystem creation/enumeration. | src/measurement.rs:674-765, src/bin/measure_graph_quality.rs:456-468, tests/measurement_pipeline.rs:42-181, TC-121, TC-122, TC-123, TC-125, TC-127 | correct-requirement-no-evidence |
| FND-006 | medium | The schema closes grammar/census languages but permits an arbitrary result key when `dimension` is `language`, and semantic validation requires all dimensions only for confusion matrices, not unresolved, ambiguous, or recall. | FR-011-AC-2, FR-011-AC-6, schemas/graph-quality-observation-v1.schema.json:109-135, src/measurement.rs:190-223, TC-113, TC-117 | implementation-bug-despite-evidence |
| FND-007 | medium | The verification stack identifies `measure_graph_quality` as the tool but hashes the separate extractor path as `executableDigest`; it therefore does not attest the executable that created the collection. | FR-012:74-77, src/bin/measure_graph_quality.rs:89-99, src/measurement.rs:244-270, TC-110 | correct-requirement-no-evidence |
| FND-008 | medium | Measured Quoin observations expose TP/FP/FN and recall but no typed precision value or precision-decision observation, despite the producer output contract requiring precision to remain directly exposed. | FR-012:31-39, src/measurement.rs:494-540, TC-108, TC-120 | implementation-bug-despite-evidence |

## Review Evidence

- `make ci`: formatting, Clippy, 145 ordinary tests, license denial, and unsafe
  checks passed; the final coverage target failed because the installed module
  catalog declares two archetypes that match no document.
- `quire validate --scope . 'spec/**/*.md' 'plan/**/*.md'`: passed after the
  current FR-011 schema-link correction, with pre-existing grammar warnings.
- Clean-clone ecosystem lane: one ignored test was explicitly run and passed
  against the real release extractor, quire-corpus, Quire, and Quoin.
- Full real scorer observation: 114 cases, 320 TP, 1 pending node FP, 0 FN. The
  checked-in ecosystem test instead scored two selected Rust cases.

## Boundary and Completeness Review

No production stubs, `todo!`, `unimplemented!`, unsafe blocks, network clients,
or internal-logic mocks were found in the issue-7 diff. Filesystem and process
operations stay at the binary boundary. The failures are semantic: population
identity, exact transformation, governed decision logic, and tests that do not
exercise their claimed behavior.
