---
type: master-requirements
name: quire-code-rs
org: agent-ix
component_type: rust-lib
tags:
  - rust
  - tree-sitter
  - knowledge-graph
  - static-analysis
  - code-extraction
implementation_language: rust
depends_on: []
relationships:
  - target: "ix://agent-ix/quire-rs"
    type: "references"
    cardinality: "1:1"
standards_alignment:
  - iso-iec-ieee-29148
title: "Master Requirements Specification"
---

# Master Requirements Specification
## quire-code-rs — Deterministic Source-Code Knowledge-Graph Extraction

---

## 1. Purpose

This document defines the scope, intent, and governing requirements framework
for `quire-code-rs`, a Rust library that turns source files into canonical
knowledge-graph records.

It is the sibling of `quire-rs`. Where `quire-rs` extracts canonical records
from markdown specification artifacts, `quire-code-rs` extracts them from source
code — Rust, TypeScript/TSX and Python — using tree-sitter grammars. Both feed
the same graph store, and both obey the same discipline: filesystem only, no
network, load → resolve → emit → drop, deterministic output.

This document establishes:

- The extraction contract consumers depend on: symbol identity, determinism,
  conservative resolution, the provenance vocabulary, and canonical record
  parity.
- The boundary between this library and its consumers — what it recovers from
  source, and what it deliberately leaves to the caller.
- The relationship between stakeholder need, library behavior, and test
  evidence.

**Core invariant**: a missing edge beats a wrong edge. Where the source does not
determine a relationship, this library emits nothing rather than a plausible
guess, and every relationship it does emit carries the evidence that produced it.

## 2. Scope

### 2.1 In Scope

- tree-sitter parsing for Rust, TypeScript/TSX and Python
  ([FR-001](./functional/FR-001-structural-fact-model.md)).
- Org-qualified symbol identity and naming
  ([FR-002](./functional/FR-002-symbol-identity.md)).
- Structural relationships — containment and path-resolved imports
  ([FR-003](./functional/FR-003-structural-edges.md)).
- The provenance vocabulary and edge deduplication
  ([FR-004](./functional/FR-004-edge-provenance-and-dedupe.md)).
- Specification mention harvesting from comments and attributes
  ([FR-005](./functional/FR-005-spec-mention-harvesting.md)).
- Canonical record emission at `quire-rs` parity
  ([FR-006](./functional/FR-006-canonical-record-emission.md)).
- Per-file parse-error isolation
  ([FR-007](./functional/FR-007-parse-error-isolation.md)).
- Per-file type environments and receiver-typed fixpoint call resolution
  ([FR-008](./functional/FR-008-type-environments-and-call-resolution.md)).
- Declaration visibility and normalized callable signatures, so a consumer can
  separate an exported-surface change from a body-only change
  ([FR-009](./functional/FR-009-declaration-visibility-and-signature.md)).
- Versioned governed graph-quality observations, including exact population,
  confusion matrices, unresolved and ambiguous censuses, producer revisions,
  and retained raw scorer output
  ([FR-011](./functional/FR-011-graph-quality-observation-schema.md),
  [FR-012](./functional/FR-012-governed-graph-quality-producer.md)).

### 2.2 Out of Scope

- **Persistence.** Consumers own the store; this library returns values.
- **File collection and watching.** Consumers decide which files exist, which
  are in scope, and when to re-extract.
- **Resolving mentions to specification artifacts.** This library reports what a
  comment says and where; mapping that to an indexed artifact needs the graph,
  which consumers hold.
- **Network access of any kind**
  ([NFR-002](./non-functional/NFR-002-filesystem-only-boundary.md)).
- **Model-inferred semantic relationships.** These are a consumer concern,
  validated separately, and must not be mixed with recovered facts.
- **Full type checking.** The type environment recovers what it safely can; it
  is not a compiler front end and does not require a compilable project.
- **Evidence persistence and portfolio policy.** This repository emits a
  versioned observation; Quoin owns storage, cross-run comparison, and reports.

## 3. System Overview

### 3.1 System Description

A consumer supplies a batch of source files — content in memory, each attributed
to an organization, repository and repository-relative path. The library parses
each file, recovers structural facts and mentions, builds per-file type
environments, resolves call relationships to a fixed point across the batch, and
returns canonical node and edge records plus per-file diagnostics.

