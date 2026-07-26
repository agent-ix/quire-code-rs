---
id: US-002
title: "Trace a requirement to the code and tests that realize it"
type: US
relationships:
  - target: "ix://agent-ix/quire-code-rs/StR-002"
    type: "traces_to"
---

# [US-002] Trace a requirement to the code and tests that realize it

## Story

**As an** engineer reviewing whether a requirement is actually built and tested
**I want** the mentions of that requirement written in the source to be surfaced as relationships
**So that** I can go from the requirement to its implementation and its tests without trusting a hand-maintained matrix.

The story stays at the level of what the engineer wants to reach; it does not
prescribe how mentions are recognized or resolved.

## Context

Repositories in this ecosystem already write the traceability down. Tests carry
`TC-NNN` tags in comments; module and function doc comments cite the `FR-NNN`,
`NFR-NNN` or `Task-NNN` they implement; comments carry `ix://` references to
artifacts in other repositories. None of it is read today.

This library is itself such a repository — its own tests carry tracking tags —
so it can be pointed at its own source as the first consumer of the behavior it
provides.

## Acceptance Examples (Illustrative)

These examples clarify expectations. They are illustrative only — not test cases
and not verification criteria.

### [US-002-EX-1] A tagged test surfaces as a verification relationship

- **Given** a test function whose comment carries a tracking tag
- **When** the file is extracted
- **Then** the engineer sees a mention linking that test to the tagged test case,
  carrying the file and line it came from

### [US-002-EX-2] A citing doc comment surfaces as an implementation relationship

- **Given** a module doc comment citing a requirement identifier
- **When** the file is extracted
- **Then** the engineer sees a mention linking that file's code to the cited
  requirement

### [US-002-EX-3] A mention of something not yet indexed is still reported

- **Given** a comment citing an artifact the consumer has not indexed yet
- **When** the file is extracted
- **Then** the mention is still reported with its text and location, leaving the
  consumer to resolve or park it

## Options (Exploratory)

Discovery considered restricting harvesting to doc comments only, to all
comments, or to comments and language-level attributes. Broader harvesting finds
more, at the cost of picking up mentions in prose that were never meant as
citations. No commitment is implied here.

## Constraints (Contextual)

Engineers said a wrong trace is worse than a missing one, and that a mention
without a file and line is not actionable. Both are context; the binding forms
live in the requirements.

## Dependencies (Contextual)

Upstream: the mention conventions used by the consuming specification
repositories. Downstream: a likely functional requirement for mention harvesting
and the consumer-side materialization of cross-layer relationships.

## Priority and Risk (Informative)

Business value is high — this closes the requirement-to-evidence loop that the
specification process assumes. Urgency is medium. Risk if unmet is that coverage
continues to be asserted rather than checked.

## Notes (Informative)

Open question for later analysis: whether a mention appearing inside a string
literal or a commented-out block should count. Captured without deciding.

## Traceability (Informative)

This story is expected to trace to the traceability stakeholder need
([StR-002](../stakeholder/StR-002-requirement-code-test-traceability.md)) and to
inform the mention-harvesting requirement.
