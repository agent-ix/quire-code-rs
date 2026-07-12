---
id: SPEC-quire-code-rs
title: quire-code-rs master specification index
type: spec
---

# quire-code-rs — Specification Index

Deterministic source-code knowledge-graph extraction library. Requirements are
authored per the quire spec conventions (EARS grammar, AC tables, sequential
IDs within this repo, `ix://agent-ix/quire-code-rs/<ID>` relationship targets).

## Scope

In scope:
- tree-sitter parsing for Rust, TypeScript/TSX, Python.
- Per-file structural fact extraction and per-file type environments.
- In-memory fact corpus + conservative receiver-typed fixpoint call resolution
  (ported approach: gitnexus scope-resolution / type-env, walk + fixpoint,
  missing-edge-beats-wrong-edge).
- Spec↔code linker: harvest `TC-NNN` / `FR-NNN` / `NFR-NNN` / `ix://` mentions
  from comments and attributes.
- Canonical record emission matching quire-rs `extract_filament_core`
  conventions (ix:// refs, stable SHA-256 ids, deduped edges with
  confidence/reason/evidence metadata).

Out of scope:
- Persistence (Postgres writing is the consumer's job — filament-ide-rs /
  analysis workers).
- File watching, incremental orchestration, IPC/UI.
- Network access of any kind.
- LLM-proposed semantic links (a consumer-side concern, validated separately).

## Planned requirement decomposition

To be authored as discrete artifacts under `functional/` / `non-functional/`:

| Area | Planned requirement |
|---|---|
| FR | Language parsing & per-file structural fact model (Rust/TS/TSX/Python) |
| FR | Symbol identity & qualified naming scheme |
| FR | Type environments + receiver-typed fixpoint call resolution with confidence/evidence |
| FR | TC/FR/NFR/ix:// mention harvesting (spec↔code linker) |
| FR | Canonical record emission (quire-rs record parity) |
| NFR | Determinism (byte-identical output for identical input) |
| NFR | No-network / filesystem-only boundary |
| NFR | Extraction time budget |
| NFR | Conservative-resolution precision target |

## ADRs

- ADR-001 (planned): tree-sitter over LSP/rust-analyzer for extraction —
  uniform multi-language story, no daemon, deterministic.
- ADR-002 (planned): in-memory fact corpus for resolution; no persistent index
  (consumers own persistence).

## Test matrix

`spec/tests.md` maps requirements → acceptance criteria → `TC-NNN` tracking
tags carried in test code (this repo dogfoods its own linker conventions).
