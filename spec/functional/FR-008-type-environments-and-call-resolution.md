---
id: FR-008
title: "Type environments and receiver-typed fixpoint call resolution"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-003"
    type: "implements"
  - target: "ix://agent-ix/quire-code-rs/StR-002"
    type: "traces_to"
---

# [FR-008] Type environments and receiver-typed fixpoint call resolution

## Description

The library SHALL build a per-file type environment binding local names to
declared types, SHALL iterate the pending bindings to a fixed point, and SHALL
emit a `calls` edge for a method call only when the receiver's type resolves to
exactly one declaring type after candidate narrowing, omitting the edge in every
other case.

## Inputs

- Structural facts and enclosing-declaration chains for the whole batch
- Declared types of parameters, locals, fields and return values, where the
  language states them
- Resolved `imports` edges from [FR-003](./FR-003-structural-edges.md), used to
  narrow candidates

## Outputs

- `calls` edges carrying `reason` `receiver-typed`, `import-scoped` or
  `name-match` with the corresponding confidence
- `implements_trait` edges from a type to each trait it implements
- `extends` edges from a type to each type it extends
- `references` edges for type mentions that are not calls

## Behavior

- The library SHALL seed each file's environment from that file's declarations,
  its resolved imports, and the declared return types of the functions it calls,
  and SHALL discard the environment once the file's edges are emitted.
- The library SHALL propagate bindings through direct assignment from a bound
  name, assignment from a call's declared return type, assignment from a typed
  field access, and assignment from a method call's declared return type,
  iterating until an iteration produces no new binding.
- The library SHALL bound the number of fixpoint iterations by a configured
  maximum.
- If the iteration bound is reached while bindings are still changing, then the
  library SHALL emit only the edges resolved so far rather than continue with
  partially propagated bindings.
- The library SHALL isolate bindings introduced inside a callable from the
  file-level environment, so that a name rebound locally SHALL NOT alter
  resolution elsewhere in the file.
- Where the receiver's type resolves to exactly one declaring type, the library
  SHALL emit the `calls` edge with `reason` `receiver-typed`.
- Where the receiver's type does not resolve and exactly one type reachable
  through the file's resolved imports declares a method of that name, the
  library SHALL emit the `calls` edge with `reason` `import-scoped`.
- Where neither the receiver's type nor the imported candidates resolve, and
  exactly one type in the whole batch declares a method of that name, the
  library SHALL emit the `calls` edge with `reason` `name-match`.
- If more than one candidate survives narrowing at any tier, then the library
  SHALL emit no edge for that call site.
- The library SHALL resolve a call to a declaration in the same file without
  consulting other files, so that same-file resolution is unaffected by batch
  composition.
- Where the file being resolved declares a type or method matching the
  candidate, the library SHALL prefer that declaration over any batch-wide
  candidate, so that a simple name occurring in more than one file — including
  the same name in two languages — SHALL NOT suppress the relationships within
  each file that declares it.
- Where a Rust method is invoked through a trait object or a generic parameter,
  and the concrete implementation is therefore not determined by the source, the
  library SHALL emit no `calls` edge.
- The library SHALL configure per-language behavior through data rather than
  through separate resolution engines per language.
- The library SHALL resolve only against the facts present in the supplied
  batch, and SHALL report the batch's file count and the number of call sites
  left unresolved, so that a consumer supplying a partial batch can see that
  resolution was bounded by what it provided rather than by the source.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-008-CON-1 | Resolution SHALL be conservative: an ambiguous call site SHALL yield no edge, never a highest-scoring guess | Reliability | Test (TC-045) |
| FR-008-CON-2 | The fixpoint iteration SHALL carry an explicit bound, and reaching it SHALL degrade to fewer edges rather than to unbounded work | Performance | Test (TC-049) |
| FR-008-CON-3 | Resolution quality SHALL be a function of the supplied batch; a consumer seeking whole-repository resolution SHALL supply the whole repository | Interface | Test (TC-052) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-008-AC-1 | A method called on a variable declared with a type from another file yields a `calls` edge with `reason` `receiver-typed` | Test (TC-044) |
| FR-008-AC-2 | A call whose receiver type is unrecoverable, where several types declare that method name, yields no edge | Test (TC-045) |
| FR-008-AC-3 | A binding assigned from another bound name resolves through the fixpoint | Test (TC-046) |
| FR-008-AC-4 | A binding assigned from a call's declared return type resolves through the fixpoint | Test (TC-047) |
| FR-008-AC-5 | A name rebound inside a callable does not change resolution at file level | Test (TC-048) |
| FR-008-AC-6 | A cyclic set of bindings terminates at the iteration bound and emits only the edges resolved so far | Test (TC-049) |
| FR-008-AC-7 | Rust `impl` blocks yield `implements_trait` edges and TypeScript subclasses yield `extends` edges | Test (TC-050) |
| FR-008-AC-8 | A call through a Rust trait object yields no `calls` edge | Test (TC-051) |
| FR-008-AC-9 | Same-file resolution produces identical edges whether or not unrelated files are present in the batch | Test (TC-052) |
| FR-008-AC-10 | Each resolution tier stamps its own `reason`, and `receiver-typed` outranks `import-scoped`, which outranks `name-match` | Test (TC-053) |
| FR-008-AC-11 | The result reports the batch file count and the unresolved call-site count | Test (TC-073) |
| FR-008-AC-12 | A simple type name declared in two files still resolves within each file that declares it | Test (TC-074) |

## Dependencies

- **Upstream**: [FR-001](./FR-001-structural-fact-model.md),
  [FR-002](./FR-002-symbol-identity.md),
  [FR-003](./FR-003-structural-edges.md)
- **Downstream**: [FR-004](./FR-004-edge-provenance-and-dedupe.md) deduplicates
  the edges; [NFR-004](../non-functional/NFR-004-conservative-resolution-precision.md)
  bounds the wrong ones
