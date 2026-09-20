---
id: FR-013
title: "Borrowed parse-tree API for external consumers"
type: FR
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-005"
    type: "implements"
  - target: "ix://agent-ix/quire-rs/FR-051"
    type: "references"
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
- `ParsedFile::diagnostic`, `Some(Diagnostic)` naming the file identifier, a
  one-based line and a zero-based column when the tree's declaration
  structure is unrecoverable, `None` for a clean parse and for a tree whose
  only errors are body-local
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
- An error is body-local, and does not trip the declaration-structure check,
  if and only if it lies within a declaration's own *executable body* node —
  the `body` a function's, method's or closure's own code lives in. Anything
  else inside a declaration — its name, its parameters, its type, its field
  list, its variant list — is structural, and so is the body of a
  non-executable container (`mod`, `impl`, `trait`, `class`, `namespace`), at
  *any* nesting depth. Where the root of the produced syntax tree, or any
  node outside an executable body under this rule, has a direct child that is
  itself an `ERROR` or `MISSING` node, meaning tree-sitter could not resolve
  even the identity of a declaration there, the crate SHALL return
  `Ok(ParsedFile)` with `ParsedFile::diagnostic` returning `Some(Diagnostic)`
  naming the one-based line and zero-based column of the first such position,
  rather than an `Ok` value indistinguishable from a clean parse. The crate
  SHALL apply this check at every nesting level, not only at the root's own
  direct children: a declaration nested inside a `mod`, `impl`, `trait`,
  `class` or `namespace` — including a Python method, which sits at depth 2
  inside `class_definition` — is exactly as reachable by this check as a
  top-level one, and so is a malformed field inside a `struct`'s own field
  list, a malformed variant inside an `enum`'s own variant list, or a
  malformed parameter inside a function's own parameter list, none of which
  are inside any declaration's executable body (PLAT-841 PR #22 review
  findings FND-007, FND-008, FND-009; see "Correction record: FND-007,
  FND-008, FND-009" below).
- The tree tree-sitter produced SHALL be available through the same `Ok`
  value whether or not `diagnostic()` returns `Some` — the diagnostic is
  attached to the tree, never a reason to withhold it, and acknowledging it is
  never gated behind `Result`'s `Err` arm.
