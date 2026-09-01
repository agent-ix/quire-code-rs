---
id: FR-012
title: "Governed graph-quality measurement producer"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-004"
    type: "implements"
  - target: "ix://agent-ix/quire-code-rs/FR-011"
    type: "depends_on"
  - target: "ix://agent-ix/quire-code-rs/NFR-004"
    type: "references"
---

# [FR-012] Governed graph-quality measurement producer

## Description

When measurement begins with a pinned corpus and MeasurementPlan, the
graph-quality producer SHALL emit one canonical graph-quality observation.

## Inputs

- A local quire-corpus root with a revision-pinned population manifest and truth
  records.
- The `extract_tree` producer and its declared producer-contract version.
- Exact parser grammar revisions and a canonical extraction-configuration digest.
- Source, extractor, scorer, and corpus revisions.
- Active [MP-001](../assurance/MP-001-graph-quality-observation.md).
- An output directory for raw scorer output and the canonical observation.

## Outputs

- One Quoin MeasurementCollection v2 on standard output. Its `rawEvidence`
  contains the schema-valid graph-quality observation and the complete parsed
  scorer report; its typed `observations` expose precision, recall, unresolved,
  and ambiguous values without transcribing away their dimensions.
- Raw scorer output at the relative path and digest named by the observation.
- Non-zero process status when the population is not measured, validation fails,
  or a wrong heuristic edge violates the decision rule.

## Behavior

- When scoring begins, the producer SHALL validate MP-001 with Quire.
- The producer SHALL invoke the corpus's real `score.py --json` scorer with the
  real release `extract_tree` binary; neither scorer truth comparison nor
  extractor behavior may be reimplemented in the measurement producer.
- The producer SHALL match MP-001's definition version to the observation
  contract.
- For a supported non-empty readable population, the producer SHALL compute the
  exact census, confusion matrices, unresolved census, ambiguous census, and
  recall by language, node kind, relation kind, and resolver tier.
- The producer SHALL retain aggregate recall independently from the
  zero-wrong-heuristic-edge decision.
- If the corpus truth set contains no match for an emitted heuristic edge, then
  the producer SHALL record the false positive.
- If any false-positive heuristic edge is recorded, then the producer SHALL exit
  non-zero.
- If the declared population is empty, then the producer SHALL emit an `empty`
  record without results and exit non-zero.
- If any supported population file is unreadable, then the producer SHALL emit
  an `unreadable` record without results and exit non-zero.
- If the population contains no supported source file, then the producer SHALL
  emit an `unsupported` record without results and exit non-zero.
- If the producer receives an incomplete revision tuple or configuration
  identity, then the producer SHALL fail without an observation using a non-zero
  status that names the incomplete field.
- When an observation is ready for emission, the producer SHALL validate it
  against the versioned schema.
- The producer SHALL retain the raw scorer output used to derive the observation.
- The producer SHALL write dimension collections in canonical sorted order.
- The producer SHALL derive the observation identifier from canonical content.
- The producer SHALL use a caller-supplied pinned collection timestamp and
  toolchain identities so Quoin's required envelope remains deterministic.
- The producer SHALL emit a Quoin v2 verification-stack attestation containing
  full clean source revisions, release executable and lock digests, pinned
  Node/Rust/Python identities, and content digests for the schema, plan,
  configuration, and retained raw scorer output.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-012-CON-1 | Recall SHALL remain a reported observation with no threshold that can compensate for or relax a false-positive heuristic edge. | Integrity | Static Test |
| FR-012-CON-2 | The producer SHALL use local filesystem inputs only. | Security | Static Test |
| FR-012-CON-3 | The producer SHALL initiate no network request. | Security | Static Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-012-AC-1 | A supported non-empty corpus emits one schema-valid observation referencing active MP-001 and retained raw scorer output. | Test (TC-118) |
| FR-012-AC-2 | The measured observation contains exact population, confusion-matrix, unresolved, ambiguous, and recall entries for all four required dimensions. | Test (TC-119) |
| FR-012-AC-3 | Zero false-positive heuristic edges passes the precision decision while observed recall is retained independently, including recall below one. | Test (TC-120) |
| FR-012-AC-4 | A false-positive heuristic edge is retained in its matrix and makes the producer exit non-zero regardless of recall. | Test (TC-121) |
| FR-012-AC-5 | Empty, unreadable, and unsupported fixtures emit their respective non-measured state, contain no results, and exit non-zero. | Test (TC-122) |
| FR-012-AC-6 | Missing required revision or configuration identities produce no observation, exit non-zero, and name the missing input. | Test (TC-123) |
| FR-012-AC-7 | Two repetitions with pinned inputs produce byte-identical canonical observations and matching raw-output digests. | Test (TC-124) |
| FR-012-AC-8 | Invalid MeasurementPlan or observation data is rejected before a record is offered to Quoin. | Test (TC-125) |

## Dependencies

- **Upstream**: [FR-010](./FR-010-producer-invocation.md) supplies canonical
  extractor output, [FR-011](./FR-011-graph-quality-observation-schema.md)
  supplies the record contract, [NFR-004](../non-functional/NFR-004-conservative-resolution-precision.md)
  supplies the precision invariant, and
  [MP-001](../assurance/MP-001-graph-quality-observation.md) governs collection
  and interpretation.
- **Downstream**: Quoin validates, retains, and reports the emitted observation.
