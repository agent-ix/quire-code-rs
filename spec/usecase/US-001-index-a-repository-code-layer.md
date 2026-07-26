---
id: US-001
title: "Index a repository's code layer into the knowledge graph"
type: US
relationships:
  - target: "ix://agent-ix/quire-code-rs/StR-001"
    type: "traces_to"
---

# [US-001] Index a repository's code layer into the knowledge graph

## Story

**As a** maintainer of a graph-indexing consumer
**I want** to hand a batch of collected source files to one library call and get back canonical graph records
**So that** my application gains a code layer without me writing a parser or linking one into my process.

This story is written from the consuming application's perspective and does not
prescribe how the library parses, resolves, or names anything.

## Context

The consumer already collects files and knows which organization and repository
each belongs to; it also already writes canonical records for markdown
specification artifacts. What it lacks is anything that turns a `.rs`, `.ts`,
`.tsx` or `.py` file into records of the same shape. The consumer expects to
treat this library the way it already treats `quire-rs`: pin it by revision,
call it, write what comes back.

Reindexing is frequent — every save in an editor can trigger one — so the
consumer also cares that a re-extraction of unchanged files produces exactly
what the previous one did, letting it skip the write entirely.

## Acceptance Examples (Illustrative)

These examples clarify expectations. They are illustrative only — not test cases
and not verification criteria.

### [US-001-EX-1] Mixed-language repository yields structural records

- **Given** a repository containing Rust, TypeScript and Python sources
- **When** the consumer extracts the collected batch
- **Then** it receives file, module, function and type records, plus the
  containment and import relationships among them

### [US-001-EX-2] A broken file does not sink the batch

- **Given** one file in the batch that does not parse
- **When** the consumer extracts the batch
- **Then** it receives a diagnostic naming that file, and full records for every
  other file

### [US-001-EX-3] Re-extraction of an unchanged tree changes nothing

- **Given** a tree extracted once already and untouched since
- **When** the consumer extracts it again
- **Then** the output is identical to the previous output, and the consumer
  skips the write

## Options (Exploratory)

Approaches raised during discovery, none implying commitment: driving a language
server per language and harvesting its symbol index; shelling out to per-language
tooling; or parsing directly with a uniform grammar toolkit. The last was
favored for having no daemon, one dependency story across languages, and no
dependence on a project's build being green.

## Constraints (Contextual)

The consumer is an interactive application, so a full index of a
medium-sized repository should not feel unbounded. The consumer also cannot
accept a parser dependency in its own process. These are context; the binding
forms are stated elsewhere.

## Dependencies (Contextual)

Upstream: the consumer's file-collection and language-allowlist behavior.
Downstream: likely functional requirements for the structural fact model,
canonical emission, and parse-error isolation.

## Priority and Risk (Informative)

Business value is high — this is the entry point for the entire code layer.
Urgency is high; consumer work is blocked on the contract. Risk if unmet is that
consumers each grow their own parser.

## Notes (Informative)

Open question captured for later: whether the library should accept file
contents in memory only, or also read paths itself. The purity argument favors
in-memory.

## Traceability (Informative)

This story is expected to trace to the single-engine stakeholder need
([StR-001](../stakeholder/StR-001-single-deterministic-code-extraction-engine.md))
and to inform requirements covering the fact model, identity, structural edges,
canonical emission, and parse-error isolation.
