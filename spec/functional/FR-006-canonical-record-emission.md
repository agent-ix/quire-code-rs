---
id: FR-006
title: "Canonical record emission at quire-rs parity"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-001"
    type: "implements"
  - target: "ix://agent-ix/quire-rs/spec"
    type: "references"
---

# [FR-006] Canonical record emission at quire-rs parity

## Description

The library SHALL emit its facts and edges as canonical graph records in the
same shape `quire-rs` produces for markdown artifacts — node records carrying a
stable identifier, an `ix://` reference, an object type and typed data, and edge
records carrying source and target references, an edge type and provenance — so
that a consumer writes code records and specification records through one path.

## Inputs

- Structural facts named per [FR-002](./FR-002-symbol-identity.md)
- Deduplicated, provenance-bearing edges per
  [FR-004](./FR-004-edge-provenance-and-dedupe.md)
- Mention records per [FR-005](./FR-005-spec-mention-harvesting.md)

## Outputs

- Node records with a lowercase-hexadecimal SHA-256 identifier, an `ix://`
  reference, one of the object types `code_file`, `code_module`,
  `code_function`, `code_type`, and a data payload carrying the `kind`
  discriminator and line span
- Edge records with `source_ref`, `edge_type`, `target_ref` and the provenance
  fields
- A per-file diagnostics list

## Behavior

- The library SHALL derive a node's identifier by hashing its object type and
  qualified name with SHA-256 and rendering the digest as lowercase
  hexadecimal, so that identifiers are stable across runs and machines and
  independent of extraction order.
- The library SHALL NOT include line spans, file content, or any other mutable
  attribute in the hashed identity, so that moving a declaration within a file
  preserves its identifier.
- The library SHALL order node records by object type and then by qualified
  name, and edge records by source reference, edge type and then target
  reference, so that the serialized output is stable.
- The library SHALL emit edge types drawn from the set `contains`, `imports`,
  `calls`, `implements_trait`, `extends`, `references`, and SHALL carry mention
  edges as `references`.
- The library SHALL serialize records without timestamps, absolute paths,
  hostnames, process identifiers, or any other environment-derived value.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-006-CON-1 | Node identity SHALL be a pure function of `(object_type, qualified_name)` | Interface | Test (TC-087) |
| FR-006-CON-2 | The emitted edge-type set SHALL match the set consumed by `ix://agent-ix/filament-ide-rs/FR-072` exactly | Interface | Test (TC-036) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-006-AC-1 | Every node record carries a 64-character lowercase-hexadecimal identifier, an `ix://` reference and a `kind` in its data | Test (TC-033) |
| FR-006-AC-2 | Moving a declaration to a different line preserves its node identifier while updating its line span | Test (TC-034) |
| FR-006-AC-3 | Node and edge records appear in the specified stable order across repeated runs | Test (TC-035) |
| FR-006-AC-4 | Every emitted edge type is drawn from the six-value set | Test (TC-036) |
| FR-006-AC-5 | Serialized output contains no timestamp, absolute path, hostname or process identifier | Test (TC-037) |
| FR-006-AC-6 | Records emitted for a fixture match a committed golden file byte for byte | Test (TC-038) |

## Dependencies

- **Upstream**: [FR-002](./FR-002-symbol-identity.md),
  [FR-004](./FR-004-edge-provenance-and-dedupe.md),
  [FR-005](./FR-005-spec-mention-harvesting.md)
- **Downstream**: [NFR-001](../non-functional/NFR-001-determinism.md) measures
  the stability this requirement establishes;
  `ix://agent-ix/filament-ide-rs/FR-072` consumes the records
