---
id: SR-006
title: "code review of issue 7 governed graph-quality measurements"
type: SpecReview
analysis: code-review
scope: "origin/main...a50288f: FR-011, FR-012, NFR-005, MP-001, IT-001, src/measurement.rs, src/bin/measure_graph_quality.rs, schemas/, tests/measurement_pipeline.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/Plan-001
    type: reviews
  - target: ix://agent-ix/quire-code-rs/TM-001
    type: references
---

## Summary

The remediated implementation is merge-ready. The governed producer now scores
the complete 114-case corpus without a subset-selection surface, preserves the
scorer as the truth comparator, enforces the active MeasurementPlan semantics,
and gates only on false-positive edges while retaining recall independently.
The real release-binary ecosystem lane passed against Quire and Quoin, including
a reversed-creation-order checkout and semantic verification of stored evidence.

## Verdict

**PASS** — no blocking correctness, compatibility, determinism, population,
governance, or test-confidence issue remains in the reviewed scope.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | low | No gaps found. | - |

## Remediation Verification

- The public producer has no `--case` input. It rejects a scorer report unless
  `scored_cases` exactly equals the complete `case_digests` population.
- Population censuses use scorer truth (`tp + fn`), and unresolved versus
  ambiguous counts use the documented extractor-reported and corpus-authored
  ambiguous-call marginals.
- The decision reads only `/confusion/axis_kind/edge/fp`; scorer status, recall,
  and non-edge false positives cannot create a false gate failure or pass.
- MP-001 must be active and match its identifier, metric, definition version,
  complete-census sampling rule, and zero-wrong-edge decision before emission.
- The raw schema closes fields and vocabularies, requires all five result
  dimensions, and rejects POSIX, UNC, drive-qualified, and slash or backslash
  parent-traversing raw paths.
- The Quoin attestation separately identifies the release producer, release
  extractor, retained raw scorer output, plan, schema, configuration, lockfile,
  source revisions, and toolchains. Typed observations include precision,
  precision decision, recall, unresolved, ambiguous, and false-positive values
  with explicit population state.

## Review Evidence

- `make fmt-check`, `make lint`, `make test`, `make deny`, and
  `make audit-unsafe`: pass. The ordinary suite ran 151 tests successfully; its
  two deliberately conditional lanes remain ignored by default.
- Explicit real lane at `a50288f`: pass with the current release producer and
  extractor, the complete 114-case quire-corpus checkout, a reversed-creation
  clone, installed Quire, and installed Quoin.
- Independent capture of Quoin's stored collection compared equal to the input
  as JSON data, including all raw evidence and typed observations. Quoin only
  normalized JSON number spelling such as `1.0` to `1`, which is semantically
  equal and is why the regression test compares retained semantics rather than
  bytes chosen by the downstream store.
- `quire validate --scope . 'spec/**/*.md' 'plan/**/*.md'
  'reviews/**/*.md'`: pass, apart from pre-existing non-fatal catalog/grammar
  warnings.
- `make coverage` reaches 238/261 backed targets with the remaining two rows
  explained by their declared method; its command status is blocked only by the
  installed external catalog's pre-existing empty `Inspections` and
  `SuiteRegistry` archetypes. All TC-108..TC-134 tags are directly present.

## Boundary and Completeness Review

No production stubs, `todo!`, `unimplemented!`, unsafe blocks, internal-logic
mocks, network client, or ambient record identity were found in the issue-7
diff. Filesystem and process operations stay at the binary boundary, while
transformation, canonicalization, decision evaluation, and schema validation
remain pure library behavior.
