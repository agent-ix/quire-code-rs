# quire-code-rs

Deterministic source-code knowledge-graph extraction (tree-sitter: Rust,
TypeScript/TSX, Python). Sibling of quire-rs and consumed by filament-ide-rs
(issue agent-ix/filament-ide-rs#141) through the `filament_extraction::Extractor`
seam. License: AGPL-3.0-or-later.

**Hard rules (mirror quire-rs):**
- Filesystem only — no network dependencies, ever.
- Deterministic output: identical input → byte-identical records (stable
  SHA-256 ids, ordered collections). No wall-clock or randomness in extraction.
- `#![forbid(unsafe_code)]` on this crate's own code; tree-sitter's C FFI stays
  in dependencies.
- Conservative resolution: a missing edge beats a wrong edge; heuristic edges
  carry `{confidence, reason, evidence[]}` metadata.

**Canonical record contract:** match quire-rs `extract_filament_core`
conventions — ix:// refs (`ix://agent-ix/{repo}/{name}`, ≥3 segments,
last-segment resolution), node identity `(object_type, container, name)` with
org-qualified names `{org}/{repo}/{relative/path}::{Parent}::{symbol}` (FR-072
requires the `{org}/` prefix so same-named repos in different orgs cannot
collide), edge dedupe on `(source_ref, edge_type, target_ref)`. Node data also
carries `visibility` (`public`/`crate`/`private`) and, for callables, a normalized
`signature` (FR-009) — neither is hashed into node identity, so a visibility or
signature change is a modification, never a delete plus an add.

**Provenance vocabulary** (consumed by filament-ide-rs FR-072): every edge
carries `confidence` ∈ [0.0, 1.0], `reason` ∈ {`syntactic`, `path-resolved`,
`name-match`, `import-scoped`, `receiver-typed`, `explicit-mention`},
`evidence: [{file, line}]` capped at 20 entries, and `count` — the total
call-site count folded onto the single deduplicated edge.

**The corpus grades this crate:** `src/bin/extract_tree.rs` is the producer
`agent-ix/quire-corpus` invokes. Its flags and its stdout are a pinned contract
(`producer_contract.version: 1`), so changing either invalidates every recorded
observation — bump the version there rather than changing the shape here.

**Traceability is gated:** `make coverage` reconciles `spec/tests.md` against
the suite with `quire coverage`. Every row is backed by a tagged test or its
declared verification method says why no symbol can exist. A tag binds every id
in its comma list — `// TC-013, FR-002-AC-6, FR-002-CON-1: …` backs three
targets, while `// TC-013 — FR-002-AC-6: …` backs only the first, because the
em dash terminates the id run. `#[ignore]` must precede `#[test]`, or the
scanner does not see the symbol at all.

**Spec-first:** requirements live under `spec/` (flat quire-rs-style tree:
`stakeholder/`, `usecase/`, `functional/`, `non-functional/`, `reviews/`,
`spec.md`, `tests.md`). IDs are sequential within this repo. Tests carry
`TC-NNN` tracking-tag comments mapped in `spec/tests.md` — this repo dogfoods
its own linker conventions.

## Commands

```bash
make fmt            # format with rustfmt
make fmt-check      # verify formatting (CI gate)
make lint           # clippy with -D warnings
make test           # cargo test
make deny           # cargo deny check licenses
make audit-unsafe   # every `unsafe {` needs a // SAFETY: comment
make ci             # fmt-check + lint + test + deny + audit-unsafe
```

Spec validation after any `spec/` edit:

```bash
quire validate --scope . "spec/**/*.md"
```

## Safety scaffolding

House kit from `agent-ix/rust-lib-cookiecutter` (originally backported from
`agent-ix/ecaz`): `clippy.toml` (MSRV pin + complexity caps), `deny.toml`
(permissive-only allow-list; AGPL permitted for this crate alone),
`rustfmt.toml` (100-char, `StdExternalCrate` grouping), `rust-toolchain.toml`
(stable + rustfmt + clippy), `scripts/check_unsafe_comments.sh`, and
`.github/workflows/ci.yml` (fmt / clippy / test / license / unsafe audit).
Unlike filament-ide-rs, **CI runs here** — PRs gate themselves.
