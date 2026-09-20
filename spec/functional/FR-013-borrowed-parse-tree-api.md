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
the tree's declaration structure could not be resolved — at any nesting
depth, not only at the top level — the crate SHALL attach a named diagnostic
naming a line and column to the returned value, and SHALL NOT withhold the
tree tree-sitter produced to do so: a diagnostic is never satisfied by
withholding the tree it is about, and acknowledging it is never gated behind
`Result`'s `Err` arm. `Err` is reserved for the separate, essentially
defensive case where no tree exists at all, and the error type returned for
that case SHALL own its file identifier rather than borrow it, so a consumer
can retain, box, or collect it independently of the source buffer any single
parse call borrowed.

## Inputs

- A `Language` selector naming one grammar compiled into the crate
- A file identifier, used only to attribute a diagnostic
- Source text, borrowed for the lifetime the caller chooses

## Outputs

- A `ParsedFile` value borrowing the parsed `tree_sitter::Tree` and the source
  text, giving the consumer the same `Node`/`TreeCursor` traversal API this
  crate uses internally, returned whenever tree-sitter produces a tree at all
- `ParsedFile::diagnostic`, `Some(Diagnostic)` naming a one-based line and a
  zero-based column when the tree's declaration structure is unrecoverable,
  `None` for a clean parse and for a tree whose only errors are body-local
- `Err(ParseError::NoTree)`, naming the file identifier and a line, only in
  the defensive case where tree-sitter produces no tree at all; this variant
  owns its file identifier and carries no lifetime parameter

## Behavior

- The crate SHALL accept a caller-supplied `Language`, file identifier and
  source text, and SHALL NOT read from the filesystem or the network to
  satisfy the call.
- The crate SHALL return a `ParsedFile` value that borrows the source text
  passed to it for the lifetime the caller chooses, rather than copying it or
  retaining an owned copy inside the crate.
- Where the root of the produced syntax tree, or a declaration-list-shaped
  body nested at *any* depth — a `mod`/`impl`/`trait` body in Rust, a `class`
  body in Python, a `class`/`namespace` body in TypeScript — has a direct
  child that is itself an `ERROR` or `MISSING` node, meaning tree-sitter could
  not resolve even the identity of a declaration there, the crate SHALL
  return `Ok(ParsedFile)` with `ParsedFile::diagnostic` returning
  `Some(Diagnostic)` naming the one-based line and zero-based column of the
  first such position, rather than an `Ok` value indistinguishable from a
  clean parse. The crate SHALL apply this check again at every nesting level,
  not only at the root's own direct children: a declaration nested inside a
  `mod`, `impl`, `trait`, `class` or `namespace` — including a Python method,
  which sits at depth 2 inside `class_definition` — is exactly as reachable by
  this check as a top-level one.
- The tree tree-sitter produced SHALL be available through the same `Ok`
  value whether or not `diagnostic()` returns `Some` — the diagnostic is
  attached to the tree, never a reason to withhold it, and acknowledging it is
  never gated behind `Result`'s `Err` arm.
- Where an error or missing node exists only inside a declaration's own body —
  its own node kind, name and signature remain resolvable, only an expression
  or statement within it does not — the crate SHALL return `Ok(ParsedFile)`
  with `diagnostic()` returning `None`, rather than treating any error
  anywhere in the tree as a declaration-structure failure. A body-local error
  remains visible to a caller that inspects the relevant node's own error
  state directly through the re-exported `tree-sitter` API. This holds at
  every nesting depth: a body-local error inside a method nested inside a
  class, or a function nested inside a namespace, stays `Ok` with
  `diagnostic()` returning `None` exactly as a top-level one does.
