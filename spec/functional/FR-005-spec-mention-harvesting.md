---
id: FR-005
title: "Specification mention harvesting from comments and attributes"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-002"
    type: "implements"
  - target: "ix://agent-ix/filament-ide-rs/FR-075"
    type: "references"
---

# [FR-005] Specification mention harvesting from comments and attributes

## Description

The library SHALL harvest specification mentions — tracking tags of the form
`TC-NNN`, requirement citations of the form `FR-NNN`, `NFR-NNN` and `Task-NNN`,
and references of the form `ix://…` — from the comments and language-level
attributes of each file, and SHALL report each mention with its text, its
enclosing code fact, and the file and one-based line where it appears.

## Inputs

- Comment and attribute text recovered per file, with positions
- The structural facts from [FR-001](./FR-001-structural-fact-model.md), used to
  attribute each mention to its enclosing declaration

## Outputs

- Mention records carrying the matched identifier, its `kind`
  (`tracking_tag`, `requirement_citation` or `artifact_reference`), the
  qualified name of the enclosing code fact, and `{file, line}`
- Edges from the enclosing code fact to the mentioned identifier, carrying
  `reason` `explicit-mention` and confidence 1.0

## Behavior

- The library SHALL recognize a tracking tag only as a whole token, so that
  `TC-001` is a mention and the `TC-001` inside a longer alphanumeric token is
  not.
- The library SHALL attribute a mention to the innermost declaration whose span
  encloses it, and SHALL attribute a mention outside every declaration to the
  file.
- The library SHALL report a mention whose target this library cannot see,
  because resolving an identifier to a specification artifact is the consumer's
  responsibility; the library SHALL NOT drop a mention for being unresolvable.
- The library SHALL address a mention edge's target by the unqualified mentioned
  identifier — `TC-001`, `FR-002`, `Task-150` — or, for an `ix://` mention, by
  the reference exactly as written, so that the consumer resolves the target
  against its own index by last-segment lookup without this library inventing a
  node it cannot see.
- The library SHALL restrict tracking-tag mentions to declarations recognized as
  tests by the language's convention, so that a tag quoted in ordinary prose is
  reported as a citation rather than as a verification claim.
- The library SHALL harvest from comments and from language-level attributes,
  and SHALL NOT harvest from string literals, so that a message that happens to
  quote an identifier is not read as a citation.
- The library SHALL emit at most one mention edge per
  `(source_ref, edge_type, target_ref)` triple, aggregating repeated mentions
  through [FR-004](./FR-004-edge-provenance-and-dedupe.md).
- The library SHALL assign confidence 1.0 to every mention, since a mention is
  read from source text and never inferred.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-005-CON-1 | The library SHALL report mentions and their locations only; mapping a mention to a `verifies`, `implements` or `references` relationship against indexed artifacts belongs to the consumer per `ix://agent-ix/filament-ide-rs/FR-075` | Interface | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-005-AC-1 | A test function whose comment carries `TC-001` yields a `tracking_tag` mention attributed to that function, with file and line | Test (TC-026) |
| FR-005-AC-2 | A module doc comment citing `FR-002` yields a `requirement_citation` mention attributed to the file's code fact | Test (TC-027) |
| FR-005-AC-3 | A comment carrying an `ix://` reference yields an `artifact_reference` mention | Test (TC-028) |
| FR-005-AC-4 | An identifier appearing inside a string literal yields no mention | Test (TC-029) |
| FR-005-AC-5 | An identifier embedded in a longer token yields no mention | Test (TC-030) |
| FR-005-AC-6 | A mention naming an artifact absent from the batch is still reported, addressed by the identifier as written | Test (TC-031) |
| FR-005-AC-8 | A tracking tag outside a test declaration is reported as a citation, not as a verification claim | Test (TC-072) |
| FR-005-AC-9 | A comment above a declaration's attributes is attributed to that declaration; an inner doc comment stays with its enclosing scope | Test (TC-093), Test (TC-094), Test (TC-098), Test (TC-099) |
| FR-005-AC-10 | A criterion-level identifier is harvested as one mention, addressed as written, and not also as its bare requirement prefix | Test (TC-095) |
| FR-005-AC-7 | Extracting this library's own test suite recovers the tracking tags its tests carry | Test (TC-032) |

## Dependencies

- **Upstream**: [FR-001](./FR-001-structural-fact-model.md) supplies the
  declaration spans mentions are attributed to
- **Downstream**: [FR-004](./FR-004-edge-provenance-and-dedupe.md) deduplicates
  mention edges; `ix://agent-ix/filament-ide-rs/FR-075` materializes them as
  cross-layer relationships