- Where an error or missing node exists only inside a declaration's own
  executable body node — its own node kind, name and signature remain
  resolvable, only an expression or statement within the body does not — the
  crate SHALL return `Ok(ParsedFile)` with `diagnostic()` returning `None`,
  rather than treating any error anywhere in the tree as a declaration-
  structure failure. A body-local error remains visible to a caller that
  inspects the relevant node's own error state directly through the
  re-exported `tree-sitter` API. This holds at every nesting depth: a
  body-local error inside a method nested inside a class, or a function
  nested inside a namespace, stays `Ok` with `diagnostic()` returning `None`
  exactly as a top-level one does. One grammar shape does not honor this
  cleanly and is a disclosed known limitation rather than a silently
  different answer: see "Known limitations" below.
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
| FR-013-AC-3 | A file whose declaration structure is unrecoverable — an error outside every declaration's own executable body, at *any* nesting depth — returns `Ok(ParsedFile)` with `diagnostic()` returning `Some`, naming the file and the one-based line of the first error position, never a result indistinguishable from a clean parse. Verified nested one level deep in a `mod`, an `impl`, inside a struct's own field list, inside an enum's own variant list, inside a function's own parameter list, two `mod`s deep, and with a `mod` missing its closing brace (Rust); nested inside a `class`, and for the Python-specific dangling-`return` recovery shape (Python); nested inside a `namespace`, including inside a `namespace`'s own `class` (TypeScript); and across a batch of files collected into one `Vec` — not only at the top level, which an earlier, depth-1-only form of this check missed (PLAT-841 PR #22 review finding FND-001), and not only at container-body boundaries, which an earlier per-container-kind form of this check still missed for a struct field, enum variant or parameter list (PLAT-841 PR #22 review findings FND-007, FND-008, FND-009) | Test (TC-136, TC-148, TC-153, TC-157, TC-159, TC-160, TC-162, TC-165, TC-166, TC-167, TC-168, TC-171, TC-172, TC-173, TC-174) |
| FR-013-AC-4 | Building `quire-code-parse` with `--no-default-features --features rust` compiles and links the Rust grammar only; the Python and TypeScript grammar crates are absent from the resolved dependency graph — verified by `make check-single-grammar`'s `cargo tree` inspection; see "Verification note on FR-013-AC-4" below for why this mints no TC id | Inspection |
| FR-013-AC-5 | Parsing identical language, file identifier and source-text bytes twice yields syntax trees whose rendered structure is identical on both calls, including across process boundaries against a committed golden fixture, not only within one test run | Test (TC-142, TC-149, TC-155) |
| FR-013-AC-6 | No source text a caller can supply as a `&str` causes a panic | Test (TC-143, TC-156) |
| FR-013-AC-7 | `ParsedFile`'s `Send`/`Sync` status is documented and verified by a compiled static assertion rather than assumed from the underlying `tree-sitter` version's documentation; the crate documentation states the fan-out pattern(s) this verified status makes sound for parsing a repository in parallel | Test (TC-144, TC-145) |
| FR-013-AC-8 | `Language` and `ParseError` — including each of `ParseError`'s struct-shaped variants individually — are declared `#[non_exhaustive]`, so a new language variant, diagnostic variant, or field on an existing diagnostic variant is not a breaking change for an existing consumer's `match` | Inspection |
| FR-013-AC-9 | `Ok(ParsedFile)` carries the tree tree-sitter produced whether or not `diagnostic()` returns `Some`; acknowledging a declaration-structure diagnostic is never gated behind `Result`'s `Err` arm, so a caller is never locked out of the tree by reading the diagnostic attached to it | Test (TC-154, TC-161) |
| FR-013-AC-10 | A body-local error — inside one declaration's own executable body, its own kind/name/signature still resolvable — returns `Ok(ParsedFile)` with `diagnostic()` returning `None`, the full tree walkable; only an error outside every declaration's own executable body trips FR-013-AC-3. Verified at the top level and nested one level deep in a `mod` (Rust), a `class` (Python), and a `namespace` (TypeScript), so nesting depth alone is never mistaken for a declaration-structure error (PLAT-841 PR #22 review finding FND-001); the Python dangling-`return` recovery shape is a disclosed exception to this, not covered here — see "Known limitations" | Test (TC-151, TC-152, TC-158, TC-163, TC-164) |
| FR-013-AC-11 | `ParseError` carries no lifetime parameter and is `'static`, verified by a compiled type-level assertion rather than assumed from its shape; a consumer boxes it as `Box<dyn std::error::Error + 'static>` and its accessors and `Display` message name the file and line it carries. An earlier shape of this crate carried a borrowed `ParsedFile` inside `ParseError`, which could not satisfy this (PLAT-841 PR #22 review finding FND-005). `ParseError::NoTree` is the only variant and is not reachable through `parse_file` for any input this crate's own fixtures or property tests have found; its `'static`-ness is real insurance against a currently unexercisable case, not motivated by a live use case today | Test (TC-139, TC-140, TC-169, TC-170) |
| FR-013-AC-12 | `Diagnostic` carries a `file` field alongside `line` and `column`, so a caller that copies a `Diagnostic` out of the loop that produced it — to log it, or to collect it alongside diagnostics from other files — does not have to re-pair it with its source file by hand, or risk pairing it with the wrong one (PLAT-841 PR #22 review round 3, finding FND-013) | Test (TC-161, TC-171) |

## Consumer obligation

Attaching `Diagnostic` to `Ok(ParsedFile)` instead of gating it behind `Err`
avoids one silent failure — a caller is never locked out of the tree by
reading the diagnostic — but it opens a second one if a consumer reads
`diagnostic()` and then does not surface what it found: a `None`-shaped
`Ok` and a `Some`-shaped `Ok` a consumer discards are indistinguishable to
whatever reads that consumer's own output, which is exactly the PLAT-14
failure mode, reintroduced one layer up instead of ended. `quire-rs`
(`ix://agent-ix/quire-rs`) is this crate's first external consumer and is
under its own, separately-landed requirement to close that gap:
`ix://agent-ix/quire-rs/FR-051-AC-9` (landed under PLAT-842) requires
`quire-rs`'s own per-file reporting to name a file and line whenever this
crate's declaration structure could not be resolved. A consumer of this
crate — `quire-rs` today, `filament-ide-rs` or a future daemon later — is
obligated to surface `ParsedFile::diagnostic()` in its own reporting rather
than discard it; this is stated here, not only in FR-051, because a fourth
consumer reading FR-013 six months from now and finding nothing here would
have no reason to know the obligation exists before writing its own silent
zero-diagnostic result.

## Known limitations

Two shapes are disclosed here rather than left to a code comment a reader
would have to find on their own:

- **TypeScript's declaration-structure recovery for a broken declaration
  nested inside a function body does not behave like Rust's or Python's**
  (PLAT-841 PR #22 review finding FND-010). This crate's own predicate is
  mechanical and grammar-agnostic — it asks whether the error lies inside a
  declaration's own executable body node, per grammar's own field structure —
  so a difference here is TypeScript's own recovery shape, not a special
  case this crate adds or omits. Owner: `quire-code-rs` (this crate); no
  further remediation is planned beyond this disclosure, since the crate's
  contract is stated at the level of the body-node rule, not at the level of
  "identical behavior across grammars" that grammar's own recovery does not
  actually provide.
- **`declare module` blocks and `.d.ts` ambient-module files are not
  separately verified against the declaration-structure predicate**
  (PLAT-841 PR #22 review finding FND-011), beyond what TypeScript's regular
  `namespace`/`class`/`function_declaration` fixtures already exercise.
  Owner: `quire-code-rs` (this crate); tracked as a fixture gap, not a known
  incorrect behavior — no case demonstrating a wrong answer for either shape
  has been found.
- **Python's error recovery for a dangling `return <expr> +` attaches the
  resulting `ERROR` node as a sibling of `function_definition`, outside the
  function's own `body` field**, rather than nesting it inside `body` the
  way an assignment (`x = <expr> +`) does. Under the body-node rule this
  reads as structural (`diagnostic()` returns `Some`) even though the error
  sits, informally, "inside the function" — a mechanical consequence of
  asking a structural question about where a node sits in the tree, not
  about what a reader would call "inside" in prose. `TC-174` documents this
  fixture and its `Some` outcome directly, rather than either fixture
  silently taking one answer or the discrepancy going unrecorded; `TC-163`
  uses an assignment-shaped fixture instead so it tests the genuinely
  body-local case its row claims.

## Correction record: FND-012, FND-013

PR #22's third review round also found two related issues in how AC-11 and
`Diagnostic` were tested and shaped.

**FND-012**: the test originally cited for AC-11's cross-file-collection
claim (`errors_from_several_files_collect_into_one_vec_outliving_their_sources`)
never touched `ParseError` at all — it collected `Diagnostic` values read off
`Ok(ParsedFile)`, which were never borrowing anything an earlier,
non-`'static` `ParseError` shape would have prevented either. It would have
passed identically against that earlier shape, so it proved nothing about
`'static`-ness. Retargeted to FR-013-AC-3 (renamed
`diagnostics_from_several_files_project_into_one_vec_outliving_their_sources`,
still `TC-171`), which is the claim it actually exercises: a consumer
collecting `Diagnostic`s across a batch of files.

**FND-013**: the hard requirement this FR implements is a diagnostic naming
*file and line*; `Diagnostic` carried only `line`/`column`, so a caller that
copied a `Diagnostic` out of the loop that produced it lost the file
identity and had to re-pair it by hand (`TC-171`'s own retargeted test did
exactly this before the fix). `Diagnostic` now carries `file: &'src str`
(FR-013-AC-12) — free to add, since `Diagnostic` is only ever handed out
from an already-borrowed `ParsedFile<'src>` carrying the same lifetime. This
makes `Diagnostic` *not* `'static`, unlike `ParseError`, which is — a
deliberate difference: `ParseError` is `'static` because it never borrows
anything, `Diagnostic` borrows its file identity from the same source its
`ParsedFile` already borrows, and asking it to own that borrow (a `String`)
was not necessary to satisfy either FND-012's retargeted test or any other
requirement, so it was not done.

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
depth and asks the structural question — is the malformed node inside a
declaration's own executable body, or somewhere else in the declaration — at
every level, not only the root's own direct children; see "Correction record:
FND-007, FND-008, FND-009" below for how that question was later sharpened
from a per-container-kind enumeration into the one body-node rule this crate
now implements.

## Correction record: FND-007, FND-008, FND-009, the body-node rule

PR #22's third review round (2026-09-20) found that the fix above, while
correct for every case it was built and tested against, still enumerated
container *kinds* whose direct children got checked (`mod`, `impl`, `trait`,
`class`, `namespace` bodies) rather than stating the underlying rule
directly. That enumeration missed three shapes, each a declaration's own
structural part rather than a body it owns: a struct's own field list
(`pub struct S { a: , }` — FND-007), a function's own parameter list
(`pub fn f( -> u32 {}` — FND-008), and an enum's own variant list, which in
fact needed no special case at all once the underlying rule was corrected
(FND-009). TC-160 and TC-168 (TypeScript) had passed only because
TypeScript's own grammar happened to land its `ERROR`/`MISSING` node inside
`class_body`, a container kind the enumeration already checked — a
per-grammar accident, not a property the rule guaranteed.

The ruling: reject a fix that special-cases each finding individually, and
implement the one rule FR-013-AC-10 already states in prose — *"its own
kind, name and signature remain resolvable"* — directly, rather than as an
enumeration that approximates it. That rule is: **an error is body-local if
and only if it lies within the declaration's own body node. Anything else
inside the declaration — its name, its parameters, its type, its field list,
its variant list — is structural.** Implemented as
[`has_declaration_structure_error`](../../crates/quire-code-parse/src/parse.rs)
and [`scan_for_declaration_structure_error`](../../crates/quire-code-parse/src/parse.rs),
which thread an `inside_executable_body: bool` through the whole subtree
beneath a declaration's own body field, resetting only at the next
declaration boundary — not
[`is_declaration_container`](../../crates/quire-code-parse/src/parse.rs),
the per-container-kind enumeration this rule replaced. Checked directly
against each grammar's own fixtures (`TC-172`, `TC-173`, and `TC-162`'s
corrected fixture), not assumed from either grammar's documentation; the one
place this stricter rule produces a result a reader might not expect on
first reading — Python's dangling-`return` recovery shape — is disclosed
under "Known limitations" and exercised directly by `TC-174`, rather than
left as a silent difference between what the rule says and what one
fixture's outcome happens to be.

A first implementation of the body-node rule regressed three passing cases
(body-local errors nested inside a `mod`, and inside a Python method nested
inside a `class`) before landing: exempting only the literal body node from
its own direct-child check, without propagating the "inside a body" state to
the whole subtree beneath it, meant a node two levels inside a body (for
example, a `MISSING` node inside a nested `binary_expression`) was
re-evaluated as if it were a fresh top-level candidate and flagged as
structural. The fix that shipped propagates the state through recursion
rather than checking it fresh at each node — recorded here because the
regression was caught by the full suite before landing, not because a wrong
answer shipped.
