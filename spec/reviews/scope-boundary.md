---
id: SR-003
title: "scope-boundary review of the quire-code-rs extraction contract"
type: SpecReview
analysis: scope-boundary
scope: "spec/spec.md, spec/functional/FR-001..008, spec/non-functional/NFR-002"
review_set: subset
---

## Summary

Scope-boundary analysis of the split between this library and its consumers.
The boundary is stated explicitly in the master spec and enforced by
requirement-level constraints rather than by convention alone. Two allocation
gaps were found: mention resolution is divided between the two sides without the
handoff shape being defined, and the batch-composition contract that cross-file
resolution depends on is assumed rather than required.

## Findings

| ID      | Severity | Summary                                                                                                                                       | Refs |
| ------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ---- |
| FND-001 | medium   | The library reports mentions and the consumer resolves them, but neither side's spec defines the reference shape carried across that handoff    | FR-005-CON-1, `ix://agent-ix/filament-ide-rs/FR-075` |
| FND-002 | medium   | Cross-file resolution quality depends entirely on what the consumer chooses to put in a batch, and no requirement states that obligation        | FR-008, ADR-002 |
| FND-003 | low      | Language selection is stated as an input without a requirement governing how a consumer determines it or what happens on a mismatch             | FR-001 |

## Responsibility Allocation

| Concern | Owner | Basis |
|---|---|---|
| Parsing and grammar dependencies | This library | [FR-001](../functional/FR-001-structural-fact-model.md), FR-001-CON-1 |
| Symbol naming and identity | This library | [FR-002](../functional/FR-002-symbol-identity.md) |
| Call resolution and its confidence | This library | [FR-008](../functional/FR-008-type-environments-and-call-resolution.md) |
| Deciding which files exist and are in scope | Consumer | `spec.md` §2.2, `ix://agent-ix/filament-ide-rs/FR-071` |
| Resolving a mention to a spec artifact | Consumer | FR-005-CON-1, `ix://agent-ix/filament-ide-rs/FR-075` |
| Persistence, layering, migration | Consumer | ADR-002, `ix://agent-ix/filament-ide-rs/FR-073` |
| Watching, tiering, incremental scheduling | Consumer | `spec.md` §2.2 |

The allocation is coherent: everything requiring a parser lives here, and
everything requiring the graph lives with the consumer. The one place the line
runs through the middle of a capability is mention resolution, which is
deliberate — recognizing `FR-002` in a comment needs the parser, and resolving it
to an artifact needs the index — but a split capability needs its handoff
defined, which FND-001 records.

## Boundary Enforcement

The boundary is not merely described. [NFR-002](../non-functional/NFR-002-filesystem-only-boundary.md)
makes "reaches nothing outside its inputs" a measurable property of the resolved
dependency closure, and FR-001-CON-1 confines the C FFI surface to dependencies.
Together these make a boundary violation detectable in CI rather than only in
review, which is what the consumer's own dependency-audit criterion
(`FR-072-AC-5`) ultimately rests on.

## Interface Stability

Two constraints — FR-004-CON-1 on the `reason` vocabulary and FR-006-CON-2 on
the edge-type set — declare their vocabularies shared with the consumer and any
change coordinated. These are the correct two to pin: they are the only values
that cross the process boundary as data whose meaning both sides interpret.

## Disposition

- **FND-001** — resolved. FR-005 now defines the handoff shape: the mention edge
  carries the identifier exactly as written, and the consumer resolves it by
  last-segment lookup against its own index.
- **FND-002** — resolved. FR-008-CON-3 states the consumer's batch-composition
  obligation, and FR-008-AC-11 requires the result to report what the batch
  bounded, so an under-supplied batch is visible in the output.
- **FND-003** — accepted. Language selection stays a consumer input; a
  mismatch between the declared language and the file's actual content
  degrades to the parse-error path already specified by FR-007, so no
  additional requirement is warranted.
