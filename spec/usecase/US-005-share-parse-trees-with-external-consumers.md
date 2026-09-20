---
id: US-005
title: "Share parse trees with a consumer doing its own classification"
type: US
relationships:
  - target: "ix://agent-ix/quire-code-rs/StR-001"
    type: "traces_to"
---

# [US-005] Share parse trees with a consumer doing its own classification

## Story

**As a** maintainer of a sibling Rust codebase that needs its own
source-structure classification
**I want** a tree-sitter syntax tree and the source text it was parsed from,
with no fact model attached
**So that** I can build my own classification on top of it — symbol kinds,
test detection, marker binding, retained source spans — without adopting this
library's own graph, and without vendoring a second, differently-pinned
tree-sitter dependency into my process

## Context

`quire-rs` hand-rolls line-structural scanners for Rust, Python and
TypeScript rather than depend on this repository, because depending on it
today means adopting its fact model, its `ObjectType`/`CodeFact` shapes, and
its own `load → resolve → emit → drop` discipline — none of which `quire-rs`
wants, and the last of which is actively wrong for it: `quire-rs` retains
source text after parsing (`SymbolExtraction::source_of()`,
`Symbol::attached_source()`), which this library's own pipeline deliberately
does not. `filament-ide-rs` and a future daemon have the same shape of need:
a tree, not a graph.

This is a distinct consumer need from [US-001](./US-001-index-a-repository-code-layer.md).
US-001 is answered by handing a consumer canonical graph records; this story
is answered by handing a consumer the tree itself, before this repository's
own classification runs over it.

## Acceptance Examples (Illustrative)

These examples clarify expectations. They are illustrative only — not test
cases and not verification criteria.

### [US-005-EX-1] A consumer walks a tree with no graph dependency

- **Given** Rust source text a consumer already holds in memory
- **When** the consumer calls the parse API for the language it selects
- **Then** it receives a syntax tree it can traverse with tree-sitter's own
  node API, and its own dependency graph names no fact model, type
  environment, call resolver or record emitter from this repository

### [US-005-EX-2] The source text survives the call

- **Given** a consumer that needs the parsed file's source text after parsing
  completes, to slice spans out of it later
- **When** the consumer calls the parse API and keeps the returned value
- **Then** the source text remains reachable through it for as long as the
  consumer's own code keeps it, with no forced drop imposed by the library

### [US-005-EX-3] A broken file fails loudly, not quietly

- **Given** a file whose syntax tree contains an error
- **When** the consumer calls the parse API for it
- **Then** the call returns a named error identifying the file and the line,
  and not a result indistinguishable from a file that legitimately declares
  nothing

### [US-005-EX-4] Selecting one language links one grammar

- **Given** a consumer that only ever hands this API Rust source
- **When** that consumer builds its own crate against this library with only
  the Rust language feature enabled
- **Then** its own build links the Rust grammar and does not compile or link
  the Python or TypeScript grammars

## Options (Exploratory)

Approaches raised during discovery, none implying commitment: each consumer
vendoring its own tree-sitter dependency and grammar set independently; a
daemon that parses once and serves trees over IPC; or one shared library
crate that every consumer pins by git revision. The last was favored for
requiring no running process, giving every consumer the same tree-sitter
version by construction, and matching how this ecosystem already pins
unpublished crates.

## Constraints (Contextual)

A consumer cannot accept a second, independently-pinned tree-sitter
dependency in its own process without risking two incompatible grammar ABIs
loaded at once. This library's own `load → resolve → emit → drop` discipline
is a choice for its own extraction pipeline and is contextual information
here, not a constraint this story inherits — the binding form is stated
elsewhere.

## Dependencies (Contextual)

Upstream: the consumer's own decision about which files to read and which
language to request for each. Downstream: this repository's own extraction
pipeline is a candidate future consumer of the same API, alongside `quire-rs`,
`filament-ide-rs` and a daemon.

## Priority and Risk (Informative)

Business value is high — `quire-rs`'s own PLAT-14/PLAT-163/PLAT-305 defects
trace to not having this. Urgency is high; `quire-rs`'s reimplementation is
blocked on this API existing. Risk if unmet is every consumer continuing to
hand-roll or vendor its own parser, each with its own gaps.

## Notes (Informative)

This library's own extraction pipeline is not required to consume this API in
this story's scope; nothing about its own pipeline changes here.

## Traceability (Informative)

This story traces to the single-engine stakeholder need
([StR-001](../stakeholder/StR-001-single-deterministic-code-extraction-engine.md))
— sharing one parse layer is what keeps "one deterministic engine" true
ecosystem-wide instead of true only inside this repository — and is expected
to inform the parse-API contract requirement.
