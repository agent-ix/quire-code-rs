---
id: SR-001
title: "base review of the quire-code-rs extraction contract"
type: SpecReview
analysis: base
scope: "spec/spec.md, spec/stakeholder/StR-001..002, spec/usecase/US-001..003, spec/functional/FR-001..008, spec/non-functional/NFR-001..004, spec/tests.md"
review_set: subset
---

## Summary

Checklist review of the initial contract spec authored to satisfy
`ix://agent-ix/filament-ide-rs/FR-072-CON-1`. Identifier formats, link
integrity, and the six coverage rules all pass; every one of the 70 acceptance
criteria carries a distinct `TC-NNN`, and no identifier is duplicated or
skipped. Two substantive defects were found in the acceptance criteria
themselves — one contradiction between naming and identity, one over-strict
error policy — plus three lower-severity issues about diagnostic volume,
harvest scope, and a knowingly red matrix row.

## Findings

| ID      | Severity | Summary                                                                                                                                                  | Refs |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- |
| FND-001 | high     | Anonymous-declaration names embed the start line, so moving one changes its node id — contradicting the stability criterion that says moving preserves it | FR-002-AC-5, FR-006-AC-2, FR-006-CON-1 |
| FND-002 | high     | Suppressing all facts from any file whose tree contains error nodes contradicts the stated intent to work on in-progress trees                            | FR-007-AC-3, ADR-001, US-001 |
| FND-003 | medium   | Every third-party import emits an unresolved-specifier diagnostic, which on a real repository produces thousands of entries carrying no signal            | FR-003-AC-3 |
| FND-004 | medium   | Mention harvesting is unrestricted by context, while the consuming requirement scopes tracking tags to test functions                                     | FR-005-AC-1, `ix://agent-ix/filament-ide-rs/FR-075` |
| FND-005 | low      | TC-032 is authored as a matrix row that is knowingly red until the mention linker lands; the matrix does not distinguish "planned" from "expected to fail" | TC-032 |

## Checklist Detail

### Identifier formats and integrity

All identifiers match their required patterns: `StR-001..002`, `US-001..003`,
`FR-001..008`, `NFR-001..004`, `TC-001..070`, with acceptance criteria numbered
`<doc-id>-AC-N` and constraints `<doc-id>-CON-N` throughout. No duplicates, no
gaps within an allocated range, and no identifier reused across types. Every
relative-path link in body prose resolves to an existing sibling artifact, and
every `ix://` relationship target carries at least three segments.

### Coverage rules

1. **Coverage** — all 70 acceptance criteria map to at least one test case; no
   AC is unmapped and no `TC` is orphaned.
2. **Language permutation** — Rust, TypeScript/TSX and Python each have
   dedicated structural criteria (TC-001..003) and precision corpora
   (TC-066..068).
3. **Conservatism** — each resolution tier has a positive criterion
   (TC-044, TC-053) and a negative one (TC-045, TC-051, TC-069).
4. **Determinism** — golden-file and repeat-extraction criteria cover the
   emitting path (TC-038, TC-054..056).
5. **Error path** — the parser-error diagnostic (TC-039..043) and the
   unresolved-specifier diagnostic (TC-017) each have a negative test.
6. **Edge case** — empty files (TC-004), files declaring nothing (TC-004),
   anonymous declarations (TC-012), unresolvable imports (TC-017), invalid
   UTF-8 (TC-043) and binding cycles (TC-049) all have dedicated cases.

### Acceptance-criteria quality

Criteria are measurable rather than adjectival: the NFRs state numeric targets
and thresholds with a stated method, and NFR-004 states its precision target as
an absolute (zero wrong edges on curated corpora) while deliberately declining
to threshold recall. The verification column uses ISO 29148 methods throughout,
annotated with the owning test case.

The two high-severity findings are defects in what the criteria *say*, not in
their form, and are addressed before this spec is treated as the gate.

## Disposition

All five findings were resolved in the same change that recorded them:

- **FND-001** — anonymous segments are now derived from the declaration's
  ordinal among its siblings rather than its start line, so the name survives an
  edit that shifts lines and the identity hash stays stable (FR-002, TC-012).
- **FND-002** — error handling moved from file granularity to declaration
  granularity: a declaration whose own subtree is clean still contributes facts,
  and only an error-node root suppresses the whole file (FR-007-AC-3,
  FR-007-AC-6, TC-071).
- **FND-003** — bare and package-absolute specifiers now yield no diagnostic at
  all; only an unresolvable *relative* import is diagnosed, since that is the
  case that signals an incomplete batch (FR-003-AC-3).
- **FND-004** — tracking tags are now recognized as verification claims only
  inside declarations the language treats as tests; elsewhere they are reported
  as ordinary citations (FR-005-AC-8, TC-072).
- **FND-005** — accepted as-is. TC-032 is deliberately authored ahead of the
  linker; the coverage notes in the matrix state that it is red until that slice
  lands.
