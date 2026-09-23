---
id: MP-001
title: Governed graph-quality observation
type: MeasurementPlan
status: active
owner: quire-code-rs-maintainers
metric: graph_quality
definition_version: quire-code.graph-quality-v2
stage: gate
ground_truth_kind: human-labelled
objective:
  direction: zero
statistical_design:
  population: every declared supported source and truth relation in one pinned quire-corpus revision
  sampling: complete census with no sampling; preserve language, node-kind, relation-kind, and resolver-tier strata
  repetitions: 2
  estimator: count
  error_model: corpus omissions, unreadable sources, unsupported languages, ambiguous bindings, and producer defects
  uncertainty: retain raw scorer output and per-dimension counts; do not synthesize an interval for a complete census
  decision_rule:
    comparator: eq
    threshold: 0
protected_apparatus:
  - Makefile
  - src/bin/measure_graph_quality.rs
  - src/measurement.rs
  - schemas/graph-quality-observation-v1.schema.json
negative_controls:
  - kind: suppressed-observation
    description: >-
      the producer rejects a scorer report whose scored-case count does not
      equal its complete case-digest map (`validate_complete_report`), so a
      collection cannot drop an unfavourable case out of the census
  - kind: apparatus-edit
    description: >-
      the producer binary, the module that turns the scorer report into the
      observation and its decision-bound count, and the observation schema
      are protected, so editing the grading logic alongside a change it
      grades changes the recorded digests
  - kind: stale-evidence
    description: >-
      the CLI refuses to run unless the source and corpus checkouts are
      clean and pinned to the exact declared revisions
      (`verify_clean_source`), so a result cannot be presented for a
      revision it was not collected against
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-012
    type: measures
  - target: ix://agent-ix/quire-code-rs/NFR-004
    type: references
---

# Governed graph-quality observation

## Decision Use

The observation determines whether a pinned extractor revision preserves the
zero-wrong-heuristic-edge invariant on the pinned corpus. Recall is a separate
diagnostic used to prioritize improvement; it does not offset a wrong edge and
has no pass threshold in this plan.

An `empty`, `unreadable`, or `unsupported` population makes no quality decision.

## Decision Rule

`statistical_design.estimator: count` is the number of wrong heuristic edges:
the scorer's overall edge false-positive count across the whole census. The
rule holds when that count equals zero (`comparator: eq`, `threshold: 0`),
matching `objective.direction: zero`; the producer publishes the outcome as the
`precision_decision` observation. The rule is evaluated only for a `measured`
population; a non-measured state makes no decision rather than counting as
zero. Recall is not part of the rule.

## Population

The population is the complete declared set of supported source files, expected
nodes, expected relations, and negative or ambiguous cases in one pinned
quire-corpus revision. No source or truth record is sampled out.

The census is stratified by language, node kind, relation kind, and resolver
tier. Files outside the supported language set remain visible in the population
state but do not become true negatives. An unreadable supported file invalidates
measurement of the whole declared population.

The truth-population census uses the scorer's recovered plus missing truth
records, excluding producer-only false positives. Unresolved counts are the
extractor's reported unresolved calls in the corpus's authored ambiguous-call
cases; ambiguous counts are those cases' expected ambiguous call sites. Each is
an exact marginal census of `call_site` / `calls` / `unresolved`, stratified by
the case language.

The corpus's expected nodes, relations, and ambiguous-call cases are
hand-authored fixtures (`ground_truth_kind: human-labelled`), not derived
mechanically from another oracle.

## Protected Apparatus (PLAT-1008)

`protected_apparatus` names the files that produce this plan's number: the
producer binary (`src/bin/measure_graph_quality.rs`), the module that turns
the scorer report into the observation and its decision-bound false-positive
count (`src/measurement.rs`), the observation schema
(`schemas/graph-quality-observation-v1.schema.json`), and the `Makefile`
target (`check-measurement`) that builds and runs them. Editing one of these
alongside a change it grades changes the recorded digests rather than earning
silent credit.

`negative_controls` declares the gaming scenarios this plan guards against:
suppressing an unfavourable case (the producer refuses a scorer report whose
scored-case count disagrees with its case-digest map), editing the protected
apparatus alongside the change it grades, and presenting a result against a
source or corpus revision it was not actually collected against
(`verify_clean_source` refuses a dirty or mismatched checkout).

## Collection Procedure

1. Record the exact extractor, producer contract, parser grammar, configuration,
   source, corpus, scorer, and measurement-definition revisions.
2. Run the corpus scorer twice with those pinned inputs in an isolated filesystem
   environment, once after reversing tracked-file creation order. Reject either
   report unless its scored-case count equals its complete case-digest map.
3. Retain each raw scorer output by relative path and content digest.
4. Compare the two canonical observation records byte for byte.
5. Validate the record against
   [FR-011](../functional/FR-011-graph-quality-observation-schema.md) and validate
   this plan with Quire before handing the observation to Quoin.

## Interpretation

Any false-positive heuristic edge fails the decision rule. Recall is reported by
dimension and overall, but a low recall result does not authorize lowering the
precision invariant. Unresolved and ambiguous counts remain separate from false
negatives so the reason for missing recovery stays inspectable.

Non-measured states carry no confusion matrices or recall. Corpus defects,
producer failures, exclusions, and other limitations remain attached to the raw
observation rather than being normalized into zeros.
