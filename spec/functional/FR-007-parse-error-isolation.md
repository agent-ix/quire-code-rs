---
id: FR-007
title: "Per-file parse-error isolation"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-001"
    type: "implements"
---

# [FR-007] Per-file parse-error isolation

## Description

If a file fails to parse or produces a syntax tree containing error nodes, then
the library SHALL emit a parser-error diagnostic naming that file and SHALL
continue extracting the remaining files in the batch, and it SHALL NOT panic on
any input.

## Inputs

- A batch of source files, any subset of which may be malformed, truncated,
  written in a language the file extension misidentifies, or not valid UTF-8

## Outputs

- A parser-error diagnostic carrying the file path, a message, and the
  one-based line and column of the first error position where the grammar
  reports one
- Complete facts and edges for every file that parsed

## Behavior

- If a file's syntax tree contains error nodes, then the library SHALL emit a
  diagnostic for that file and SHALL suppress that file's facts rather than
  emitting facts recovered from a partially valid tree, so that a consumer never
  writes records derived from an unparseable file.
- The library SHALL continue the batch after a per-file failure, and the batch
  result SHALL carry both the successful records and the diagnostics.
- The library SHALL return a diagnostic, rather than propagate a panic, for
  input that is not valid UTF-8, that is empty, or that exceeds the
  configured size bound.
- The library SHALL NOT allow a failure in one file to alter the records
  emitted for any other file in the batch.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-007-CON-1 | No extraction entry point SHALL panic on arbitrary byte input | Reliability | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-007-AC-1 | A batch containing one syntactically invalid file yields a diagnostic for it and complete records for the others | Test (TC-039) |
| FR-007-AC-2 | The diagnostic carries the file path and the first error position where the grammar reports one | Test (TC-040) |
| FR-007-AC-3 | A file containing error nodes contributes no facts | Test (TC-041) |
| FR-007-AC-4 | Records for the healthy files are identical whether or not the malformed file is present in the batch | Test (TC-042) |
| FR-007-AC-5 | Arbitrary byte input, including invalid UTF-8 and empty input, yields a diagnostic and no panic | Test (TC-043) |

## Dependencies

- **Upstream**: [FR-001](./FR-001-structural-fact-model.md) parsing
- **Downstream**: [FR-006](./FR-006-canonical-record-emission.md) carries the
  diagnostics alongside records
