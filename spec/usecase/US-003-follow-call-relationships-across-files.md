---
id: US-003
title: "Follow call relationships across files"
type: US
relationships:
  - target: "ix://agent-ix/quire-code-rs/StR-002"
    type: "traces_to"
---

# [US-003] Follow call relationships across files

## Story

**As an** engineer assessing the blast radius of a change
**I want** to see which functions call the one I am about to modify, including calls made through a typed receiver in another file
**So that** I can judge what a change breaks without reading the whole repository.

The story describes what the engineer wants to reach and deliberately says
nothing about how a receiver's type is determined.

## Context

Structural containment and imports are cheap to recover and already useful, but
the question engineers actually ask is about calls. Most interesting calls go
through a receiver — `store.upsert(...)`, `self.resolve(...)` — where the
callee's identity depends on the receiver's type, which is declared somewhere
else, possibly in another file.

Determining that type exactly is what a compiler does, and this library is not
one. It works on trees that may not compile, without a build, across three
languages. So it recovers what it can and must be explicit about how confident
it is in each answer, and silent when it cannot tell.

## Acceptance Examples (Illustrative)

These examples clarify expectations. They are illustrative only — not test cases
and not verification criteria.

### [US-003-EX-1] A call through a typed receiver resolves across files

- **Given** a variable declared with a type defined in another file, and a method
  called on it
- **When** the repository is extracted
- **Then** the engineer sees a call relationship to that type's method, marked as
  having been resolved by receiver type

### [US-003-EX-2] An undeterminable receiver produces nothing

- **Given** a method called on a value whose type cannot be recovered, where
  several unrelated types define a method of that name
- **When** the repository is extracted
- **Then** the engineer sees no call relationship for that call site

### [US-003-EX-3] Repeated calls collapse with their evidence retained

- **Given** the same function called from the same caller many times
- **When** the repository is extracted
- **Then** the engineer sees one relationship carrying how many call sites it
  represents and a bounded list of where they are

## Options (Exploratory)

Discovery considered name-only matching (cheap, produces obviously wrong edges in
any codebase with repeated method names), full type inference (correct, but
needs a compilable project and a per-language implementation), and a bounded
recovery of local type bindings iterated to a fixed point. The last was
favored as the only option that degrades to silence instead of to noise.

## Constraints (Contextual)

Engineers stated that a wrong call edge discredits the view and that ambiguity
should therefore resolve to nothing. Recovery is expected to be usable on trees
that do not compile.

## Dependencies (Contextual)

Upstream: the structural facts and symbol identity recovered per file.
Downstream: a likely functional requirement for type environments and call
resolution, and a precision requirement bounding wrong edges.

## Priority and Risk (Informative)

Business value is high; call relationships are the most-asked-for view. Urgency
is medium — structural extraction is useful before this lands. Risk if unmet is
a code layer that shows structure but cannot answer the question engineers
actually have.

## Notes (Informative)

Open question for later: whether trait-dispatched calls in Rust should resolve
to the trait method, to every implementation, or to nothing. Captured for
analysis; the conservative default is nothing.

## Traceability (Informative)

This story is expected to trace to the traceability stakeholder need
([StR-002](../stakeholder/StR-002-requirement-code-test-traceability.md)) and to
inform the type-environment and call-resolution requirement.
