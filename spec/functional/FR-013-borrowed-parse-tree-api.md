---
id: FR-013
title: "Borrowed parse-tree API for external consumers"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-005"
    type: "implements"
---

# [FR-013] Borrowed parse-tree API for external consumers

## Description

When a consumer supplies a language selector, a file identifier and source
text, the `quire-code-parse` crate SHALL parse the text with the matching
tree-sitter grammar and return a value that borrows that text and the
resulting syntax tree, and it SHALL NOT perform fact extraction, symbol
classification, marker binding, or any other analysis over the result. Where
the tree's declaration structure could not be resolved, the crate SHALL
report a named diagnostic naming the file and a line, and SHALL still hand
back the tree tree-sitter produced — a diagnostic is never satisfied by
withholding the tree it is about.

## Inputs

- A `Language` selector naming one grammar compiled into the crate
- A file identifier, used only to attribute a diagnostic
- Source text, borrowed for the lifetime the caller chooses

## Outputs

- A `ParsedFile` value borrowing the parsed `tree_sitter::Tree` and the source
  text, giving the consumer the same `Node`/`TreeCursor` traversal API this
  crate uses internally
- On a declaration-structure failure, a named `ParseError::Syntax` carrying
  the file identifier, a one-based line number, and the `ParsedFile` produced
  despite the error

## Behavior

- The crate SHALL accept a caller-supplied `Language`, file identifier and
  source text, and SHALL NOT read from the filesystem or the network to
  satisfy the call.
- The crate SHALL return a `ParsedFile` value that borrows the source text
  passed to it for the lifetime the caller chooses, rather than copying it or
  retaining an owned copy inside the crate.
- Where the root of the produced syntax tree, or one of its direct
  (top-level) children, is itself an `ERROR` or `MISSING` node — meaning
  tree-sitter could not resolve even the identity of a top-level item there —
  the crate SHALL return `Err` with a named diagnostic carrying the file
  identifier and the one-based line of the first error position, rather than
  an `Ok` value the caller would otherwise have to inspect for silence. That
  `Err` SHALL carry the `ParsedFile` tree-sitter produced despite the error, so
  a caller forced to acknowledge the diagnostic is never also locked out of
  the tree it names.
- Where an error or missing node exists only inside a declaration's own body —
  its own node kind, name and signature remain resolvable, only an expression
  or statement within it does not — the crate SHALL return `Ok` with the full
  tree, rather than treating any error anywhere in the tree as a failure. A
  body-local error remains visible to a caller that inspects the relevant
  node's own error state directly through the re-exported `tree-sitter` API.
- The crate SHALL NOT classify a node, attribute a declaration to a symbol
  kind, bind a marker to a declaration, or perform any analysis beyond
  producing the tree; that judgment belongs to the consumer, the same
  boundary [FR-005-CON-1](./FR-005-spec-mention-harvesting.md) states for
  this repository's own extraction pipeline.
- The crate SHALL expose the `tree-sitter` crate's own `Node` and
  `TreeCursor` traversal API directly, rather than reimplementing an
  equivalent traversal surface.
- The crate SHALL produce a syntax tree whose structure renders identically
  for two calls given identical `Language`, file identifier and source-text
  bytes, so a consumer's own derived output is stable across repeated parsing
  of unchanged content.
- The crate SHALL NOT panic for any source text a caller supplies.
- The crate SHALL make each supported language reachable only through a
  like-named Cargo feature, so a consumer selecting a subset of languages
  compiles and links only the grammars it selected.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-013-CON-1 | The crate SHALL NOT impose this repository's own `load → resolve → emit → drop` discipline (ADR-002) on a caller of this API; ADR-002 governs this repository's own extraction pipeline only, and this crate places no upper bound on how long a caller retains the returned `ParsedFile` and its borrowed source | Interface | Inspection |
