---
id: US-004
title: "Assess extractor quality from versioned raw observations"
type: US
relationships:
  - target: "ix://agent-ix/quire-code-rs/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quire-code-rs/FR-011"
    type: "traces_to"
  - target: "ix://agent-ix/quire-code-rs/FR-012"
    type: "traces_to"
---

# [US-004] Assess extractor quality from versioned raw observations

## Story

**As an** assurance practitioner comparing extractor revisions
**I want** each corpus run to retain dimensioned raw observations and exact input identities
**So that** I can distinguish precision, recall, ambiguity, and unavailable measurement conditions before using the result in a decision.

## Context

A single aggregate quality score hides the extractor's deliberate precision and
recall trade. The practitioner needs confusion matrices and unresolved or
ambiguous counts split by language, node kind, relation kind, and resolver tier.
The same view must expose whether the population was actually measurable.

## Acceptance Examples (Illustrative)

### [US-004-EX-1] Measured population remains decomposable

- **Given** a supported non-empty corpus with pinned truth
- **When** the practitioner inspects its observation
- **Then** the population census, dimensioned confusion matrices, unresolved
  counts, ambiguous counts, revisions, and raw scorer output are visible together

### [US-004-EX-2] Empty input is not perfect quality

- **Given** an empty corpus population
- **When** the producer attempts a measurement
- **Then** the result is `empty`, contains no metric results, and cannot be read as
  zero wrong edges with full recall

### [US-004-EX-3] Precision and recall remain separate

- **Given** a run with zero wrong heuristic edges and incomplete recovery
- **When** the practitioner inspects the result
- **Then** the invariant passes while recall is reported at its observed value
  without being converted into the precision decision

## Dependencies (Contextual)

The shared corpus owns truth and population declarations. Quoin consumes the
versioned record and owns cross-run reporting.
