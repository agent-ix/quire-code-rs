---
id: FR-004
title: "Edge provenance vocabulary and deduplication"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-003"
    type: "implements"
  - target: "ix://agent-ix/filament-ide-rs/FR-072"
    type: "references"
---

# [FR-004] Edge provenance vocabulary and deduplication

## Description

The library SHALL attach to every emitted edge a provenance record carrying
`confidence`, `reason`, `evidence` and `count`, and SHALL emit at most one edge
per distinct `(source_ref, edge_type, target_ref)` triple, folding every
contributing site into that single edge's evidence and count.

## Inputs

- Candidate edges produced by [FR-003](./FR-003-structural-edges.md),
  [FR-005](./FR-005-spec-mention-harvesting.md) and
  [FR-008](./FR-008-type-environments-and-call-resolution.md)
- The file and one-based line of each contributing site

## Outputs

- `confidence`, a float in the closed interval [0.0, 1.0]
- `reason`, exactly one of `syntactic`, `path-resolved`, `name-match`,
  `import-scoped`, `receiver-typed`, `explicit-mention`
- `evidence`, an ordered array of `{file, line}` entries, at most 20
- `count`, the total number of contributing sites, including those beyond the
  evidence cap

## Behavior

- The library SHALL deduplicate edges on the triple
  `(source_ref, edge_type, target_ref)`, and SHALL NOT treat two edges differing
  only in provenance as distinct.
- When several sites contribute to one deduplicated edge, the library SHALL set
  `count` to the total number of contributing sites and SHALL retain the first
  20 evidence entries in source order, discarding the remainder.
- Where contributing sites carry different confidences, the library SHALL retain
  the highest confidence and the `reason` that produced it, so that a
  well-resolved site is not degraded by a weaker one for the same relationship.
- The library SHALL assign confidence 1.0 to `reason` values `syntactic`,
  `path-resolved` and `explicit-mention`, since each is read directly from
  source text rather than inferred.
- The library SHALL order evidence entries by file path and then by line, so
  that the array is stable across runs.
- The library SHALL NOT emit an edge whose `confidence` falls outside [0.0, 1.0]
  or whose `reason` lies outside the enumerated set.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-004-CON-1 | The `reason` vocabulary SHALL match the set consumed by `ix://agent-ix/filament-ide-rs/FR-072` exactly; adding a value is a coordinated contract change | Interface | Test (TC-088) |
| FR-004-CON-2 | The evidence array SHALL never exceed 20 entries regardless of contributing site count | Performance | Test (TC-021) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-004-AC-1 | Two call sites producing the same triple yield one edge with `count` 2 and two evidence entries | Test (TC-020) |
| FR-004-AC-2 | Thirty contributing sites yield `count` 30 and exactly 20 evidence entries | Test (TC-021) |
| FR-004-AC-3 | Sites of differing confidence for one triple yield the highest confidence and its matching `reason` | Test (TC-022) |
| FR-004-AC-4 | Every emitted edge carries a `reason` drawn from the enumerated set and a confidence within [0.0, 1.0] | Test (TC-023) |
| FR-004-AC-5 | Evidence entries are ordered by file then line across repeated runs | Test (TC-024) |
| FR-004-AC-6 | Edges whose `reason` is `syntactic`, `path-resolved` or `explicit-mention` carry confidence 1.0 | Test (TC-025) |

## Dependencies

- **Upstream**: all edge-producing requirements
- **Downstream**: [FR-006](./FR-006-canonical-record-emission.md) serializes
  provenance; `ix://agent-ix/filament-ide-rs/FR-072` consumes it