| FR-013-CON-2 | The crate SHALL depend on no fact-model, type-environment, call-resolution or record-emission code, so a consumer wanting only a syntax tree does not compile them | Structural | Inspection (`cargo tree`) |
| FR-013-CON-3 | Each supported language's grammar SHALL be reachable only through a like-named Cargo feature (`rust`, `python`, `typescript`), enforced by gating each `Language` variant on that feature | Structural | Test (TC-137, TC-138) |
| FR-013-CON-4 | Mapping a mention, a symbol, or any classification against the returned tree belongs to the consumer, per [FR-005-CON-1](./FR-005-spec-mention-harvesting.md) | Interface | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-013-AC-1 | A consumer parses Rust source and walks the returned tree with tree-sitter's own `Node` API, with no dependency edge to the fact model, type environment, call resolver or record emitter | Test (TC-135, TC-146, TC-147) |
| FR-013-AC-2 | The returned `ParsedFile` borrows its source text; the value and slices taken from it remain valid for the caller-chosen lifetime after the call returns | Test (TC-141, TC-150) |
| FR-013-AC-3 | A file whose declaration structure is unrecoverable — the root, or a top-level child, is itself an `ERROR`/`MISSING` node — returns `Err` naming the file identifier and the one-based line of the first error position, never an empty or default `Ok` result | Test (TC-136, TC-139, TC-140, TC-148, TC-153) |
| FR-013-AC-4 | Building `quire-code-parse` with `--no-default-features --features rust` compiles and links the Rust grammar only; the Python and TypeScript grammar crates are absent from the resolved dependency graph | Static (`make check-single-grammar`'s dependency-graph inspection; no `#[test]` fn inspects `cargo tree`, so this criterion mints no TC id — see Dependencies) |
| FR-013-AC-5 | Parsing identical language, file identifier and source-text bytes twice yields syntax trees whose rendered structure is identical on both calls, including across process boundaries against a committed golden fixture, not only within one test run | Test (TC-142, TC-149, TC-155) |
| FR-013-AC-6 | No source text a caller can supply as a `&str` causes a panic | Test (TC-143, TC-156) |
| FR-013-AC-7 | `ParsedFile`'s `Send`/`Sync` status is documented and verified by a compiled static assertion rather than assumed from the underlying `tree-sitter` version's documentation; the crate documentation states the fan-out pattern(s) this verified status makes sound for parsing a repository in parallel | Test (TC-144, TC-145) |
| FR-013-AC-8 | `Language` and `ParseError` — including each of `ParseError`'s struct-shaped variants individually — are declared `#[non_exhaustive]`, so a new language variant, diagnostic variant, or field on an existing diagnostic variant is not a breaking change for an existing consumer's `match` | Inspection |
| FR-013-AC-9 | `Err(ParseError::Syntax { .. })` carries the `ParsedFile` tree-sitter produced despite the error; a caller forced to acknowledge the diagnostic through `Result` is never also locked out of the tree it names | Test (TC-154) |
| FR-013-AC-10 | A body-local error — inside one declaration's own expression or statement, its own kind/name/signature still resolvable — returns `Ok` with the full tree walkable, not `Err`; only a top-level item tree-sitter could not resolve as a declaration at all trips FR-013-AC-3 | Test (TC-151, TC-152) |

## Dependencies

- **Upstream**: none within this repository — `quire-code-parse` depends on
  `tree-sitter` and its per-language grammar crates only.
- **Downstream**: `quire-rs` (PLAT-843) is this API's first external
  consumer, reimplementing its Rust symbol scanner over the returned tree
  while retaining the source text this crate borrows rather than owns.
  `filament-ide-rs` and a future daemon are later consumers.

## Verification note on FR-013-AC-4

AC-4's load-bearing half — that the Python and TypeScript grammar crates are
*absent* from the resolved dependency graph under `--features rust` — is
verified by `make check-single-grammar`'s `cargo tree` inspection, run in CI
on every PR. No `#[test]` fn can falsify that half: a compiled Rust test can
only observe what is reachable from *inside* the crate, never what a sibling
crate failed to link. TC-137 and TC-138 verify adjacent but distinct claims
(every compiled-in language loads its grammar; a `rust`-only build has
exactly one `Language` variant) and are cited under FR-013-CON-3 instead. This
mirrors the four `Inspection`-only constraints in `spec/tests.md`'s Coverage
Notes: a real verification method that mints no test symbol, reported
unbacked by construction rather than as an overclaim.
