---
id: StR-003
title: "Assurance consumers need governed extractor-quality observations"
type: StR
relationships:
  - target: "ix://agent-ix/quire-code-rs/FR-011"
    type: "satisfied_by"
  - target: "ix://agent-ix/quire-code-rs/FR-012"
    type: "satisfied_by"
  - target: "ix://agent-ix/quire-code-rs/NFR-005"
    type: "satisfied_by"
---

# [StR-003] Assurance consumers need governed extractor-quality observations

## Stakeholder Need

Assurance consumers require the extractor to emit versioned quality observations
whose population, truth comparison, unresolved work, ambiguity, raw output, and
producer revisions shall remain independently inspectable, so that a measurement
can inform a decision without turning missing input into a successful zero.

## Rationale

The corpus currently proves extractor behavior inside this repository, but a
downstream assurance system needs more than a passing test. It needs the exact
population that was scored, the revisions that produced it, and raw results that
can be reinterpreted later. Without those records, a zero may mean no wrong edges,
no supported files, an unreadable tree, or a scorer that never ran.

Versioned governed observations make those conditions distinguishable and let
Quoin compare results without importing extractor-specific logic.

## Validation Criteria

| ID | Criteria | Validation |
|----|----------|------------|
| StR-003-VC-1 | A supported non-empty corpus produces dimensioned confusion matrices, unresolved and ambiguous censuses, and an exact population census. | Test (TC-108) |
| StR-003-VC-2 | Empty, unreadable, and unsupported populations produce explicit non-measured states and no metric results. | Test (TC-109) |
| StR-003-VC-3 | Every measured observation pins extractor, grammar, configuration, source, corpus, scorer, and measurement-definition revisions. | Test (TC-110) |
| StR-003-VC-4 | The observation retains raw scorer output and validates against its versioned schema and active MeasurementPlan. | Test (TC-111) |

## Stakeholders

The primary stakeholders are assurance practitioners and Quoin maintainers who
consume extractor-quality evidence. Extractor maintainers are decision owners for
the zero-wrong-edge invariant and the interpretation of recall regressions.

## Context and Assumptions

The shared corpus supplies a revision-pinned truth set and a declared population.
The extractor producer contract is versioned. The observation producer runs
offline against local files and does not own evidence storage or portfolio policy.

## Dependencies

- **Upstream**: [NFR-004](../non-functional/NFR-004-conservative-resolution-precision.md)
  defines the zero-wrong-edge invariant and independent recall reporting.
- **Downstream**: [FR-011](../functional/FR-011-graph-quality-observation-schema.md)
  defines the record contract, and
  [FR-012](../functional/FR-012-governed-graph-quality-producer.md) defines the
  producer behavior.

## Priority and Risk (Informative)

Priority is P0. If unmet, downstream reports can compare incomparable runs or
misread absence as perfect quality.
