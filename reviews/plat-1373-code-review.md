---
id: SR-16313
title: "code review of the green coverage and validate gates (PR #57)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-code-rs@0521422f452522d81537046ed19811b4dcffbf8f; scripts/check_coverage.sh, tests/dependency_boundary.rs, spec/tests.md, spec/functional/FR-011-graph-quality-observation-schema.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-012
    type: reviews
---

## Summary

Ticket: PLAT-1373. Code review, with the rust-review lane (rust-style loaded),
of `git diff origin/main...HEAD` on PR #57. The PR makes four changes. It drops
`archetype-matches-nothing` from the failing reasons in
`scripts/check_coverage.sh`. It adds TC-133 and TC-134 static tests to
`tests/dependency_boundary.rs`. It removes `object: data_schema` from FR-011. It
renames three `spec/tests.md` headers from `Coverage Status` to `Status`.

The coder's claim was checked and holds. The three diagnostics come from the
`spec-artifacts-process` manifest's `traceability:` block: `inspection`
(archetype `Inspections`), `interface-acceptance-criterion` (`interface`) and
`suite` (`SuiteRegistry`). This repository declares no traceability model of
its own and has no document of those archetypes. In quire-rs, FR-050's CR-117
note calls this noise "a declaration-side fact
(agent-ix/spec-artifacts-process#75) rather than an engine one", and that issue
is still open. FR-057 makes `trace:archetype-matches-nothing` a warning in both
postures.

The trouble is that the script now drops the whole reason, so it also drops
the same diagnostic for the declarations this repository depends on:
`test-case`, `acceptance-criterion`, `nfr-acceptance-criterion` and
`traces-to`. See FND-001.

The spec edits are correct. On origin/main, `quire validate --scope .
'spec/**/*.md'` fails on two documents. FR-011 fails the `data_schema` record,
which is missing `id` and `fields`, and no other spec document carries
`object:`. `spec/tests.md` fails because three headers do not match the
module's asserted `Status` columns (`stakeholder_coverage`,
`user_story_coverage`, `functional_coverage`). At HEAD, validate exits 0. The
fourth table, `### Measurement Plan Coverage`, has no `under_section` assertion
in the module manifest, so its `Coverage Status` header is legitimately
outside the contract.

The PR adds no ledger, digest, pin or manifest.

The rust-review lane covered the new tests. Through
`locked-build.sh ... cargo test --test dependency_boundary`, all 3 tests pass.
Both new tests guard against vacuity with `checked > 0`. Dependency scoping
reads `workspace_members` and not a hard-coded name. Trace comments follow the
file's existing `// TC-NNN` form. The scans are plain substring matches; their
one real weakness is FND-002.

## Verdict

**FAIL (FND-001).** I reproduced the defect in a scratch copy of HEAD by
changing `spec/tests.md` frontmatter from `type: TestMatrix` to
`type: TestMatrx`, which is the "matrix minted no TC id" defect named in the
gate's own header. With the PR's script, the run reports
`coverage: 139/177 targets backed` and exits 0. The real figure is 364 targets.
With origin/main's script, the run fails with
`archetype-matches-nothing: declaration 'test-case' declares archetype
'TestMatrix'`. The rest of the PR is sound and could merge once FND-001 is
fixed.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | high | Removing `archetype-matches-nothing` from the failing set wholesale makes the gate pass when the repository's own Test Matrix (or FR/NFR docs) is mistyped. Reproduced: `type: TestMatrx` → `139/177 targets backed`, exit 0, against a 364-target corpus; origin/main's script fails on `declaration 'test-case'`. Minimal fix: keep the reason and exempt only `d["declaration"] in {"inspection", "interface-acceptance-criterion", "suite"}` (module-declared targets with no instance here; owner fix is agent-ix/spec-artifacts-process#75), so any other declaration matching nothing still fails. | scripts/check_coverage.sh:72-84 |
| FND-002 | low | TC-133/TC-134 substring-scan whole source files, comments included, so a doc comment linking `https://…` or mentioning `std::net` fails the test though no behaviour changed. Strip `//` comment text before matching (or match only string literals / `use` paths) so the scan tests code, not prose. | tests/dependency_boundary.rs:118-155 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-003 | low | The skip `if d.get("declaration") in MODULE_UNUSED_DECLARATIONS: continue` drops every reason for those three declarations, not just `archetype-matches-nothing`. If the repository later adds an `Inspections` or `SuiteRegistry` document with a wrong section or id column, `section-holds-no-table` / `id-column-matches-nothing` on that declaration is silently swallowed. Narrow it to `d["reason"] == "archetype-matches-nothing" and d.get("declaration") in MODULE_UNUSED_DECLARATIONS`. Non-blocking: no such document exists today. | scripts/check_coverage.sh:71-82 |

## Dispositions

| FND | outcome | reason |
|---|---|---|
| FND-001 | fixed | 345d8fb restores `archetype-matches-nothing` and skips only the three module declarations; re-run: real tree exit 0 (325/364), mistyped `TestMatrx` exits 1 on `declaration 'test-case'`. |
| FND-002 | fixed | 345d8fb adds `strip_line_comments` to `producer_sources()`; `//` after `:` (a URL scheme in a string) is kept, so code literals are still scanned; 3/3 tests pass. |
