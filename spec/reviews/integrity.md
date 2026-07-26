---
id: SR-002
title: "integrity review of the quire-code-rs extraction contract"
type: SpecReview
analysis: integrity
scope: "spec/functional/FR-001..008, spec/non-functional/NFR-001..004"
review_set: subset
---

## Summary

Integrity analysis of the contract for completeness, internal consistency and
atomicity. The requirement set is complete against the five contract elements
`ix://agent-ix/filament-ide-rs/FR-072-CON-1` enumerates, and no requirement
contradicts another at the level of stated behavior. Three internal
inconsistencies were found in the finer detail — a naming/identity conflict, an
undefined target for unresolved mentions, and an unstated interaction between
the fixpoint iteration bound and determinism.

## Findings

| ID      | Severity | Summary                                                                                                                                     | Refs |
| ------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ---- |
| FND-001 | high     | Line-derived anonymous segments make the qualified name positional, defeating the identity stability FR-006 requires of a pure name hash      | FR-002, FR-006-CON-1 |
| FND-002 | medium   | An unresolvable mention has no target node, but the provenance requirement deduplicates on a triple that requires `target_ref` — the shape of that reference is undefined | FR-004, FR-005 |
| FND-003 | medium   | The fixpoint iteration bound may be reached at different points for different batch compositions, making edge output batch-dependent in a way the determinism requirement does not acknowledge | FR-008-CON-2, NFR-001 |
| FND-004 | low      | FR-004 defines confidence values for three `reason` variants and leaves the numeric values for the three inferred variants to FR-008, which states ranking but not values | FR-004, FR-008 |

## Completeness Against the Gate

`FR-072-CON-1` requires that this tree normatively define five things. Each is
covered by at least one requirement carrying acceptance criteria:

| Contract element | Covered by |
|---|---|
| Symbol identity | [FR-002](../functional/FR-002-symbol-identity.md), with [FR-006](../functional/FR-006-canonical-record-emission.md) hashing it into record ids |
| Determinism | [NFR-001](../non-functional/NFR-001-determinism.md), supported by the ordering rules in FR-006 |
| Conservative resolution | [FR-008](../functional/FR-008-type-environments-and-call-resolution.md), bounded by [NFR-004](../non-functional/NFR-004-conservative-resolution-precision.md) |
| Provenance vocabulary | [FR-004](../functional/FR-004-edge-provenance-and-dedupe.md) |
| Canonical record parity | [FR-006](../functional/FR-006-canonical-record-emission.md) |

The vocabulary in FR-004 matches the consuming requirement value for value: six
`reason` variants, confidence on the closed unit interval, evidence capped at 20
entries, and `count` as the aggregate site total. The edge-type set in FR-006
likewise matches exactly. Both carry a constraint marking any change as a
coordinated contract change rather than a local decision.

## Consistency

No two requirements state conflicting behavior for the same trigger. The
interactions worth naming are:

- FR-003 and FR-008 both emit edges, and both delegate provenance and
  deduplication to FR-004 rather than restating it — a single owner for the
  vocabulary, which is what keeps it from drifting.
- FR-007 suppresses facts for a failed file, and FR-004 deduplicates across the
  batch; a file's absence therefore changes no other file's records, which
  FR-007-AC-4 asserts directly.
- FND-001 above is the one genuine contradiction: FR-002 makes a name positional
  for anonymous declarations while FR-006 requires a name-derived identifier to
  survive a move.

## Atomicity

Each requirement states one coherent capability and is independently verifiable.
FR-008 is the largest and was examined for splitting: it covers environment
construction, fixpoint propagation, tiered narrowing and type-relation edges.
These are kept together because the tiers are meaningful only against the
environment the same requirement builds, and splitting would scatter the
conservatism rule — the single most important property in the contract — across
four documents. Its acceptance criteria remain individually verifiable, which is the test that
matters.

## Disposition

- **FND-001** — resolved. Anonymous segments are ordinal-derived rather than
  line-derived, restoring the property that a qualified name is positional only
  in the file-path sense (FR-002).
- **FND-002** — resolved. FR-005 now states the target addressing explicitly: a
  mention edge targets the identifier as written, leaving last-segment
  resolution to the consumer's index.
- **FND-003** — resolved by disclosure rather than by change. FR-008 now
  requires the result to report the batch file count and the unresolved
  call-site count, and FR-008-CON-3 makes batch composition the consumer's
  stated obligation, so a bound-limited result is visible rather than silent.
- **FND-004** — accepted. Confidence values for the three inferred `reason`
  variants are ranked in FR-008 and left numerically unfixed deliberately;
  pinning exact floats before the precision corpora exist would be a guess
  encoded as a contract.
