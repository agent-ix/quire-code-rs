---
id: MP-001
title: Governed graph-quality observation
type: MeasurementPlan
status: active
owner: quire-code-rs-maintainers
metric: graph_quality
definition_version: quire-code.graph-quality-v1
stage: gate
statistical_design:
  population: every declared supported source and truth relation in one pinned quire-corpus revision
  sampling: complete census with no sampling; preserve language, node-kind, relation-kind, and resolver-tier strata
  repetitions: 2
  estimator: exact confusion-matrix counts and recall derived from the declared truth set
  error_model: corpus omissions, unreadable sources, unsupported languages, ambiguous bindings, and producer defects
  uncertainty: retain raw scorer output and per-dimension counts; do not synthesize an interval for a complete census
  decision_rule: reject a measured run with any wrong heuristic edge; report recall independently; treat non-measured states as no decision
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
