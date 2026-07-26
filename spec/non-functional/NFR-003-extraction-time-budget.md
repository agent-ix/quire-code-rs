---
id: NFR-003
title: "Extraction time budget for interactive reindexing"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: "ix://agent-ix/quire-code-rs/FR-001"
    type: "constrains"
  - target: "ix://agent-ix/quire-code-rs/FR-008"
    type: "constrains"
---

# [NFR-003] Extraction time budget for interactive reindexing

## Statement

The library SHALL complete a full extraction of a 5,000-file, 500,000-line
mixed-language batch within 60 seconds on a single core, and SHALL complete a
single-file re-extraction within 50 milliseconds at p95.

## Scope

- Applies to: parsing, structural fact extraction, mention harvesting, call
  resolution and record emission for one batch.
- Operational context: a single core of a contemporary developer machine, warm
  process, content already in memory. Excludes consumer-side collection and
  persistence.

## Rationale

Extraction runs inside an interactive application. A full index is tolerable as
a one-time cost at workspace open, but only if it finishes on the order of a
minute rather than an hour; beyond that, consumers will index in the background
indefinitely and users will never see a complete graph. The single-file figure
matters more day to day, because it bounds what happens on save — the cost of
re-extracting one edited file must stay under the threshold at which an editor
feels laggy.

The fixpoint resolution of
[FR-008](../functional/FR-008-type-environments-and-call-resolution.md) is the
component most able to violate this budget, since iteration count interacts with
corpus size; its explicit iteration bound exists partly to keep this requirement
satisfiable.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Full extraction of a 5,000-file / 500,000-line batch, single core | 40 s | 60 s | Benchmark |
| Single-file re-extraction, p95 | 20 ms | 50 ms | Benchmark |
| Peak resident memory during full extraction | 1.0 GB | 2.0 GB | Benchmark |
| Fixpoint iterations to convergence on the benchmark corpus | ≤ 5 | ≤ 10 | Benchmark |

## Verification

A committed benchmark corpus of the stated size is extracted end to end on a
single core, measuring wall time, peak resident memory and fixpoint iteration
count against the thresholds. A separate benchmark re-extracts single files
drawn from that corpus and reports the p95. Benchmarks report measurements on
every run; they gate on threshold only when run in the performance lane, so that
ordinary CI is not made flaky by shared-runner variance.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-003-AC-1 | Full extraction of the benchmark corpus completes within 60 seconds on a single core | Test (TC-062) |
| NFR-003-AC-2 | Single-file re-extraction stays within 50 milliseconds at p95 | Test (TC-063) |
| NFR-003-AC-3 | Peak resident memory during full extraction stays within 2.0 GB | Test (TC-064) |
| NFR-003-AC-4 | Fixpoint resolution converges within 10 iterations on the benchmark corpus | Test (TC-065) |

## Dependencies

- **Upstream**: [FR-001](../functional/FR-001-structural-fact-model.md) parsing
  and [FR-008](../functional/FR-008-type-environments-and-call-resolution.md)
  resolution dominate the measured cost
- **Downstream**: consumer-side full-index budgets, which include this cost plus
  collection and persistence
