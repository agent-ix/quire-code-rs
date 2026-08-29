---
id: StR-002
title: "Engineers need requirement-to-code-to-test traceability recovered from source"
type: StR
relationships:
  - target: "ix://agent-ix/quire-code-rs/FR-005"
    type: "satisfied_by"
  - target: "ix://agent-ix/quire-code-rs/FR-008"
    type: "satisfied_by"
  - target: "ix://agent-ix/quire-code-rs/NFR-004"
    type: "satisfied_by"
---

# [StR-002] Engineers need requirement-to-code-to-test traceability recovered from source

## Stakeholder Need

Engineers working in a specification-driven codebase shall receive, for any
given requirement, the set of code symbols that cite it and the set of tests
that tag it, recovered from what the source itself states rather than from a
separately maintained traceability document. Every relationship the graph
asserts shall correspond to a real citation or a resolved binding in the code.

## Rationale

Specification-driven repositories already encode traceability in the source:
tests carry `TC-NNN` tracking tags, doc comments cite the requirements they
implement, and comments carry `ix://` references. That information is written
once and then goes unread, because nothing harvests it. Meanwhile the manually
maintained matrices that duplicate it drift from the code within weeks.

The same argument applies to structural relationships. An engineer asking "what
calls this function" wants an answer derived from the code, and one that is
wrong is worse than one that is incomplete: a fabricated call edge sends a
reader to the wrong place and, once discovered, discredits every other edge in
the view. A recovered-traceability graph is therefore only useful if it is
conservative — it must prefer silence to a guess.

## Validation Criteria

This need is considered satisfied when a test function tagged with a tracking
tag, a doc comment citing a requirement, and a comment carrying an `ix://`
reference each surface as a recoverable relationship with the file and line
that produced it, and when a deliberately ambiguous call site — one whose
receiver type cannot be determined — yields no relationship at all rather than a
plausible-looking wrong one. Satisfaction is judged against fixture corpora
whose correct relationships are known in advance.

| ID | Criteria | Validation |
|----|----------|------------|
| StR-002-VC-1 | A tracking tag on a test function surfaces as a relationship carrying its file and line | Test (TC-026) |
| StR-002-VC-2 | A requirement citation and an `ix://` reference each surface as a relationship | Test (TC-028) |
| StR-002-VC-3 | An ambiguous call site yields no relationship rather than a plausible wrong one | Test (TC-045) |
| StR-002-VC-4 | The precision corpora emit zero wrong edges in every supported language | Test (TC-066) |

## Stakeholders

The primary stakeholders are engineers and reviewers navigating a
specification-driven repository, who decide whether to trust the graph. Affected
parties include the automated gap-analysis tooling that reports coverage from
these relationships, and the requirement authors whose matrices the tooling
checks.

## Context and Assumptions

It is assumed that mentions follow the ecosystem's established textual
conventions (`TC-NNN`, `FR-NNN`, `NFR-NNN`, `Task-NNN`, `ix://…`), that
resolving a mention to a specification artifact is the consumer's job — this
library reports the mention and its location — and that the code being analyzed
does not typecheck cleanly in every case, since extraction must work on
in-progress trees.

## Stakeholder Constraints (Contextual)

Engineers have stated that they would rather see fewer relationships than
review a view containing wrong ones. This preference is made binding by a
precision requirement rather than by this need.

## Dependencies

**Upstream**: the mention conventions defined by the consuming specification
repositories. **Downstream**: an anticipated functional requirement for mention
harvesting, an anticipated functional requirement for conservative call
resolution, and a precision requirement bounding wrong edges.

## Priority and Risk (Informative)

Business value is high because recovered traceability is the difference between
a specification process that is checked and one that is merely written down.
Urgency is medium. The risk if unmet is a graph that engineers stop consulting,
which strands the entire code layer.

## Notes (Informative)

Open question for later analysis: whether relationships inferred by a language
model should ever be admitted alongside recovered ones. The prevailing view is
that they belong to a separate, separately validated layer owned by consumers.

## Traceability

This need is expected to be satisfied by mention harvesting
([FR-005](../functional/FR-005-spec-mention-harvesting.md)), receiver-typed call
resolution ([FR-008](../functional/FR-008-type-environments-and-call-resolution.md)),
and the conservative-resolution precision requirement
([NFR-004](../non-functional/NFR-004-conservative-resolution-precision.md)).
