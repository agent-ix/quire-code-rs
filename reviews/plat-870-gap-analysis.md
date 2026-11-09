---
id: SR-11086
title: "PLAT-870 quire-code-rs PR55 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-code-rs PR55; crates/quire-code-parse/src/parse.rs, src/parse.rs, spec/tests.md"
review_set: subset
---

## Summary

Ticket: PLAT-870. Plan completion: not assessed. `quoin matrix --json` (quoin 0.29.1) on head 90c7028 reports FR-013-AC-13 (TC-184/185 tests), FR-013-AC-14 (`diagnostic_some_implies_syntax_error_some`) and FR-013-AC-15 (`syntax_error_names_the_earlier_body_local_error_before_a_structural_one`) as `tagged`. Each new test can fail: TC-187 fails against main's position rule (main gives line 2 for both), TC-184 fails if `syntax_error()` returned `diagnostic()`, TC-185 fails on a spurious Some. No stubs or new untraced production code. spec/tests.md flips TC-184..187 and the FR-013 / US-005 / StR-001 rows to complete; every TC in TC-135..175 and TC-184..187 is now written and passing, so the row states match reality.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test pins diagnostic()'s position with two structural errors in one file, the case where the changed position rule now departs from AC-3's "first error position" (see SR-11085 FND-001) | crates/quire-code-parse/src/parse.rs:934 |

## Verdict

Coverage of the new ACs is real and traced. One gap: the changed `diagnostic()` position has no multi-structural-error test, which is exactly where it regresses.