### 3.2 Data Flow

```text
   consumer-collected files (in memory, org/repo/path attributed)
                              │
                              ▼
              tree-sitter parse, per file, per language        FR-001
                              │
              ┌───────────────┼────────────────┐
              ▼               ▼                ▼
     structural facts    mentions in     import specifiers
     + qualified names   comments/attrs         │
        FR-001/FR-002        FR-005             │ FR-003
              │               │                 │
              └───────────────┼─────────────────┘
                              ▼
                  in-memory fact corpus (whole batch)
                              │
                              ▼
             per-file type environments + fixpoint            FR-008
             receiver-typed call resolution
                              │
                              ▼
            provenance stamping + edge dedupe                 FR-004
                              │
                              ▼
             canonical records + diagnostics             FR-006 / FR-007
                              │
                              ▼
          optional governed corpus scorer                 FR-011 / FR-012
          (versioned observation + raw output)
                              │
                              ▼
                    consumer writes to its store
```

### 3.3 Intended Users

Graph-indexing consumers — principally Filament IDE
(`ix://agent-ix/filament-ide-rs/FR-072`) and analysis workers — that pin this
library by git revision and call it in-process.

## 4. Requirements Architecture

| Class | IDs | Location |
|---|---|---|
| Stakeholder requirements | StR-001..StR-003 | `stakeholder/` |
| User stories | US-001..US-004 | `usecase/` |
| Functional requirements | FR-001..FR-012 | `functional/` |
| Non-functional requirements | NFR-001..NFR-005 | `non-functional/` |
| Measurement plans | MP-001 | `measurement/` |
| Integration tests | IT-001 | `integration/` |
| Test matrix | TM-001 | `tests.md` |
| Spec reviews | SR-NNN | `reviews/` |

Identifiers are sequential within this repository and are never reused.
Relationship targets use `ix://agent-ix/quire-code-rs/<ID>`; cross-repository
targets name the owning repository.

## 5. Requirements Summary

### 5.1 Stakeholder Requirements

| ID | Need |
|---|---|
| [StR-001](./stakeholder/StR-001-single-deterministic-code-extraction-engine.md) | One deterministic extraction engine; no parser dependency in consumers |
| [StR-002](./stakeholder/StR-002-requirement-code-test-traceability.md) | Traceability recovered from source, and trustworthy |
| [StR-003](./stakeholder/StR-003-governed-extractor-quality-observations.md) | Governed extractor-quality observations preserve population, raw results, and revisions |

### 5.2 User Stories

| ID | Story |
|---|---|
| [US-001](./usecase/US-001-index-a-repository-code-layer.md) | Index a repository's code layer through one library call |
| [US-002](./usecase/US-002-trace-a-requirement-to-code-and-tests.md) | Trace a requirement to the code and tests that realize it |
| [US-003](./usecase/US-003-follow-call-relationships-across-files.md) | Follow call relationships across files |
| [US-004](./usecase/US-004-assess-versioned-extractor-quality.md) | Assess extractor quality from versioned raw observations |

### 5.3 Functional Requirements

| ID | Requirement |
|---|---|
| [FR-001](./functional/FR-001-structural-fact-model.md) | Per-file structural fact model (Rust, TS/TSX, Python) |
| [FR-002](./functional/FR-002-symbol-identity.md) | Org-qualified symbol identity |
| [FR-003](./functional/FR-003-structural-edges.md) | `contains` and path-resolved `imports` edges |
| [FR-004](./functional/FR-004-edge-provenance-and-dedupe.md) | Provenance vocabulary and edge deduplication |
| [FR-005](./functional/FR-005-spec-mention-harvesting.md) | Specification mention harvesting |
| [FR-006](./functional/FR-006-canonical-record-emission.md) | Canonical record emission at `quire-rs` parity |
| [FR-007](./functional/FR-007-parse-error-isolation.md) | Per-file parse-error isolation |
| [FR-008](./functional/FR-008-type-environments-and-call-resolution.md) | Type environments and fixpoint call resolution |
| [FR-009](./functional/FR-009-declaration-visibility-and-signature.md) | Declaration visibility and normalized callable signature |
| [FR-010](./functional/FR-010-producer-invocation.md) | Corpus extractor producer invocation contract |
| [FR-011](./functional/FR-011-graph-quality-observation-schema.md) | Versioned graph-quality observation schema |
| [FR-012](./functional/FR-012-governed-graph-quality-producer.md) | Governed graph-quality measurement producer |

