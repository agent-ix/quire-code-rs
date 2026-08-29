---
id: SR-004
title: "code review of the traceability, producer and corpus-findings stack"
type: SpecReview
analysis: code-review
scope: "src/, tests/, spec/, scripts/, and the agent-ix/quire-corpus runner (bounds.py, digest.py, score.py, tests/)"
review_set: subset
---

## Summary

Code review of the three stacked changes on this crate — the Test Matrix
repair, the producer binary, and the six defects the shared corpus found —
together with the `agent-ix/quire-corpus` runner they are measured by. Two high
findings, three medium, two low. All seven are closed in the branch under
review; the verdict records what the review found, not what survived it.

## Verdict

**FAIL** — two high findings. Both are fixed here (FND-001 in `src/parse.rs`,
FND-002 in `src/bin/extract_tree.rs`), each with a regression test that fails
against the code as reviewed.

## Findings

| ID      | Severity | Summary                                                        | Refs                          |
| ------- | -------- | -------------------------------------------------------------- | ----------------------------- |
| FND-001 | high     | An annotation run escaped its block and claimed the next declaration | src/parse.rs:301        |
| FND-002 | high     | The producer aborted on one unreadable file, recursed unbounded, and followed symlinks | src/bin/extract_tree.rs:96 |
| FND-003 | medium   | Two ancestor guards used a narrower declaration set than the walker | src/parse.rs:808           |
| FND-004 | medium   | Five scorer capabilities shipped with no acceptance criteria and no tests | score.py:150           |
| FND-005 | medium   | The producer contract another repository pins had no owning requirement or test | src/bin/extract_tree.rs:1 |
| FND-006 | medium   | The corpus reached 2 of 6 resolution tiers and 8 of 12 TypeScript declaration forms | corpus.yaml:1 |
| FND-007 | low      | FR-008's behavior clause contradicted its own restated AC-8      | spec/functional/FR-008-type-environments-and-call-resolution.md:47 |
| FND-008 | low      | Three corpus expectations were wrong, and only running them said so | fixtures/relations/import-scoped-call/rust/expected.yaml:1 |

## Detail

### FND-001 — an annotation run escaped its block

`pending_comments` became a `Walker` field so that a leading comment could
survive the `export` wrapper between itself and its declaration. It also
survived the end of a block. Given:

```rust
pub fn alpha() {
    // TC-001 describes something inside alpha.
}

pub fn beta() {}
```

the comment was attributed to `beta`. A tracking tag written about one function
became a verification claim on a different one — the precise failure class this
whole programme exists to detect, introduced while fixing its sibling.

Fixed by clearing the run when the frame ends, since a comment left over
introduced nothing, and by skipping anonymous grammar tokens rather than
treating them as the end of a run. TC-099 fails against the code as reviewed.

### FND-002 — the producer could not survive a real repository

`collect` recursed (a deep tree overflows the stack), followed symbolic links (a
link to an ancestor never terminates), and returned `Err` on the first non-UTF-8
file, aborting the extraction of everything else. A measurement tool pointed at
somebody's repository fails on all three.

Fixed: iterative walk, `symlink_metadata` so links are not followed, and
unreadable files collected — records are written, every unreadable path is
named, and the run exits non-zero. A partial tree must not be scoreable as a
whole one. TC-105 covers the link; the unreadable path is asserted through the
corpus's own producer contract.

### FND-003 — two ancestor guards used a narrower declaration set

`decl_for` answered "declarations whose node kind alone identifies them", which
silently stopped including `variable_declarator` the moment it gained a value
guard. `in_trait_impl` and the visibility walk therefore stopped at a different
set of declarations than the walker did. One node-aware lookup now, and no
kind-only sibling to reach for.

### FND-004 — scorer capabilities with no criteria and no tests

Mention-kind grading, bounded diagnostics, the determinism re-run, and payload
hygiene were all added to `score.py` with nothing in `spec/` and no test. The
matrix reported full coverage throughout, because a matrix cannot see what
nobody declared — the same shape as the defect this stack opened with. Closed by
quire-corpus FR-005 and TC-032..TC-038.

### FND-005 — the producer contract had no owning requirement

`agent-ix/quire-corpus` pins this crate's producer as
`producer_contract.version: 1`, and every governed observation recorded against
this extractor was measured through it. It was an `examples/` file with no FR
and no test, so a change to its flags or its stdout shape would invalidate a
body of evidence rather than break a build. It is now a `[[bin]]` — an example
cannot be run by an integration test, and an unexecutable contract is an
unverified one — governed by FR-010 and covered by TC-101..TC-105.

### FND-006 — the corpus was not exhaustive

`import-scoped` and `name-match` — the two heuristic resolution tiers, where a
wrong edge is most likely — had no case at all, and four of TypeScript's twelve
declaration forms had no fixture. 22 cases to 72, 27 graded assertions to 183.
Discrimination is now recorded rather than assumed: at one revision the pre-fix
producer scores 0.982/0.880 with 11 failing cases and the current one 1.0/1.0.

### FND-007 — a spec clause contradicted its own criterion

FR-008-AC-8 was restated so that a trait-object call resolves to the trait's own
method. The Behavior section still said no edge is emitted. Split: a trait
object names the interface, and a generic parameter names nothing and still
emits nothing.

### FND-008 — three corpus expectations were wrong

Found by running the corpus, not by reading it. The largest: two tier cases
wrote `fn drive(handle: Unknown)` intending "the receiver's type is not known".
It means the opposite — the type is written, it resolves to a type declaring no
such method, and FR-008 says a known receiver lacking the method does not fall
back. The producer emitting nothing was correct. Each correction names the
clause that decided it, in the file.

## Gates

Run, not assumed:

```
cargo fmt --check                                    pass
cargo clippy --all-targets --all-features -D warnings pass
cargo test                                            134 passed, 1 ignored (perf lane)
cargo deny check                                      advisories, bans, licenses, sources ok
make coverage                                         217/225 backed, 0 status lies
quire-corpus: bounds / tests / score                  72 cells, 36 tests, 183 assertions
```
