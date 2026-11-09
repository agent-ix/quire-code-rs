---
id: SR-16314
title: "gap analysis of the green coverage and validate gates (PR #57)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-code-rs@0521422f452522d81537046ed19811b4dcffbf8f; tests/dependency_boundary.rs, spec/functional/FR-012-governed-graph-quality-producer.md, spec/tests.md, src/bin/measure_graph_quality.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-012
    type: reviews
---

## Summary

Ticket: PLAT-1373. Planless gap analysis scoped to the PR diff. Plan
completion: not assessed.

At HEAD, `scripts/check_coverage.sh` reports `325/364 targets backed` with 4
unbacked rows, all 4 explained by their method: NFR-002-AC-3, TC-061,
FR-013-AC-4 and FR-013-AC-8. Before this PR, FR-012-CON-2 and FR-012-CON-3 were
not backed. The two new tests now back them through TC-133 and TC-134, and the
matrix rows (`spec/tests.md:231-232`) carry `Static` / ✅.

Each test can fail. Adding any listed URL scheme, quoted git remote
subcommand, socket API or network crate to a scanned source or to a workspace
member's direct dependencies trips its assertion. The `checked > 0` guards
stop a missing `src` tree from passing vacuously. No stub code and no
coverage inflation were found in the diff.

## Verdict

**PASS with one low finding.** The constraints are backed by real checks.
FND-001 records what those checks cannot see.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | low | TC-134 states "the producer initiates no network request", but the producer delegates work to external programs it spawns (`args.python <corpus>/score.py`, `quire validate`, `git status`), which a source scan of this repo cannot see. The test comment should say subprocess behaviour is out of scope (owned by the corpus scorer / quire), so the ✅ does not over-claim. | src/bin/measure_graph_quality.rs:121-160, tests/dependency_boundary.rs:137-139 |

## Dispositions

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | 345d8fb adds to the TC-134 comment: "Subprocess behaviour (the scorer's python, `quire validate`, `git status`) is out of scope for a source scan." |
