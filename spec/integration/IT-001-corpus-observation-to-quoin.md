---
id: IT-001
title: "Pinned corpus observation is accepted by Quoin"
type: IT
relationships:
  - target: "ix://agent-ix/quire-code-rs/FR-011"
    type: "verifies"
  - target: "ix://agent-ix/quire-code-rs/FR-012"
    type: "verifies"
  - target: "ix://agent-ix/quire-code-rs/NFR-005"
    type: "verifies"
---

# [IT-001] Pinned corpus observation is accepted by Quoin

## Objective

Verify the complete local evidence path from a pinned quire-corpus checkout,
through the real extractor and scorer, through schema and MeasurementPlan
validation, into Quoin's evidence intake without losing raw output or dimensioned
counts.

## Target Integration

The producer under test invokes the real `extract_tree` binary and reads the real
quire-corpus population and truth files from the filesystem. Quire validates
MP-001 and the observation schema. Quoin accepts and renders the resulting
engine-agnostic record through its evidence and report surfaces.

## Preconditions

The quire-code-rs source, quire-corpus checkout, extraction configuration, parser
grammars, scorer, Quire, and Quoin are available locally at recorded exact
revisions. The selected corpus fixture is non-empty, readable, and includes truth
for all required dimensions. An empty temporary output and Quoin store are
available.

## Inputs

- Pinned complete-graph corpus manifest, sources, truth records, and population
  declaration.
- Built `extract_tree` and graph-quality producer binaries.
- Active [MP-001](../measurement/MP-001-graph-quality-observation.md).
- A deterministic output directory relative to the run root.

## Test Procedure

1. Validate MP-001 with the installed Quire catalog.
   - IT-001-SC-01: Quire exits zero and reports no document diagnostic.
2. Run the graph-quality producer twice over the pinned corpus after reversing
   fixture creation order.
   - IT-001-SC-02: both runs exit according to the zero-wrong-edge decision and
     emit byte-identical observations with matching raw-output digests.
3. Validate each observation against the versioned JSON Schema.
   - IT-001-SC-03: both observations validate and name MP-001's exact definition
     version.
4. Submit one observation and its raw scorer output to a real temporary Quoin
   evidence store.
   - IT-001-SC-04: Quoin accepts the record and retains its content digest and
     producer tuple.
5. Render Quoin's report for the observation.
   - IT-001-SC-05: population state, precision result, recall, unresolved counts,
     ambiguous counts, raw-output reference, and all four dimensions remain
     inspectable without an aggregate trust score.

## Expected Results

The same pinned run produces the same observation bytes, the record validates
against both its schema and governing plan, and Quoin retains and renders the
observation without collapsing its dimensions or non-score fields.

## Metadata

- Priority: P0
- Target Integration: quire-corpus, Quire, and Quoin
- Automation: Automated

## Dependencies

Real extractor, scorer, Quire, Quoin, filesystem, and evidence-store paths are
required. No process, file I/O, schema validation, or evidence intake is mocked.

## Traceability

Verifies [FR-011](../functional/FR-011-graph-quality-observation-schema.md),
[FR-012](../functional/FR-012-governed-graph-quality-producer.md), and
[NFR-005](../non-functional/NFR-005-deterministic-quality-observations.md).
