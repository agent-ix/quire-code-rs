//! Compiles the thread-safety claim in `ParsedFile`'s docs instead of
//! trusting it.
//!
//! Implements FR-013-AC-7. This assertion is here *because* the obvious
//! assumption about `tree_sitter::Tree` turned out to be wrong for the
//! version this crate pins: folklore (and this crate's own original design
//! note) held that `Tree` is `Send` but not `Sync`, matching older
//! tree-sitter releases. `tree-sitter = "0.26"` (the version pinned in
//! `Cargo.toml`, single-sourced per the crate docs) instead declares
//! `unsafe impl Sync for Tree {}` alongside `Send` — verified by reading
//! `tree-sitter-0.26.11/binding_rust/lib.rs` directly, not by trusting either
//! claim. `ParsedFile` adds no field of its own that narrows that: `&str`,
//! `String` and `Language` are all `Send + Sync`. If a future tree-sitter
//! upgrade changed `Tree`'s bound again, one of these two tests would stop
//! compiling — a compile error here rather than a data race a consumer's
//! parallel fan-out discovers at runtime.
//!
//! Each assertion is wrapped in a `#[test]` fn, rather than invoked at module
//! scope, so it is a real test symbol this repository's own trace-tag scanner
//! can bind a `TC-NNN` id to (FR-005) — a bare macro invocation with no
//! enclosing declaration mints nothing for it to attribute the comment to.

use quire_code_parse::ParsedFile;

// TC-144, FR-013-AC-7: `ParsedFile` may be sent to another thread — a worker
// thread may parse and send its `ParsedFile` back to a collecting thread.
#[test]
fn parsed_file_is_send() {
    static_assertions::assert_impl_all!(ParsedFile<'static>: Send);
}

// TC-145, FR-013-AC-7: `ParsedFile` may also be shared behind `&ParsedFile`
// reached from two threads at once — concurrent read-only traversal of one
// already-parsed tree from a pool of reader threads is sound, because this
// crate exposes no `&mut` method on `ParsedFile` and tree-sitter's own `Tree`
// is `Sync` at the pinned version.
#[test]
fn parsed_file_is_sync() {
    static_assertions::assert_impl_all!(ParsedFile<'static>: Sync);
}
