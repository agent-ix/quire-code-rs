//! `quire-code-parse` — a shared tree-sitter parse layer.
//!
//! One function, [`parse_file`]; one borrowing type, [`ParsedFile`], holding
//! a syntax tree and the source text it was parsed from; and the diagnostic
//! that comes back attached to it, [`Diagnostic`], when the tree's
//! declaration structure could not be trusted — see "A parse failure is
//! loud" below for what that means and why it is not `Err`. [`ParseError`]
//! is reserved for the separate, rare case where no tree exists at all.
//!
//! ## What this crate is for
//!
//! `quire-code-rs` — this crate's own workspace sibling — parses source and
//! builds a structural fact/edge graph from it: symbol identity, containment,
//! call resolution, canonical record emission. That pipeline is a consumer of
//! parsing, not the whole of it, and other consumers want the tree without
//! wanting the graph:
//!
//! - `quire-rs` extracts test symbols and trace-tag bindings — its own
//!   classification (`SymbolKind`, bench/fuzz detection, marker binding) —
//!   and needs the source text retained after parsing, not dropped.
//! - `filament-ide-rs` edits files live, where a syntax tree exists for
//!   content mid-edit.
//! - A daemon wants to parse a repository once and hand trees to more than
//!   one consumer.
//!
//! None of them want the fact model, the type environment, the fixpoint call
//! resolver or the record emitter compiled into their process to get a tree.
//! This crate is `quire-code-rs`'s own answer to that: a workspace member
//! with no dependency edge to any of those four
//! (FR-013-CON-2) — verified in this crate's `Cargo.toml`, which names only
//! `tree-sitter` and its per-language grammars.
//!
//! ## The boundary this crate keeps
//!
//! This crate parses and yields a syntax tree. It does not classify a single
//! node, decide that a declaration is a test, bind a marker in a comment to
//! the declaration it documents, or construct an identity for anything in the
//! tree. That judgment belongs to the consumer —
//! `ix://agent-ix/quire-code-rs/FR-005-CON-1` states the same boundary for
//! this repository's own extraction pipeline: *"mapping a mention to a
//! `verifies`, `implements` or `references` relationship against indexed
//! artifacts belongs to the consumer."* This crate holds that line one layer
//! lower, before any fact exists to map from.
//!
//! A consumer's own symbol identity — `quire-rs`'s is
//! `(language, path, qualified_name, kind)` — is built entirely on the
//! consumer's side from what it reads out of the tree. This crate exposes no
//! digest, hash or identity helper derived from a node's span or byte
//! offsets, so that reaching for one is never the path of least resistance in
//! place of the consumer's own construction.
//!
//! ## The caller holds the source
//!
//! [`ParsedFile`] **borrows** its source text; this crate never copies it.
//! `quire-code-rs`'s own pipeline follows ADR-002's *load → resolve → emit →
//! drop* discipline — source text is dropped once records are emitted — but
//! that governs its own pipeline, not this API. `quire-rs` needs the text
//! retained for `SymbolExtraction::source_of()` and
//! `Symbol::attached_source()`; a caller of [`parse_file`] chooses how long to
//! keep the source and the [`ParsedFile`] borrowing it, and this crate places
//! no upper bound on that lifetime.
//!
//! ## A parse failure is loud — and the tree is never the price of saying so
//!
//! [`parse_file`] returns `Ok(ParsedFile)` whenever tree-sitter produces a
//! tree at all — essentially always — and [`ParsedFile::diagnostic`] returns
//! `Some(Diagnostic)`, naming a file, one-based line and zero-based column,
//! exactly when the tree's *declaration structure* is unrecoverable. The rule
//! is one sentence, not a per-container enumeration: an error is body-local,
//! and does not count, if and only if it lies within a declaration's own
//! *executable body* node (the `body` a function, method or closure's code
//! actually lives in); everything else inside a declaration — its name, its
//! parameters, its type, its field list, its variant list — is structural,
//! and so is the body of a non-executable container (`mod`, `impl`, `trait`,
//! `class`, `namespace`), at any nesting depth. Checking only the root's own
//! direct children — an earlier shape of this predicate — missed every
//! declaration nested one level down; since every Python method sits at
//! depth 2 inside `class_definition`, that earlier shape structurally could
//! not see a broken Python method at all (PLAT-841 PR #22 review finding
//! FND-001). A later shape fixed that by enumerating container *kinds* whose
//! direct children got checked, but still missed a broken struct field, enum
//! variant or parameter list sitting *inside* a declaration rather than
//! inside a container (PLAT-841 PR #22 review findings FND-007, FND-008,
//! FND-009); the body-node rule above subsumes all of these without a new
//! special case, because it asks the same question everywhere — is this
//! error inside the one node a declaration's own executable code lives in,
//! or is it somewhere else in the declaration. Any of these misses is the
//! exact PLAT-14 shape this crate exists to end, reintroduced one layer down:
//! a hand-rolled Python scanner desynced mid-file and returned zero symbols
//! for the file with nothing in its return type distinguishing that from
//! "this file declares nothing" — 509 passing tests were invisible to
//! coverage for as long as that distinction did not exist in a type.
//!
//! **The tree is never gated behind `Err`.** An earlier shape of this crate
//! put the declaration-structure diagnostic on `Err(ParseError::Syntax)`,
//! carrying the tree inside it. That could not be made `'static`: a
//! `ParsedFile<'src>` borrows its source, so an error carrying one could not
//! either, which meant a consumer could not `?` it into `anyhow::Result`,
//! box it as `Box<dyn Error + 'static>`, or collect it across files into a
//! `Vec` outliving any one source buffer (PLAT-841 PR #22 review finding
//! FND-005) — a real cost to a binary walking a tree of files, exactly
//! `quire-rs`'s own shape. Moving the diagnostic onto `Ok(ParsedFile)`
//! instead removes that tension: [`ParseError`] is now reserved for the rare,
//! essentially defensive case where no tree exists at all, owns its `file`,
//! and is `'static`; the tree is available through the same `Ok` value
//! whether or not `diagnostic()` is `Some`, so acknowledging the diagnostic
//! and keeping the tree were never actually in tension — only `Result`'s
//! shape made them look that way.
//!
//! This diagnostic fires on a narrower condition than "the tree contains an
//! error anywhere": an error nested inside one declaration's own *executable
//! body* (an incomplete expression, mid-edit) leaves that declaration's own
//! kind, name and signature fully readable, and tree-sitter recovers it
//! locally in the shapes this crate's fixture suite exercises for each
//! grammar — checked directly against those fixtures, not assumed from
//! either grammar's documentation, and re-checked at every nesting depth
//! after FND-001, not only at the top level. One shape does not honor this
//! cleanly, and it is common rather than an edge case: in Python, an error
//! that is the *last statement in its suite* — a broken assignment, a
//! dangling `return`, an unclosed call, an unterminated `if`, alike —
//! recovers with the resulting `ERROR` attached as a sibling of the
//! enclosing `function_definition`, outside the function's own `body`
//! field, so it reads as structural under this rule even though it sits,
//! informally, "inside the function". This is a disclosed, known limitation
//! of the mechanical rule, not a silently different answer, and it is not
//! specific to `return`; see FR-013's "Known limitations" section and
//! `TC-174`/`TC-175`. Outside that shape, that case returns `Ok` with
//! `diagnostic()` returning `None`, tree fully walkable; a caller
//! that wants to know a body-local error exists can see it directly via
//! tree-sitter's own [`Node::has_error`](tree_sitter::Node::has_error) on
//! whatever node it is inspecting. Treating every error anywhere as a
//! structural diagnostic would mean `filament-ide-rs`'s live-editing use case
//! — cited above as this crate's reason to exist — gets a diagnostic on
//! essentially every keystroke, since mid-edit content almost always carries
//! a body-local `ERROR` or `MISSING` node. It would also reproduce, one layer
//! down, the exact defect this program exists to end: one `ERROR` node
//! anywhere silently costing a whole file's worth of symbols.
//!
//! ## Consumer obligation
//!
//! Returning `Some(Diagnostic)` instead of `Err` only avoids one silent
//! failure; it opens a second one if a consumer reads `diagnostic()` and
//! then does not surface what it found. `ix://agent-ix/quire-rs`'s own
//! per-file reporting is required to name a file and line for exactly this
//! condition (`ix://agent-ix/quire-rs/FR-051-AC-9`, landed under PLAT-842);
//! this crate is the layer that condition is discovered at, so a consumer
//! that drops `diagnostic()` on the floor reintroduces PLAT-14 one layer up
//! instead of one layer down. See FR-013's own "Consumer obligation" section
//! for the full statement and its relationship to FR-051.
//!
//! ## `tree_sitter` is re-exported, deliberately
//!
//! This crate does not wrap [`tree_sitter::Node`] or `TreeCursor` behind its
//! own traversal API. Reimplementing an equivalent surface would mean owning
//! a traversal API this crate did not design, and every future improvement to
//! tree-sitter's own would have to be hand-ported. Instead, [`parse_file`]
//! returns tree-sitter's own [`Tree`](tree_sitter::Tree), and this crate
//! re-exports the `tree_sitter` crate itself so a consumer never names it as
//! a direct dependency of its own.
//!
//! The consequence, accepted rather than hidden: every consumer of this
//! crate — `quire-code-rs`, `quire-rs`, `filament-ide-rs`, and later a daemon
//! — is pinned to the one `tree_sitter` version this crate's `Cargo.toml`
//! names. That is the intended effect, not a side effect: it single-sources
//! the version across the ecosystem instead of each consumer pinning (and
//! drifting from) its own.
//!
//! ## Thread safety
//!
//! [`ParsedFile`] is [`Send`] **and** [`Sync`] at the `tree_sitter` version
//! this crate pins — checked by a compiled static assertion in
//! `tests/thread_safety.rs`, not assumed. (Older tree-sitter releases made
//! `Tree` `Send`-only; that is no longer this pin's behavior, which is why
//! this is verified rather than stated from memory.) See [`ParsedFile`]'s own
//! docs for the two sound fan-out patterns this enables when parsing a
//! repository in parallel.
//!
//! ## No I/O
//!
//! This crate never reads a path, opens a socket, or touches the filesystem.
//! The caller supplies bytes already in memory and a [`Language`]; walking a
//! repository, deciding which files to read, and choosing a language per file
//! all stay the caller's job.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
// Tests exercise failure paths by panicking on an unexpected result, which is
// what `assert!`/`expect`/`panic!` are for; the deny above targets this
// crate's production code, not its own test suite.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented
    )
)]

mod error;
mod language;
mod parse;

pub use error::{Diagnostic, ParseError};
pub use language::Language;
pub use parse::{parse_file, ParsedFile};

/// tree-sitter's own crate, re-exported deliberately (see the module docs
/// above). A consumer walks a [`ParsedFile`]'s tree with `quire_code_parse`'s
/// `tree_sitter::Node` / `tree_sitter::TreeCursor` — the same types this
/// crate uses internally — rather than a wrapper this crate would have to
/// maintain.
pub use tree_sitter;
