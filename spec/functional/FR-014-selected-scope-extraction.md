---
id: FR-014
title: "Selected-scope extraction against a typed manifest"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-001"
    type: "implements"
---

# [FR-014] Selected-scope extraction against a typed manifest

## Description

The library SHALL extract the records of a selected set of production modules and
functions against an explicit, typed selection manifest, and SHALL report whether
the selection was covered in full, in part, or not at all. The scope result is
produced by the extractor's own traversal of the manifest and the supplied files;
a caller cannot set it.

## Inputs

- A `SelectionManifest`: a selection identity and the selected modules, each with
  its org, repo, repository-relative path, the content identity (SHA-256 of the
  file content) the selection was made against, and optionally selected function
  names.
- The candidate source files the caller holds.

## Outputs

- An `ExtractionOutcome` carrying the selection identity, a scope, the files
  supplied outside the manifest, and the extraction result for the accepted files.
- Scope `Full`, `Partial { missing }` or `Unavailable { reason }`.

## Behavior

- The manifest is authoritative. The set of selected modules is never inferred
  from the supplied files.
- A selected module is accounted for only when a supplied file has its identity
  key, its content identity equals the manifest's, and it parses with no
  diagnostics. A selected function is accounted for only when its module is
  accounted for and the module declares it.
- The scope SHALL be `Full` only when every selected module and function is
  accounted for.
- If some but not all of the selection is accounted for, then the scope SHALL be
  `Partial` listing each unaccounted entry with its cause: not supplied, identity
  mismatch (expected and actual), unparsable (with the diagnostics), or function
  absent.
- If the manifest is empty or names a module more than once, or no selected
  module is accounted for, then the scope SHALL be `Unavailable` with a typed
  reason.
- A supplied file outside the manifest, or a second file for an already supplied
  key, SHALL be listed as extra and SHALL NOT contribute records. A file whose
  identity differs from the manifest or that fails to parse SHALL NOT contribute
  records.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-014-AC-1 | Every selected module present, matching and clean yields `Full` and the records of exactly those files | Test (TC-177) |
| FR-014-AC-2 | An omitted selected module yields `Partial` listing it as not supplied | Test (TC-178) |
| FR-014-AC-3 | A supplied file outside the manifest is listed extra and contributes no records | Test (TC-179) |
| FR-014-AC-4 | A file whose content identity differs from the manifest is listed as an identity mismatch with expected and actual identities and contributes no records | Test (TC-180) |
| FR-014-AC-5 | An unparsable selected file yields `Partial` listing it unparsable with its diagnostics | Test (TC-181) |
| FR-014-AC-6 | A selection with no accounted module, an empty manifest, or a duplicate module entry yields `Unavailable` with a typed reason | Test (TC-182) |
| FR-014-AC-7 | A selected function the accounted module does not declare is listed absent and the scope is `Partial`; the outcome preserves the selection identity | Test (TC-183) |

## Dependencies

- **Upstream**: [FR-001](./FR-001-structural-fact-model.md) facts,
  [FR-007](./FR-007-parse-error-isolation.md) diagnostics
