---
id: SR-11089
title: "PLAT-362 quire-code-rs PR56 spec-review (base)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-code-rs PR56; spec/functional/FR-013-borrowed-parse-tree-api.md, spec/tests.md (PR diff only)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-013
    type: reviews
---

## Summary

Ticket: PLAT-362 (part 1). Base checklist over the spec diff: FR-013-CON-3 now names `bash` among the like-named features; spec/tests.md adds TC-189 (FR-013-AC-1) and TC-190 (FR-013-AC-5) and widens the StR-001, US-005 and FR-013 coverage ranges to TC-190. The new ids collide with nothing on origin/main (highest TC there is TC-188). `quire validate --scope . "spec/**/*.md"` reports the same two pre-existing failures as main (tests.md column names; StR-003 EARS warning), nothing introduced by this diff. Sub-analyses beyond base were not run: the diff adds no requirement statement, no domain object and no relationship edge. Criterion strength was judged by hand (no Jev client).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-013-AC-4 still reads "the Python and TypeScript grammar crates are absent from the resolved dependency graph", though CON-3 now lists `bash` and `make check-single-grammar` now asserts tree-sitter-bash is absent too; AC-4 understates its own gate | spec/functional/FR-013-borrowed-parse-tree-api.md:152 |
| FND-002 | low | TC-190 asserts a Bash structural diagnostic (FR-013-AC-3 territory) but its row binds only FR-013-AC-5, so the Bash diagnostic behaviour is traced to no criterion | spec/tests.md:287 |

## Verdict

Minor changes. CON-3's amendment is correct and testable through TC-137/TC-138. Update AC-4's text to name the bash grammar, and either bind TC-190 to FR-013-AC-3 as well or split the diagnostic assertion into its own row.
