---
id: FR-002
title: "Org-qualified symbol identity and naming scheme"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-001"
    type: "implements"
  - target: "ix://agent-ix/quire-code-rs/FR-001"
    type: "references"
---

# [FR-002] Org-qualified symbol identity and naming scheme

## Description

The library SHALL name every structural fact with the qualified form
`{org}/{repo}/{relative/path}::{Parent}::{symbol}`, in which the path is
repository-relative and forward-slash separated and each enclosing declaration
contributes one `::`-separated segment, so that a fact's name is unique under
the consuming graph's `(object_type, container, name)` identity rule.

## Inputs

- The organization and repository names supplied with the batch
- The file's repository-relative path
- The chain of enclosing declarations for the symbol being named

## Outputs

- A qualified name string per structural fact
- An `ix://` reference of the form `ix://{org}/{repo}/{qualified-name}` for each
  fact, carrying at least three segments

## Behavior

- The library SHALL prefix every qualified name with the organization, so that
  identically named repositories owned by different organizations SHALL NOT
  produce colliding names.
- The library SHALL name a `code_file` fact with the org, repo and path alone,
  with no `::` segment.
- The library SHALL append one `::`-separated segment for each enclosing
  declaration, outermost first, ending with the symbol's own name.
- Where a Rust method is declared in an `impl` block, the library SHALL use the
  implementing type as the parent segment, and where the `impl` block implements
  a trait, the library SHALL record the trait separately rather than in the
  name, so that inherent and trait methods of one type share a naming scheme.
- Where a declaration is anonymous, the library SHALL synthesize a segment from
  the declaration kind and its one-based start line, so that the name stays
  stable for unchanged input and no two anonymous declarations in one file
  collide.
- The library SHALL produce the same qualified name for an unchanged declaration
  across runs, machines and rebuilds.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-002-CON-1 | Path segments SHALL be forward-slash separated regardless of host platform | Portability | Test |
| FR-002-CON-2 | `ix://` references SHALL carry at least three segments, matching the ecosystem's last-segment resolution rule | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-002-AC-1 | A free function in `src/lib.rs` of `agent-ix/quire-code-rs` is named `agent-ix/quire-code-rs/src/lib.rs::extract` | Test (TC-008) |
| FR-002-AC-2 | A method on a type is named with the type as its parent segment, for both inherent and trait `impl` blocks | Test (TC-009) |
| FR-002-AC-3 | A `code_file` fact's name carries the org, repo and path and no `::` segment | Test (TC-010) |
| FR-002-AC-4 | The same repository extracted under two different orgs produces disjoint name sets | Test (TC-011) |
| FR-002-AC-5 | Two anonymous declarations in one file receive distinct, line-derived, run-stable segments | Test (TC-012) |
| FR-002-AC-6 | A Windows-style path supplied by a consumer is normalized to forward slashes in emitted names | Test (TC-013) |
| FR-002-AC-7 | Every emitted `ix://` reference has at least three segments | Test (TC-014) |

## Dependencies

- **Upstream**: [FR-001](./FR-001-structural-fact-model.md) supplies the
  declarations being named
- **Downstream**: [FR-003](./FR-003-structural-edges.md) and
  [FR-008](./FR-008-type-environments-and-call-resolution.md) relate facts by
  these names; [FR-006](./FR-006-canonical-record-emission.md) hashes them into
  record ids
