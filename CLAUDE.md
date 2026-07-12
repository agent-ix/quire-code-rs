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
qualified names `{repo}/{relative/path}::{Parent}::{symbol}`, edge dedupe on
`(source_ref, edge_type, target_ref)`.

**Spec-first:** requirements live under `spec/` (flat quire-rs-style tree:
`stakeholder/`, `usecase/`, `functional/`, `non-functional/`, `reviews/`,
`spec.md`, `tests.md`). IDs are sequential within this repo. Tests carry
`TC-NNN` tracking-tag comments mapped in `spec/tests.md` — this repo dogfoods
its own linker conventions.

## Commands

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings
cargo test
cargo deny check
```
