---
id: NFR-002
title: "No-network boundary: extraction reaches nothing outside its inputs"
type: NFR
quality_attribute: security
relationships:
  - target: "ix://agent-ix/quire-code-rs/FR-001"
    type: "constrains"
  - target: "ix://agent-ix/quire-code-rs/StR-001"
    type: "traces_to"
---

# [NFR-002] No-network boundary: extraction reaches nothing outside its inputs

## Statement

The library SHALL derive its output solely from the inputs passed to it. The
library SHALL NOT open a network connection, read the filesystem, read an
environment variable, or spawn a process.

## Scope

- Applies to: the whole crate and its full transitive dependency closure.
- Operational context: extraction running inside a consumer's process, including
  desktop applications whose security posture constrains what native code may
  reach.

## Rationale

The principal consumer is an application whose security model assumes a small,
auditable native surface and no unsanctioned egress. A parsing library is an
attractive place for that assumption to quietly break, because grammars are
native code and dependency trees grow. Stating the boundary as a measurable
property of the dependency closure — rather than as an intention — is what makes
it enforceable in review and in CI.

The purity this enforces is also what makes determinism
([NFR-001](./NFR-001-determinism.md)) achievable: a function that cannot read the
clock, the environment or the network has far fewer ways to vary.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| HTTP, RPC or socket client crates in the resolved dependency closure | 0 | 0 | architecture-conformance |
| Filesystem, environment or process-spawn calls in extraction paths | 0 | 0 | architecture-conformance |
| Extraction output difference between an online and a network-isolated run | none | none | Test |

## Verification

A dependency audit resolves the full closure and asserts that no HTTP, RPC or
socket client crate appears in it. A source audit over the extraction modules
asserts the absence of filesystem, environment and process-spawn calls. A
functional test extracts a fixture tree with the network unavailable and asserts
the output matches the online run.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-002-AC-1 | The resolved dependency closure contains no HTTP, RPC or socket client crate | Test (TC-059) |
| NFR-002-AC-2 | Extraction paths contain no filesystem read, environment read or process spawn | Test (TC-060) |
| NFR-002-AC-3 | Extraction with the network unavailable produces output identical to the online run | Analysis (TC-061) |

## Dependencies

- **Upstream**: [FR-001](../functional/FR-001-structural-fact-model.md), which
  takes content in memory rather than reading paths
- **Downstream**: `ix://agent-ix/filament-ide-rs/FR-072`, whose dependency-audit
  acceptance criterion rests on this boundary
