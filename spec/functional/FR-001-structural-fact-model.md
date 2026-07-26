---
id: FR-001
title: "Per-file structural fact model for Rust, TypeScript/TSX and Python"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-001"
    type: "implements"
  - target: "ix://agent-ix/quire-code-rs/StR-001"
    type: "traces_to"
---

# [FR-001] Per-file structural fact model for Rust, TypeScript/TSX and Python

## Description

The library SHALL parse each supplied source file with a tree-sitter grammar
selected by the file's language and SHALL emit one `code_file` fact per file
together with one `code_module`, `code_function` or `code_type` fact for each
module, callable and type declaration the grammar exposes in that file.

## Inputs

- Source file content, in memory, as UTF-8 text
- The file's repository-relative path, forward-slash separated
- The owning organization and repository names
- The file's language, one of `rust`, `typescript`, `tsx`, `python`

## Outputs

- Structural facts typed `code_file`, `code_module`, `code_function` and
  `code_type`, each carrying a `kind` discriminator that names the concrete
  declaration form (for example `struct`, `enum`, `trait`, `impl`, `class`,
  `interface`, `type_alias`, `method`, `arrow_function`)
- The declaration's start and end line, one-based and inclusive
- Facts in a stable order determined by the declaration's position in the file

## Behavior

- The library SHALL emit exactly one `code_file` fact for every file it accepts,
  including a file whose body declares nothing.
- The library SHALL emit a `code_function` fact for each free function, method,
  and named or assigned function expression the grammar exposes.
- The library SHALL emit a `code_type` fact for each Rust `struct`, `enum`,
  `trait` and type alias, each TypeScript `class`, `interface`, `enum` and type
  alias, and each Python class.
- The library SHALL emit a `code_module` fact for each Rust inline module and
  each TypeScript namespace — that is, for each module declared *within* a file.
- Where a language equates a module with a file, as Python and Rust file-modules
  do, the library SHALL let that file's `code_file` fact stand as the module and
  SHALL NOT emit a second fact for the same construct, so that one source
  construct never yields two node identities.
- Where a declaration is nested inside another declaration, the library SHALL
  record the enclosing declaration so that containment
  ([FR-003](./FR-003-structural-edges.md)) is derivable without re-parsing.
- The library SHALL order the facts emitted for one file by the declaration's
  start position, and SHALL NOT depend on hash-map iteration order for any
  emitted sequence.
- The library SHALL NOT read the filesystem to obtain file content, and SHALL
  NOT resolve the supplied path against the filesystem.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-001-CON-1 | The crate's own code SHALL carry `#![forbid(unsafe_code)]`; tree-sitter's C FFI SHALL remain confined to dependencies | Security | Inspection |
| FR-001-CON-2 | Language support SHALL be added by grammar and query configuration rather than by forking the extraction engine per language | Maintainability | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-001-AC-1 | A Rust fixture yields `code_file`, `code_module`, `code_function` and `code_type` facts covering every declaration it contains | Test (TC-001) |
| FR-001-AC-2 | A TypeScript and a TSX fixture yield facts for classes, interfaces, type aliases, methods and arrow-function consts | Test (TC-002) |
| FR-001-AC-3 | A Python fixture yields facts for its classes, methods and free functions, with the file fact standing as the module | Test (TC-003) |
| FR-001-AC-4 | A file declaring nothing still yields exactly one `code_file` fact | Test (TC-004) |
| FR-001-AC-5 | Every emitted fact carries a `kind` discriminator and one-based inclusive start and end lines | Test (TC-005) |
| FR-001-AC-6 | Facts for one file are ordered by declaration start position across repeated runs | Test (TC-006) |
| FR-001-AC-7 | A dependency audit confirms no filesystem or network crate is reachable from the extraction path | Test (TC-007) |

## Dependencies

- **Upstream**: [US-001](../usecase/US-001-index-a-repository-code-layer.md)
  batch extraction
- **Downstream**: [FR-002](./FR-002-symbol-identity.md) names these facts,
  [FR-003](./FR-003-structural-edges.md) relates them,
  [FR-006](./FR-006-canonical-record-emission.md) emits them
