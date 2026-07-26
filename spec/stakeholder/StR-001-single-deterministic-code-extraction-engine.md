---
id: StR-001
title: "Ecosystem needs one deterministic code-extraction engine"
type: StR
relationships:
  - target: "ix://agent-ix/quire-code-rs/FR-001"
    type: "satisfied_by"
  - target: "ix://agent-ix/quire-code-rs/FR-006"
    type: "satisfied_by"
  - target: "ix://agent-ix/quire-code-rs/NFR-001"
    type: "satisfied_by"
---

# [StR-001] Ecosystem needs one deterministic code-extraction engine

## Stakeholder Need

Consumers of the Filament knowledge graph shall obtain source-code facts from a
single extraction engine that produces byte-identical records for identical
input, so that no consumer maintains its own parser, no consumer links a
source-parsing library into its own process, and identical repositories indexed
on different machines yield identical graphs.

## Rationale

The knowledge graph already ingests markdown specification artifacts through
`quire-rs`. Extending it to source code creates a second extraction path, and
the obvious implementation — linking tree-sitter directly into each consumer —
fails three ways. First, it duplicates parsing, symbol-resolution and naming
rules across every consumer that wants code facts, and those rules drift.
Second, it drags a C FFI surface into application processes such as the
Filament IDE, whose security posture assumes a small, auditable native
dependency set. Third, extraction that is not bit-reproducible makes reindexing
non-idempotent: a re-scan of unchanged sources rewrites graph rows, invalidates
caches, and produces spurious diffs that hide real change.

Concentrating parsing behind one library reverses all three. The parser
dependency lives in exactly one crate, the identity and resolution rules have
exactly one implementation to review, and a determinism guarantee makes
reindexing a no-op when nothing changed.

## Validation Criteria

This need is considered satisfied when a consumer obtains code nodes and edges
for a mixed-language repository through this library's API alone, with no
source-parsing crate appearing anywhere else in that consumer's dependency tree,
and when two extractions of an unchanged tree — performed in separate processes
on separate machines — produce byte-identical output. Satisfaction is judged by
inspecting the consumer's resolved dependency graph and by comparing serialized
extraction output across repeated runs.

## Stakeholders

The primary stakeholders are the maintainers of graph-indexing consumers —
principally Filament IDE (`ix://agent-ix/filament-ide-rs/FR-072`) and the
analysis workers — who are accountable for index correctness and for the native
dependency surface of their processes. Affected parties are the engineers who
query the resulting graph and who lose trust in it when the same input yields
different answers on different days.

## Context and Assumptions

Source files arrive already collected and attributed to an organization and
repository by the consumer; this library does not walk repositories or decide
which files are in scope. It is assumed that extraction runs offline, that the
supported language set begins with Rust, TypeScript/TSX and Python, and that
consumers own all persistence.

## Stakeholder Constraints (Contextual)

Consumers index whole repositories inside an interactive application, so
extraction is expected to complete fast enough that a full index of a
medium repository is not perceived as a background job that never finishes.
This expectation is refined into a concrete budget by a non-functional
requirement rather than being binding here.

## Dependencies

**Upstream**: the canonical record conventions established by
`ix://agent-ix/quire-rs/spec`, which this engine matches so that code and spec
records land in one store. **Downstream**: anticipated functional requirements
for the structural fact model, symbol identity, and canonical emission, and a
non-functional requirement establishing determinism.

## Priority and Risk (Informative)

Business value is high: the code layer is the precondition for every
requirement-to-code traversal the graph promises. Urgency is high because
consumer work is gated on the contract this need drives. The risk if unmet is a
graph whose contents depend on which machine built it, which is worse than no
code layer at all.

## Notes (Informative)

Open question for later analysis: whether languages beyond the initial three
are added here or through a plugin surface. Captured without introducing a
requirement.

## Traceability

This need is expected to be satisfied by the structural fact model
([FR-001](../functional/FR-001-structural-fact-model.md)), canonical record
emission ([FR-006](../functional/FR-006-canonical-record-emission.md)), and the
determinism requirement
([NFR-001](../non-functional/NFR-001-determinism.md)).
