---
id: SR-11085
title: "PLAT-870 quire-code-rs PR55 code-review (Rust lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-code-rs PR55; crates/quire-code-parse/src/parse.rs, crates/quire-code-parse/src/lib.rs, src/parse.rs, spec/tests.md"
review_set: subset
---

## Summary

Ticket: PLAT-870. Reviewed `git diff origin/main...HEAD` at head 90c7028 (two commits). Rust lane (rust-review, rust-style loaded) folded in. Adds `ParsedFile::syntax_error()` (leftmost ERROR/MISSING anywhere), routes quire-code-rs's own syntax-error detection through it (deleting its duplicate `first_error_position` with its bare `as u32` casts), and changes `diagnostic()`'s position from "leftmost error of any kind" to the node returned by `first_declaration_structure_error`. Focused runs: `cargo test -p quire-code-parse` all pass (31 unit + integration); `cargo test --lib parse::` 32 pass; clippy `-D warnings` clean on both packages (default features).

Detection (Some/None) of `diagnostic()` is unchanged: the bool-to-Option refactor keeps identical control flow. Only the reported position moves. A probe built against the PR head shows the new position is not the first structural error in source order when an earlier structural error sits deeper than a later one: `mod m {\n    pub fn ( ) { }\n}\nfn ( ) {}\n` gives `diagnostic()` = (4,0) while the earliest structural error is (2,4) (main reported (2,4)). Cause: `scan_for_declaration_structure_error` checks all direct children of a node before recursing into any child, so a shallow later error wins over a deep earlier one; the doc comment's "first such structural error in a pre-order walk" is false.

Consumers: quire-rs `symbols/{rust,python,typescript}.rs` read `diagnostic()` only to build an `Err("line N: unresolvable declaration structure (column C)")` string; Some/None gating is unaffected, the only pinned string (`tc1884`, truncated file) still yields (1,0) on the PR head. FR-007-AC-2 / TC-040 / TC-075 in the root crate are unaffected: `syntax_error()` uses the same leftmost-error walk the deleted local function used. Saturating `line_number`/`column_number` pre-exist on main; the PR extends them to the root crate, an improvement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | diagnostic() names a later shallow structural error instead of an earlier deeper one (repro: (4,0) vs earliest structural (2,4)); regresses AC-3 "first error position" against main, and the doc comment's "first ... in a pre-order walk" claim is false | crates/quire-code-parse/src/parse.rs:345-370 |
| FND-002 | low | TC-184 Python and TypeScript cases assert file and line but not column, though FR-013-AC-13 requires a zero-based column for all three languages | crates/quire-code-parse/src/parse.rs:860-884 |

## Examined scope

```yaml
scope:
- {id: FR-013-AC-13, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-14, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-15, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-3, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-9, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-10, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-12, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-007-AC-2, path: spec/functional/FR-007-parse-error-isolation.md, role: examined}
- {id: FR-007-AC-7, path: spec/functional/FR-007-parse-error-isolation.md, role: examined}
```

## Verdict

Changes requested (one medium). `syntax_error()` itself is correct against AC-13/14/15, idiomatic, panic-free, documented; the root-crate refactor is a clean dedup. Fix FND-001 in-PR: in `scan_for_declaration_structure_error`, walk children once in source order, returning a child that is ERROR/MISSING when the direct-child check applies, else recursing into it before moving to the next sibling; correct the doc comment; add a TC-187-adjacent test with two structural errors (deep earlier, shallow later) asserting the earlier line. FND-002: add column asserts.
