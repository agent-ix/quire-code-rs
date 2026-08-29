---
id: NFR-001
title: "Determinism: identical input produces byte-identical output"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quire-code-rs/FR-006"
    type: "constrains"
  - target: "ix://agent-ix/quire-code-rs/StR-001"
    type: "traces_to"
---

# [NFR-001] Determinism: identical input produces byte-identical output

## Statement

The library SHALL produce byte-identical serialized output for identical input
across repeated runs, across threads, across processes, across machines and
across rebuilds.

## Scope

- Applies to: every record, edge, mention and diagnostic the extraction API
  returns, and their serialized form.
- Operational context: any host, any core count, any batch ordering supplied by
  the consumer.

## Rationale

Consumers reindex constantly and skip writes when extraction output matches what
they already stored. That optimization is only sound if identical input really
does produce identical output; otherwise every re-scan rewrites unchanged rows,
invalidates caches, and buries real changes in noise. Determinism is also what
makes a golden-file suite possible, and the golden files are how the contract
stays honest as languages are added.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Serialized output equality across 100 repeated extractions of one fixture tree | all equal | all equal | Test |
| Serialized output equality across extractions driven from 8 concurrent threads | all equal | all equal | Test |
| Serialized output equality under shuffled input batch ordering | all equal | all equal | Test |
| Occurrences of order-observable `HashMap`/`HashSet` iteration in extraction paths | 0 | 0 | architecture-conformance |
| Occurrences of clock, randomness, process or environment reads in extraction paths | 0 | 0 | architecture-conformance |

## Verification

A determinism suite extracts a mixed-language fixture tree repeatedly — serially,
concurrently, and with the input batch shuffled — and asserts byte equality of
the serialized output across every run. A static audit over the extraction
modules asserts the absence of order-observable hash iteration and of any read
from the clock, a random source, the process table or the environment.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-001-AC-1 | One hundred repeated extractions of a fixture tree serialize byte-identically | Test (TC-054) |
| NFR-001-AC-2 | Extractions driven from eight concurrent threads serialize byte-identically | Test (TC-055) |
| NFR-001-AC-3 | Shuffling the input batch order leaves the serialized output unchanged | Test (TC-056) |
| NFR-001-AC-4 | A static audit finds no order-observable hash iteration in extraction paths | Test (TC-057) |
| NFR-001-AC-5 | A static audit finds no clock, randomness, process or environment read in extraction paths | Test (TC-058) |

## Dependencies

- **Upstream**: [FR-006](../functional/FR-006-canonical-record-emission.md)
  defines the ordering and identity rules this requirement measures
- **Downstream**: consumer-side incremental reindexing, which is only correct
  when this holds
