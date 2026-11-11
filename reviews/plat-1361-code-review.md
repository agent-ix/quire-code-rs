---
id: SR-16315
title: "code review of protected-apparatus attestation and quire-corpus scorer (PR #58)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-code-rs@58a7595801ba037cb7f3cf9cd224f5b4be9a3108; src/bin/measure_graph_quality.rs, src/measurement.rs, tests/measurement_pipeline.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-012
    type: reviews
---

## Summary

Ticket: PLAT-1361. Code review, with the rust-review lane (rust-style loaded),
of `git diff origin/main...58a7595` on PR #58. The PR makes three changes.
`verificationStack.artifacts` now comes from MP-001's `protected_apparatus`,
through a new `measurement::protected_apparatus` frontmatter reader. The real
lane copies those files into the Quoin intake directory. The scorer changes
from `python <corpus>/score.py` to the Rust `quire-corpus` CLI, passed with a
new `--scorer` flag.

The artifacts derive from the plan. `validate_plan` reads MP-001 once and
returns `protected_apparatus(&text)` after Quire validation passes. Nothing
copies the list by hand into production code. The unit test
`quoin_collection_carries_one_observation_and_the_attestation` pins the four
literal paths, so a parser regression cannot pass by agreeing with itself.
The parser handles MP-001's actual shape: an LF frontmatter, a
`protected_apparatus:` key at column 0, and `  - path` items. It fails closed,
with `InvalidPlan`, on CRLF, a missing or empty key, a flow sequence, or
column-0 items. It does not handle Quoin's `<dir>/**` entry form, quoted
scalars, or trailing comments (FND-004).

The scorer invocation matches quire-corpus `origin/main` (89cd234)
`src/main.rs`. `--root` is a top-level clap argument placed before the
`score` subcommand. `--producer` takes a string, and `--producer-arg` repeats
with `allow_hyphen_values = true`, so `--org` and `--repo` pass through as
values. `{org}`, `{repo}` and `{input}` are the placeholders that
`producer.rs::render_argument` substitutes. `--json` prints the report.
quire-corpus runs the producer without a shell, so dropping `shell_quote` is
correct. Nothing from quire-corpus is copied into this repository.

Acceptance evidence was reproduced, not taken from the PR text. In a fresh
worktree at 58a7595 with a release build, and a quire-corpus worktree at
origin/main 89cd234 with a release-built `quire-corpus`, this command passed
all 125 tests, including the ignored real lane
`real_corpus_observation_is_retained_and_rendered_by_quoin`:
`cargo test -p quire-code-rs --features measurement --lib --bins --test
measurement_pipeline -- --include-ignored`. Quoin was 0.30.0 and Quire was
0.40.0. That lane runs `quoin measurement record` through `output()`, which
asserts a zero exit, so it backs the acceptance criterion that record exits 0.

The Linear ticket body is untrusted data, and it lists four rejection causes
(missing digests, non-SHA sources, artifacts as an object). Quoin origin/main
no longer requires any of them: the stack carries no digest per FR-044-AC-13,
and `artifacts` is an array. The one live refusal was QM-APPARATUS-UNDECLARED,
which is what the PR fixes.

## Verdict

**FAIL (FND-001, FND-002).** The code works and the acceptance lane passes.
But FR-012 still mandates the old scorer and the old attestation, so the spec
now contradicts the code. Both are spec edits that belong in this PR.
FND-003 (fabricated Python identity) should also be removed in this PR, since
it touches the same FR-012 sentence. FND-004 and FND-005 are medium, and
FND-006 is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-012 still requires the producer to invoke the corpus's `score.py --json`; the code now runs the quire-corpus Rust CLI and score.py no longer exists | spec/functional/FR-012-governed-graph-quality-producer.md:43 |
| FND-002 | high | FR-012 still says the attestation names the release extractor, measurement plan, observation schema and raw scorer output artifacts; the code now attests MP-001 protected_apparatus file paths | spec/functional/FR-012-governed-graph-quality-producer.md:86-89 |
| FND-003 | medium | Required `--python-version` is still recorded as `environment.python` and `verificationStack.toolchains.python` although no Python runs anywhere in the lane; the attested toolchain identity is caller-fabricated | src/measurement.rs:285, src/measurement.rs:290, src/bin/measure_graph_quality.rs:397 |
| FND-004 | medium | `protected_apparatus` copies entries verbatim; a Quoin `<dir>/**` entry (or a quoted/commented scalar) would be emitted literally and `quoin measurement record` would refuse with QM-APPARATUS-UNDECLARED instead of the producer failing closed | src/measurement.rs:53-71 |
| FND-005 | medium | `--scorer` accepts any executable; the scoring logic is no longer covered by `verify_clean_source(corpus)` (score.py was), unlike the extractor and producer which are bound to `target/release` | src/bin/measure_graph_quality.rs:118-127 |
| FND-006 | low | Frontmatter extraction (`strip_prefix("---\n")` + `split_once("\n---\n")`) is now duplicated between `validate_plan_semantics` (bin) and `protected_apparatus` (lib) | src/measurement.rs:53-57, src/bin/measure_graph_quality.rs:170-177 |

## Detail

**FND-003.** The producer lane now runs Rust binaries (`extract_tree`,
`measure_graph_quality` and `quire-corpus`) plus the Quire and Quoin CLIs. The
`python` value is whatever the caller types (`3.14.7` in every test).
`environment` and `verificationStack.toolchains` are what Quoin retains and
reports as the run's environment, so a Python identity there is misleading
attested data. Quoin does not require it: `validate/mod.rs` accepts any
`environment` object, and `validate/stack.rs` never reads `toolchains`.
Remove it in this PR. That means the flag, `Args::python_version`,
`CollectionInputs::python_version`, both JSON keys, the usage string, the
test arguments, and "Node/Rust/Python identities" in FR-012. The edit is small
and sits on the FR-012 sentence that FND-002 already rewrites.

**FND-004.** In quoin origin/main, `store/apparatus/mod.rs` resolves each
`protected_apparatus` entry. A `<directory>/**` entry becomes every file under
that directory, and `declared()` requires each resolved file to appear in
`artifacts`. MP-001 has only file entries today, so this works now. The first
glob entry would silently ship a collection that intake refuses. The fix is
to reject any entry that is not a plain relative file path (contains `*`,
quotes, or ` #`) with `InvalidPlan`, or else to resolve `/**` the way Quoin
does.

**FND-005.** MP-001's `stale-evidence` negative control says results cannot be
presented against a modified corpus checkout. While the scorer was
`<corpus>/score.py`, the clean-checkout check covered the grading logic. Now
the scorer binary can come from any build or any version, and the unit tests
pass `/usr/bin/true` or a shell script. Either bind `--scorer` to a release
build of the verified corpus checkout, as `validate_release_extractor` does
for the extractor, or state in MP-001/FR-012 that scorer provenance is the
caller's responsibility.
