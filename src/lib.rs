//! quire-code-rs — deterministic source-code knowledge-graph extraction.
//!
//! Sibling of quire-rs: where quire-rs extracts canonical graph records from
//! markdown spec artifacts, quire-code-rs extracts them from source code
//! (tree-sitter: Rust, TypeScript/TSX, Python). Same discipline: filesystem
//! only, no network, load → resolve → emit → drop, deterministic output
//! (stable SHA-256 record ids, ix:// refs, edge dedupe on
//! `(source_ref, edge_type, target_ref)`).
//!
//! The normative contract is this repository's own `spec/` tree. This crate
//! implements FR-001 through FR-008 and satisfies
//! `ix://agent-ix/filament-ide-rs/FR-072-CON-1`.

#![forbid(unsafe_code)]

pub mod edges;
pub mod extract;
pub mod facts;
pub mod imports;
pub mod lang;
#[cfg(feature = "measurement")]
pub mod measurement;
pub mod mentions;
pub mod naming;
pub mod parse;
pub mod records;
pub mod resolve;
pub mod typeenv;

pub use edges::{Edge, EdgeType, Evidence, Reason};
pub use extract::{extract, extract_with, ExtractionResult, ExtractionStats};
pub use facts::{CodeFact, Diagnostic, LineSpan, ObjectType, Severity};
pub use lang::Language;
pub use mentions::{Mention, MentionKind};
pub use parse::{parse_file, ParsedFile, SourceFile};
pub use records::{EdgeRecord, NodeRecord};
pub use resolve::{resolve_call, Resolution};
pub use typeenv::{Corpus, RawBinding, TypeEnv, TypeSource};
