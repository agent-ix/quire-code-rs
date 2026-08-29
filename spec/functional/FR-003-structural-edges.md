---
id: FR-003
title: "Containment and path-resolved import edges"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-001"
    type: "implements"
  - target: "ix://agent-ix/quire-code-rs/FR-002"
    type: "references"
---

# [FR-003] Containment and path-resolved import edges

## Description

The library SHALL emit a `contains` edge from each structural fact to every
declaration syntactically nested within it, and SHALL emit an `imports` edge
from a file to each module or symbol it imports whenever the import specifier
resolves to a file present in the same extraction batch.

## Inputs

- The structural facts and enclosing-declaration chains from
  [FR-001](./FR-001-structural-fact-model.md)
- Import, `use`, and `from … import` statements recovered per file
- The set of repository-relative paths present in the batch

## Outputs

- `contains` edges, one per nesting relationship, carrying `reason` `syntactic`
  and confidence 1.0
- `imports` edges for resolved specifiers, carrying `reason` `path-resolved` and
  confidence 1.0

## Behavior

- The library SHALL emit `contains` from a `code_file` fact to each top-level
  declaration in that file, and from each declaration to each declaration nested
  directly within it, so that containment forms a tree rooted at the file.
- The library SHALL resolve a relative import specifier against the importing
  file's directory, applying the language's extension and index-file
  conventions, and SHALL emit an `imports` edge when the result names a path
  present in the batch.
- If a relative import specifier does not resolve to a path present in the
  batch, then the library SHALL omit the edge rather than emit an edge to a
  synthesized target, and SHALL record the unresolved specifier as a diagnostic.
- Where a specifier is package-absolute or bare, the library SHALL omit the edge
  and SHALL NOT record a diagnostic, because an external dependency is expected
  rather than anomalous and diagnosing every one would bury the relative-import
  failures that indicate a genuinely incomplete batch.
- The library SHALL mark every edge emitted under this requirement with
  confidence 1.0, because both relationships are read directly from syntax
  rather than inferred.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-003-CON-1 | Import resolution SHALL consult only the batch's path set and SHALL NOT stat the filesystem or read package manifests | Security | Test (TC-060) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-003-AC-1 | A nested declaration yields a `contains` edge from its immediate parent, and containment over a file forms a tree | Test (TC-015) |
| FR-003-AC-2 | A relative import naming a file in the batch yields an `imports` edge with `reason` `path-resolved` | Test (TC-016) |
| FR-003-AC-3 | An import of a third-party package yields no edge and no diagnostic, while an unresolvable relative import yields no edge and one diagnostic | Test (TC-017) |
| FR-003-AC-4 | A relative import written without an extension resolves through the language's extension and index conventions | Test (TC-018) |
| FR-003-AC-6 | A declaration's containment parent is the declaration its qualified name names, in every language | Test (TC-092) |
| FR-003-AC-5 | Every `contains` and `imports` edge carries confidence 1.0 | Test (TC-019) |

## Dependencies

- **Upstream**: [FR-001](./FR-001-structural-fact-model.md) facts,
  [FR-002](./FR-002-symbol-identity.md) names
- **Downstream**: [FR-004](./FR-004-edge-provenance-and-dedupe.md) governs the
  provenance these edges carry;
  [FR-008](./FR-008-type-environments-and-call-resolution.md) narrows call
  candidates using resolved imports