### 5.4 Non-Functional Requirements

| ID | Requirement | Quality attribute |
|---|---|---|
| [NFR-001](./non-functional/NFR-001-determinism.md) | Byte-identical output for identical input | Reliability |
| [NFR-002](./non-functional/NFR-002-filesystem-only-boundary.md) | No network, filesystem, environment or process access | Security |
| [NFR-003](./non-functional/NFR-003-extraction-time-budget.md) | Extraction time budget for interactive reindexing | Performance efficiency |
| [NFR-004](./non-functional/NFR-004-conservative-resolution-precision.md) | Zero wrong edges on known-binding corpora | Reliability |
| [NFR-005](./non-functional/NFR-005-deterministic-quality-observations.md) | Byte-identical governed observations for pinned inputs | Reliability |

### 5.5 Measurement and Integration

| ID | Artifact |
|---|---|
| [MP-001](./measurement/MP-001-graph-quality-observation.md) | Governing decision use, population, collection, and interpretation for graph-quality observations |
| [IT-001](./integration/IT-001-corpus-observation-to-quoin.md) | Real corpus-to-producer-to-Quoin evidence path |

## 6. Architecture Decisions

### ADR-001: tree-sitter rather than language servers or compiler front ends

**Status**: Accepted.

**Context**: Recovering symbols and call relationships across three languages
could be done by driving a language server per language, by embedding each
language's compiler front end, or by parsing directly with a uniform grammar
toolkit.

**Decision**: Parse with tree-sitter grammars, one per language, configured by
data.

**Consequences**: One dependency story and one extraction engine across all
languages, with no daemon to supervise and no dependence on a project's build
being green — extraction works on in-progress trees, which is the common case in
an editor. The cost is that no type information arrives for free: everything
[FR-008](./functional/FR-008-type-environments-and-call-resolution.md) recovers,
it recovers itself, and it recovers less than a compiler would. That trade is
acceptable precisely because
[NFR-004](./non-functional/NFR-004-conservative-resolution-precision.md) makes
under-recovery safe and over-claiming forbidden. Tree-sitter also brings a C FFI
surface, confined to dependencies by
[FR-001](./functional/FR-001-structural-fact-model.md).

### ADR-002: in-memory fact corpus, consumer-owned persistence

**Status**: Accepted.

**Context**: Cross-file call resolution needs a view of the whole batch. That
view could be a persistent index maintained by this library, or a corpus built
per batch and dropped.

**Decision**: Build the fact corpus in memory for the batch, resolve against it,
emit records, and drop it. This library stores nothing.

**Consequences**: The library stays a pure function of its inputs, which is what
makes [NFR-001](./non-functional/NFR-001-determinism.md) and
[NFR-002](./non-functional/NFR-002-filesystem-only-boundary.md) achievable, and
it keeps schema and migration concerns entirely on the consumer's side. The cost
is that incremental re-extraction of a single file cannot consult facts from
files outside the batch, so a consumer wanting whole-repository resolution
supplies the whole repository; the budget for doing so is
[NFR-003](./non-functional/NFR-003-extraction-time-budget.md).

## 7. Verification Approach

Every acceptance criterion maps to a `TC-NNN` test case in
[tests.md](./tests.md). Tests carry their tracking tag as a comment, so this
repository is the first consumer of its own mention-harvesting behavior — the
matrix is checkable by the library it documents.

Quality gates: `make ci` runs formatting, clippy with warnings denied, the test
suite, license checks and the unsafe audit.
`quire validate --scope . "spec/**/*.md"` validates this tree.

## 8. References

- ISO/IEC/IEEE 29148 — Requirements Engineering
- `agent-ix/quire-rs` — sibling engine for markdown artifacts; source of the
  canonical record conventions this library matches
  ([FR-006](./functional/FR-006-canonical-record-emission.md))
- `agent-ix/filament-ide-rs` — principal consumer; its FR-072 defines the
  integration contract and its FR-072-CON-1 gates the pin on this spec tree
- `agent-ix/rust-lib-cookiecutter` — source of this repository's safety
  scaffolding, originally backported from `agent-ix/ecaz`
- tree-sitter — the parsing toolkit adopted in ADR-001
