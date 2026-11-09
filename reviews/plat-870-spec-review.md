---
id: SR-11082
title: "base spec review of FR-013 syntax_error amendment (PR #53)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-code-rs; spec/functional/FR-013-borrowed-parse-tree-api.md, spec/tests.md (PR #53 diff only)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-013
    type: reviews
---

## Summary

Ticket: PLAT-870. Base checklist over the PR #53 diff: the FR-013 amendment
adding `ParsedFile::syntax_error()`, FR-013-AC-13 and FR-013-AC-14, and
spec/tests.md rows TC-184..TC-186. IDs are well formed and collide with
nothing on origin/main (highest TC there is TC-183; FR-013's highest AC was
AC-12) or any other open PR (PR #53 is the only open PR). Every new AC has TC
rows, and the coverage rows name AC-13/AC-14 and TC-184..TC-186. The
criterion-strength lens was not run as a separate artifact because no Jev
client is installed; its question (can AC-13/14 fail) was judged manually
here instead.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium   | AC-13/AC-14 never pin "first ERROR/MISSING node anywhere" when a body-local error precedes a structural one; an implementation returning diagnostic()'s position whenever it is Some passes both ACs yet violates the FR statement | spec/functional/FR-013-borrowed-parse-tree-api.md:163-164 |
| FND-002 | low      | spec/tests.md marks the FR-013 coverage row (and StR-001 / US-005 rows) ✅ while TC-184..TC-186 are 🚧 unwritten, contrary to the table's own rule that a row whose test is unwritten carries 🚧 | spec/tests.md:45,57,85 |
| FND-003 | low      | AC-14's second clause (body-local-only file: syntax_error Some, diagnostic None) restates AC-13's first clause; redundant | spec/functional/FR-013-borrowed-parse-tree-api.md:164 |

## Verdict

Amendment is consistent with existing FR-013: `diagnostic()` semantics are
unchanged (only a naming parenthetical was added to its Outputs line; AC-3 and
AC-10 are untouched), and the stated implication diagnostic Some => syntax_error
Some holds by construction, since the declaration-structure predicate fires
only on a direct child that is itself an ERROR or MISSING node. AC-13 and
AC-14 can fail (a clean parse returning Some, a body-local file returning
None, a structural file returning None from syntax_error). Pre-existing
`quire validate` errors are identical on origin/main and are not findings.
