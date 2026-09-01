---
id: SR-005
title: "base review of governed graph-quality measurements"
type: SpecReview
analysis: base
scope: "StR-003, US-004, FR-011, FR-012, NFR-005, MP-001, IT-001, and TM-001 rows TC-108..TC-134"
review_set: subset
---

## Summary

The extension is ready for tasking after correcting four blocking contract
defects found in review. The corrected contract keeps corpus truth comparison in
the real scorer, emits Quoin's actual MeasurementCollection v2 envelope, retains
the complete scorer report as raw evidence, and makes every ambient input
explicit.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | high | The draft emitted a private graph record that Quoin's real intake rejects instead of a MeasurementCollection v2; resolved by making stdout the Quoin envelope and nesting schema-valid raw evidence. | FR-012, IT-001 |
| FND-002 | high | NFR-005 prohibited timestamps while Quoin v2 requires one; resolved by making the timestamp a pinned caller input, never a clock read. | NFR-005, FR-012 |
| FND-003 | high | The schema promised unknown-language rejection but accepted arbitrary language census keys; resolved with a closed language census and conditional dimension vocabulary. | FR-011-AC-6 |
| FND-004 | medium | The TestMatrix tables used non-catalog column names and a malformed Test Case Summary separator; resolved and validated. | TM-001 |
| FND-005 | medium | The schema existed only inside Markdown and therefore could not be executed; resolved by requiring a checked-in authoritative schema file. | FR-011 |
| FND-006 | medium | Scorer orchestration permitted duplicate truth logic; resolved by requiring real `score.py --json` plus the real release extractor. | FR-012 |

## Readiness Checks

- Requirements are atomic and have stable TC-108..TC-134 mappings.
- Ownership remains in quire-code-rs; quire-corpus owns truth comparison and
  Quoin owns storage/reporting.
- Empty, unsupported, unreadable, invalid-plan, invalid-schema, wrong-edge, and
  nondeterministic failure domains are explicit.
- Provenance pins source, corpus, scorer, parser grammar, configuration,
  executable, lockfile, plan, schema, and toolchain identities.
- Verification includes pure schema/transform tests and a real three-project
  integration path with no mocked process or filesystem boundary.

## Verdict

**PASS** — the corrected requirements are taskable. Implementation must not
invent true negatives where the scorer declares no negative population; that
field remains `null`, while false positives independently gate the run.