- The crate SHALL return `Err(ParseError::NoTree)` — naming the file
  identifier and a line, and owning its file identifier rather than borrowing
  it — only in the defensive case where tree-sitter produces no tree at all
  for the given input. `ParseError` SHALL carry no lifetime parameter, so a
  consumer can convert it into `anyhow::Result`, box it as
  `Box<dyn std::error::Error + 'static>`, or collect it across files into a
  `Vec` that outlives any one file's own source buffer.
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
| FR-013-AC-3 | A file whose declaration structure is unrecoverable — the root, or a declaration-list-shaped body nested at *any* depth, has a direct child that is itself an `ERROR`/`MISSING` node — returns `Ok(ParsedFile)` with `diagnostic()` returning `Some`, naming the one-based line of the first error position, never a result indistinguishable from a clean parse. Verified nested one level deep in a `mod`, an `impl`, inside a struct's own field list, two `mod`s deep, and with a `mod` missing its closing brace (Rust); nested inside a `class` (Python); nested inside a `namespace`, including inside a `namespace`'s own `class` (TypeScript) — not only at the top level, which an earlier, depth-1-only form of this check missed (PLAT-841 PR #22 review finding FND-001) | Test (TC-136, TC-148, TC-153, TC-157, TC-159, TC-160, TC-162, TC-165, TC-166, TC-167, TC-168) |
| FR-013-AC-4 | Building `quire-code-parse` with `--no-default-features --features rust` compiles and links the Rust grammar only; the Python and TypeScript grammar crates are absent from the resolved dependency graph — verified by `make check-single-grammar`'s `cargo tree` inspection; see "Verification note on FR-013-AC-4" below for why this mints no TC id | Inspection |
| FR-013-AC-5 | Parsing identical language, file identifier and source-text bytes twice yields syntax trees whose rendered structure is identical on both calls, including across process boundaries against a committed golden fixture, not only within one test run | Test (TC-142, TC-149, TC-155) |
| FR-013-AC-6 | No source text a caller can supply as a `&str` causes a panic | Test (TC-143, TC-156) |
| FR-013-AC-7 | `ParsedFile`'s `Send`/`Sync` status is documented and verified by a compiled static assertion rather than assumed from the underlying `tree-sitter` version's documentation; the crate documentation states the fan-out pattern(s) this verified status makes sound for parsing a repository in parallel | Test (TC-144, TC-145) |
| FR-013-AC-8 | `Language` and `ParseError` — including each of `ParseError`'s struct-shaped variants individually — are declared `#[non_exhaustive]`, so a new language variant, diagnostic variant, or field on an existing diagnostic variant is not a breaking change for an existing consumer's `match` | Inspection |
| FR-013-AC-9 | `Ok(ParsedFile)` carries the tree tree-sitter produced whether or not `diagnostic()` returns `Some`; acknowledging a declaration-structure diagnostic is never gated behind `Result`'s `Err` arm, so a caller is never locked out of the tree by reading the diagnostic attached to it | Test (TC-154, TC-161) |
| FR-013-AC-10 | A body-local error — inside one declaration's own expression or statement, its own kind/name/signature still resolvable — returns `Ok(ParsedFile)` with `diagnostic()` returning `None`, the full tree walkable; only a declaration tree-sitter could not resolve at all trips FR-013-AC-3. Verified at the top level and nested one level deep in a `mod` (Rust), a `class` (Python), and a `namespace` (TypeScript), so nesting depth alone is never mistaken for a declaration-structure error (PLAT-841 PR #22 review finding FND-001) | Test (TC-151, TC-152, TC-158, TC-163, TC-164) |
| FR-013-AC-11 | `ParseError` carries no lifetime parameter and is `'static`, verified by a compiled type-level assertion rather than assumed from its shape; a consumer boxes it as `Box<dyn std::error::Error + 'static>` and its accessors and `Display` message name the file and line it carries. An earlier shape of this crate carried a borrowed `ParsedFile` inside `ParseError`, which could not satisfy this (PLAT-841 PR #22 review finding FND-005) | Test (TC-139, TC-140, TC-169, TC-170) |

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

## Correction record: FND-001, the declaration-structure predicate

PR #22's second review round (2026-09-20) found that the shipped predicate —
checking only the root node's *direct* children for an `ERROR`/`MISSING` node
— implemented a positional rule ("depth 1"), while this FR's own Behavior
section already stated a semantic one ("the declaration's own kind, name and
signature remain resolvable"). The two disagreed for every declaration nested
one level down. Because every Python method sits at depth 2 inside
`class_definition`, the depth-1 form could not see a broken Python method at
all — the exact PLAT-14 shape ("a genuinely broken file reports nothing")
this crate exists to end, reintroduced one layer down, and quieter than the
original defect because the tree still came back.

The ruling was to fix the code to meet the already-correct spec rather than
narrow the spec to match the code — rewriting AC-3/AC-10 down to a documented
positional blind spot would have shipped that blind spot into the layer three
repos depend on, and is the same failure PLAT-842 (FR-051's "declaration
structure" definition) was sequenced first to prevent. The fix walks to full
depth and asks the structural question — is the malformed node a direct child
of a declaration-list-shaped body, or nested inside an executable
body that merely shares a node kind with one — at every level, not only the
root's own direct children; see
[`is_declaration_container`](../../crates/quire-code-parse/src/parse.rs) for
the per-grammar detail, verified empirically (via `to_sexp()` on nested
fixtures) rather than assumed, including where tree-sitter's own recovery
attaches the error somewhere other than the first guess (Python attaches an
unresolvable method to `class_definition` itself, not to its `block`).
