//! quire-code-rs — deterministic source-code knowledge-graph extraction.
//!
//! Sibling of quire-rs: where quire-rs extracts canonical graph records from
//! markdown spec artifacts, quire-code-rs extracts them from source code
//! (tree-sitter: Rust, TypeScript/TSX, Python). Same discipline: filesystem
//! only, no network, load → resolve → emit → drop, deterministic output
//! (stable SHA-256 record ids, ix:// refs, edge dedupe on
//! `(source_ref, edge_type, target_ref)`).
//!
//! Planned module layout (authored via the spec cycle in `spec/`):
//! per-file structural facts + type environments, an in-memory fact corpus
//! driving a conservative receiver-typed fixpoint call resolver, and a
//! spec↔code linker harvesting TC-/FR-/NFR-/ix:// mentions from comments.

#![forbid(unsafe_code)]
