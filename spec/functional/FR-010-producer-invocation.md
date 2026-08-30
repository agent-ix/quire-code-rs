---
id: FR-010
title: "Producer invocation contract"
type: FR
---

# FR-010 — Producer invocation contract

## Description

The quire-code-rs crate SHALL ship an `extract_tree` binary that reads a source
tree and writes canonical records as JSON on standard output.

## Behavior

- When invoked as `extract_tree --org <org> --repo <repo> <dir>`, the binary SHALL
  emit the records of every supported-language file under `<dir>`.
- The binary SHALL write no non-record content on standard output.
- If a supported-language file cannot be read, then the binary SHALL name that
  file on standard error.
- If any supported-language file cannot be read, then the binary SHALL exit
  non-zero after writing the records it did produce.
- When a directory entry is a symbolic link, the binary SHALL leave that entry
  unvisited.

## Rationale

The library takes source *text* and never opens a file (NFR-002), so nothing
inside the boundary can read a tree. Something has to, before any corpus can
grade the output, and that something belongs outside the library.

`agent-ix/quire-corpus` pins this invocation as `producer_contract.version: 1`.
Every governed observation ever recorded against this extractor was measured
through it, so a silent change to the flags or to the shape on stdout
invalidates a body of evidence rather than breaking a build.

A partial tree must never be scoreable as a whole one: reporting the records
and *also* failing is what keeps an unreadable file from becoming a measured
absence.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-010-AC-1 | The documented invocation writes parseable canonical records on stdout and exits zero | Test (TC-101) |
| FR-010-AC-2 | Files are extracted in a stable order regardless of the order the filesystem returns them | Test (TC-102) |
| FR-010-AC-3 | A file in an unsupported language is skipped without a diagnostic and without failing the run | Test (TC-103) |
| FR-010-AC-4 | A missing or malformed argument exits non-zero and writes usage to stderr, never partial records to stdout | Test (TC-104) |
| FR-010-AC-5 | A symbolic link is not followed, so a link to an ancestor cannot make the walk non-terminating | Test (TC-105) |

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-010-CON-1 | The binary SHALL NOT be the only place a behavior is implemented; it reads a tree and calls the library, and owns no extraction logic | Maintainability | Inspection |
| FR-010-CON-2 | A change to the flags or to the stdout shape SHALL be accompanied by a version bump of the corpus producer contract, because prior observations were measured through it | Interface | Inspection |

## Dependencies

- **Upstream**: [FR-006](./FR-006-canonical-record-emission.md) defines the
  canonical records, [NFR-001](../non-functional/NFR-001-determinism.md) defines
  stable output, and [NFR-002](../non-functional/NFR-002-filesystem-only-boundary.md)
  keeps tree walking outside the library API.
- **Downstream**: the pinned quire-corpus producer contract invokes this binary,
  and [FR-012](./FR-012-governed-graph-quality-producer.md) scores its output.
