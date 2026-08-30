---
id: NFR-005
title: "Pinned quality observations are byte-identical"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quire-code-rs/FR-012"
    type: "constrains"
  - target: "ix://agent-ix/quire-code-rs/StR-003"
    type: "traces_to"
---

# [NFR-005] Pinned quality observations are byte-identical

## Statement

The governed measurement producer SHALL emit byte-identical canonical observation
records for identical pinned inputs regardless of filesystem enumeration order.

## Scope

- Applies to measured and non-measured observation states.
- Covers record ordering, observation identity, census ordering, result ordering,
  relative raw-output references, and serialized bytes.
- Excludes storage paths chosen later by Quoin.

## Rationale

Repeated observations are comparable only if stable inputs produce stable bytes.
Timestamps, absolute paths, host identity, unordered maps, or random identifiers
would manufacture change where none occurred and make the record unsuitable for
content-addressed evidence.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Distinct canonical record byte sequences across two pinned repetitions | 1 | 1 | Test |
| Distinct observation identifiers across two pinned repetitions | 1 | 1 | Test |
| Host-specific or run-time fields in the versioned record | 0 | 0 | Analysis |

## Verification

Run the producer twice over the same pinned corpus after permuting filesystem
creation order. Compare canonical bytes and observation identifiers, then inspect
the schema and serialized output for timestamps, absolute paths, host names,
process identifiers, and random values.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-005-AC-1 | Two repetitions with identical pinned inputs emit byte-identical canonical observation records. | Test (TC-126) |
| NFR-005-AC-2 | Permuting filesystem creation and enumeration order does not change the canonical record or observation identifier. | Test (TC-127) |
| NFR-005-AC-3 | The versioned record contains no timestamp, absolute path, host name, PID, or random value. | Test (TC-128) |

## Dependencies

- **Upstream**: [FR-012](../functional/FR-012-governed-graph-quality-producer.md).
- **Downstream**: Quoin may content-address and compare the emitted records.
