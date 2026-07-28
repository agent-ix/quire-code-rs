---
id: FR-009
title: "Declaration visibility and normalized callable signature"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-001"
    type: "implements"
  - target: "ix://agent-ix/quire-code-rs/StR-001"
    type: "traces_to"
  - target: "ix://agent-ix/quire-code-rs/FR-001"
    type: "references"
  - target: "ix://agent-ix/quire-code-rs/FR-006"
    type: "references"
---

# [FR-009] Declaration visibility and normalized callable signature

## Description

The library SHALL record, for every structural declaration it emits, the
declaration's visibility as its own language expresses it, and SHALL record for
every callable a normalized signature summarizing its parameter types and
return type, so that a consumer can distinguish a change to a file's exported
surface from a change confined to a body.

The motivating consumer is `ix://agent-ix/filament-ide-rs/FR-074` export-set
change tiering: when a source file is edited, the files that call into it need
to re-resolve their edges only if the edited file's *exported* symbol set
changed. Without visibility, a new private helper reads as an export and
over-invalidates; without a signature, changing a parameter's type reads as a
body-only edit and under-invalidates.

## Inputs

- The declaration node recovered per
  [FR-001](./FR-001-structural-fact-model.md)
- The file's language, one of `rust`, `typescript`, `tsx`, `python`

## Outputs

- A `visibility` value on every structural fact and node record, one of
  `public`, `crate`, `private`
- A `signature` value on every `code_function` fact and node record whose
  grammar exposes a parameter list, absent on every other declaration

## Behavior

### Visibility

- The library SHALL classify a declaration as `public` when its language marks
  it as visible outside its defining module, as `crate` when its language marks
  it as visible within the defining compilation unit or package but not beyond
  it, and as `private` otherwise.
- For Rust, the library SHALL classify a declaration carrying `pub` as
  `public`, a declaration carrying `pub(crate)`, `pub(super)` or `pub(in …)` as
  `crate`, and a declaration carrying no visibility modifier as `private`.
- For Rust, the library SHALL classify an associated item declared in a trait
  body, or in an `impl Trait for Type` block, as `public`, because such an item
  carries the trait's visibility rather than its own and a caller reaches it
  through the trait. An item in an inherent `impl Type` block SHALL be
  classified by its own modifier.
- For TypeScript and TSX, the library SHALL classify a declaration carrying
  `export` as `public`, a class member carrying `private` or `#`-prefixed
  private-name syntax as `private`, a class member carrying `protected` as
  `crate`, a class member carrying no access modifier as `public`, and any
  other unexported top-level declaration as `private`.
- For Python, which has no visibility keyword, the library SHALL classify a
  declaration by the documented naming convention: a name beginning with two
  underscores and not ending with two underscores as `private`, a name
  beginning with one underscore as `crate`, and every other name as `public`.
- The library SHALL classify a `code_file` fact as `public`, because a file is
  the unit a consumer addresses from outside.

### Signature

- The library SHALL render a callable's signature as its parameter list in
  declaration order, parenthesized and comma-separated, followed by ` -> ` and
  the return type when the declaration states one.
- The library SHALL render each parameter as its declared type when the grammar
  exposes one, and as the parameter's binding name otherwise, so that a
  callable in an unannotated language still contributes its arity.
- The library SHALL render a method receiver (`self`, `&self`, `&mut self`,
  `this`, `cls`) as `self`, so that a receiver's spelling is not mistaken for a
  signature change.
- The library SHALL collapse every run of whitespace within a rendered type to
  a single space and SHALL strip comments, so that reformatting a declaration
  does not change its signature.
- The library SHALL omit the signature entirely for a declaration whose grammar
  exposes no parameter list, including every `code_file`, `code_module` and
  `code_type` fact.

### Identity

- The library SHALL NOT include visibility or the signature in a node's hashed
  identity, so that a declaration that changes visibility or signature keeps its
  identifier and is observed as a modification rather than as a delete and an
  add.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-009-CON-1 | Both fields SHALL be additive to the existing serialized record shape, so a consumer that ignores them is unaffected | Interface | Test |
| FR-009-CON-2 | Visibility classification SHALL be driven by per-language configuration rather than by forking the extraction engine, per FR-001-CON-2 | Maintainability | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-009-AC-1 | A Rust fixture classifies `pub` as `public`, `pub(crate)` and `pub(super)` as `crate`, and a bare declaration as `private` | Test (TC-078) |
| FR-009-AC-2 | A TypeScript fixture classifies `export` as `public`, an unexported declaration as `private`, and class members by their access modifier with no modifier meaning `public` | Test (TC-079) |
| FR-009-AC-3 | A Python fixture classifies `__helper` as `private`, `_helper` as `crate`, `helper` and `__init__` as `public` | Test (TC-080) |
| FR-009-AC-4 | A Rust trait's associated items, and the items of a trait `impl`, are `public` with no modifier, while an unmarked inherent-`impl` item is `private` | Test (TC-081) |
| FR-009-AC-5 | A callable's signature renders declared parameter types and return type in declaration order, with a receiver rendered as `self` | Test (TC-082) |
| FR-009-AC-6 | Reformatting a declaration across lines, and adding a comment inside its parameter list, leaves its signature byte-identical | Test (TC-083) |
| FR-009-AC-7 | An unannotated Python callable renders its parameter names, and a callable with no parameter list carries no signature at all | Test (TC-084) |
| FR-009-AC-8 | Changing a parameter's type changes the signature while preserving the node identifier; adding a private helper leaves every existing node record unchanged | Test (TC-085) |
| FR-009-AC-9 | Records serialized before this requirement deserialize unchanged, and a record carrying neither field round-trips | Test (TC-086) |

## Dependencies

- **Upstream**: [FR-001](./FR-001-structural-fact-model.md) supplies the
  declaration nodes this requirement annotates;
  [FR-002](./FR-002-symbol-identity.md) supplies the names
- **Downstream**: [FR-006](./FR-006-canonical-record-emission.md) carries both
  fields on the emitted records; `ix://agent-ix/filament-ide-rs/FR-074`
  consumes them for export-set change tiering
