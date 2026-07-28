# quire-code-rs

Deterministic source-code knowledge-graph extraction for the Filament/Quire
ecosystem. Sibling of [quire-rs](https://github.com/agent-ix/quire-rs): quire-rs
extracts canonical graph records from markdown spec artifacts; quire-code-rs
extracts them from source code via tree-sitter (Rust, TypeScript/TSX, Python).

## What it produces

Canonical graph records in the same shape quire-rs emits — ix:// node refs,
stable SHA-256 record ids, edges deduped on `(source_ref, edge_type, target_ref)`
with `{confidence, reason, evidence[], count}` provenance metadata.

- **Nodes**: `code_file`, `code_module`, `code_function`, `code_type`
  (struct/enum/class/interface/trait/alias via a `kind` discriminator).
- **Edges**: `contains`, `imports`, `calls` (receiver-typed, confidence-scored),
  `implements_trait`, `extends`, `references`, plus cross-layer `verifies`
  (test → TC artifact) and `implements` (code → FR/NFR artifact).
- **Identity**: `{repo}/{relative/path}::{Parent}::{symbol}`, stable across
  re-scans, compatible with `ix://agent-ix/{repo}/{name}` resolution.
- **Export surface**: every node carries a `visibility` (`public`/`crate`/
  `private`) and every callable a normalized `signature`, so a consumer can
  tell an exported-surface change from a body-only edit and re-resolve only the
  dependents that a change can actually reach.

## Design principles

- **Deterministic & pure** — filesystem only, no network; identical input yields
  byte-identical output.
- **Conservative resolution** — a per-file type environment feeds a fixpoint
  receiver-typed call binder; a missing edge always beats a wrong edge, and
  every heuristic edge carries confidence + evidence.
- **Spec↔code traceability** — a linker harvests `TC-NNN`/`FR-NNN`/`NFR-NNN`/
  `ix://` mentions from comments and attributes, closing the requirement →
  code → test loop against a spec-derived graph.

## Status

Spec-first repo — requirements are being authored under `spec/` before
implementation lands. Consumed by
[filament-ide-rs](https://github.com/agent-ix/filament-ide-rs) (issue #141)
via a thin `filament-code-extraction` integration crate.

## License

AGPL-3.0-or-later.
