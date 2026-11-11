---
id: SR-16316
title: "gap analysis of protected-apparatus attestation and quire-corpus scorer (PR #58)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-code-rs@58a7595801ba037cb7f3cf9cd224f5b4be9a3108; src/bin/measure_graph_quality.rs, src/measurement.rs, tests/measurement_pipeline.rs, spec/functional/FR-012-governed-graph-quality-producer.md, spec/assurance/MP-001-graph-quality-observation.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-012
    type: reviews
---

## Summary

Ticket: PLAT-1361. Planless gap analysis scoped to the PR #58 diff.

Plan completion: not assessed

The diff adds two behaviors:

- the verification-stack `artifacts` are derived from MP-001
  `protected_apparatus`;
- the producer scores with the quire-corpus CLI.

Neither has an owning acceptance criterion. FR-012's Behavior bullets still
describe the old scorer and the old artifact names (see SR-16315 FND-001 and
FND-002). The new unit test `artifacts_cover_every_protected_apparatus_path`
carries no `// TC-NNN` trace comment, and `spec/tests.md` is unchanged.

The acceptance lane itself is real. I ran
`real_corpus_observation_is_retained_and_rendered_by_quoin` with
`--include-ignored` against a clean quire-corpus origin/main build, using
Quoin 0.30.0 and Quire 0.40.0. It passed. It asserts a zero exit from
`quoin measurement record` and from both `quoin report` calls, and it asserts
that the retained `verificationStack/artifacts` are unchanged. The test is
still `#[ignore]`, so `make ci` never runs it. That was already true before
this PR.

The repository's own matrix gate cannot be used as evidence here.
`scripts/check_coverage.sh` fails identically on origin/main (108ddda) and at
58a7595, with `hollow-denominator` and no rows backed, because Quire 0.40.0's
traceability model binds `#[trace(...)]` attributes and this repository uses
`// TC-NNN` comments. This PR did not cause that.

The PR adds no stubs, and no ledger, digest or pin. Semantic review was not
run (it is opt-in, and this was a non-interactive reviewer dispatch).

## Verdict

**CONDITIONAL.** The fix needs three things:

- an FR-012 AC for the protected-apparatus attestation, and one for the
  quire-corpus scorer invocation, with FR-012's Behavior bullets rewritten;
- a TC row for the new unit test;
- a `// TC-NNN, FR-012-AC-N` trace comment on that test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new artifacts-from-protected_apparatus behavior and the quire-corpus scorer invocation have no owning FR-012 acceptance criterion; the new unit test carries no TC/AC trace comment and spec/tests.md has no row for it | src/measurement.rs:769-793, spec/functional/FR-012-governed-graph-quality-producer.md:98-111 |
