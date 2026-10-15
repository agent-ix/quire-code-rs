---
id: SR-11080
title: "gap analysis of FR-014 selected-scope extraction (PR #41)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-code-rs@17cfb4e6a9e7655a1b76162b9555345982a7f573; spec/functional/FR-014-selected-scope-extraction.md, spec/tests.md TC-177..TC-183, src/selected.rs, tests/selected_extraction.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-014
    type: reviews
---

## Summary

Ticket: QSL-775. Each of FR-014-AC-1 to AC-7 has a tagged test that can fail.
`quire coverage` at the reviewed sha reports 317/355 backed, and no FR-014 or
TC-177..TC-183 row is unbacked. The 4 unbacked rows (NFR-002-AC-3, TC-061,
FR-013-AC-4, FR-013-AC-8) already exist on main and are explained by method.
The ticket's vectors and contract tests are all present: full selection, an
omitted module, an unparsable file, an absent module, an extra file and an
identity mismatch. The `make ci` coverage step fails with 3
`archetype-matches-nothing` FAILs on declarations `inspection`,
`interface-acceptance-criterion` and `suite`. This PR touches none of them; the
failure comes from installed-module drift, not from this change. All other gate
steps exited 0. The result maps onto Contract IR `SourceInventoryIncompleteReason`:
`ModuleUnavailable{module, cause}` and `ParseFailed{cause}`. The adapter belongs
to IR-740, so defining the types locally is acceptable. Production stubs: 0.

## Verdict

**PASS.** Every acceptance criterion is backed by a test that can fail, and the
PR adds no unbacked row.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | low | No findings (placeholder) | - |
