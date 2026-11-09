---
id: SR-11087
title: "PLAT-362 quire-code-rs PR56 code-review (Rust lane)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-code-rs PR56; Cargo.toml, Cargo.lock, Makefile, crates/quire-code-parse/Cargo.toml, crates/quire-code-parse/src/language.rs, crates/quire-code-parse/tests/integration.rs"
review_set: subset
---

## Summary

Ticket: PLAT-362 (part 1). Reviewed `git diff origin/main...HEAD` at head ff8125c (one commit). Rust lane (rust-review, rust-style loaded) folded in. Adds an opt-in `bash` feature to `quire-code-parse` (tree-sitter-bash 0.25.1, MIT, crates.io) and a `#[cfg(feature = "bash")] Language::Bash` variant; `Language` stays `#[non_exhaustive]`; the TC-138 rust-only cfg now also excludes `bash`; `check-single-grammar` gains a bash-absence check.

Reviewer runs (scratch worktree, locked-build lane, debug=0): `cargo test -p quire-code-parse --features bash` all pass (TC-189, TC-190 included); `cargo clippy -p quire-code-parse --all-targets --features bash -- -D warnings` clean; `make lint` and `cargo fmt --check` clean; `make check-single-grammar` passes; `cargo deny check` passes (advisories, bans, licenses, sources ok; only pre-existing wildcard-path warning). Mutation A (add `dep:tree-sitter-bash` to the `rust` feature) makes `check-single-grammar` fail with "FAIL: tree-sitter-bash is reachable", so that gate is real. Mutation B (`Language::Bash => tree_sitter_bash::NO_SUCH_LANGUAGE`) leaves `cargo clippy --workspace --all-targets -D warnings` and `cargo test --workspace --no-run` (the exact CI lint/test steps) at exit 0; only a `--features bash` build fails (exit 101).

Probes (34 shell inputs, each parsed twice with sexp equality): no false diagnostic or syntax_error for heredocs (`<<EOF`, `<<'EOF'`, `<<-EOF`, heredoc inside a function), here-string, `$( )`, backticks, `$'…'`, double quotes, comments, `case` patterns (top level and inside a function), brace group, `${x:-}}`, `$(( ))`, CRLF (plain, heredoc, case), nested functions and all FR-051-AC-35 spellings (including `my-fn.x:y`). Broken inputs that do diagnose: unclosed function, `if` without `fi`, `case` without `esac`, unterminated quote, unterminated heredoc, unclosed top-level `$(`, broken nested function. Determinism held for every input. No dependency-boundary violation (only tree-sitter-bash plus its existing cc / tree-sitter-language deps), no vendored content, no unsafe.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | No gate compiles, lints or tests the `bash` feature: CI and `make lint`/`make test` use default features and `check-single-grammar` uses rust only, so TC-189, TC-190 and the Bash arm of TC-137 never run, while spec/tests.md marks them passing and quire coverage counts them backed. Mutation B (Language::Bash bound to a non-existent symbol) passes the CI clippy and test steps | Makefile:36-42, .github/workflows/ci.yml:29-35, crates/quire-code-parse/tests/integration.rs:199-233 |
| FND-002 | medium | cargo deny (`make deny`, CI License Check) never evaluates tree-sitter-bash: it is absent from both `cargo deny list` and `cargo deny --all-features list` at the workspace root; only `--manifest-path crates/quire-code-parse/Cargo.toml --all-features` includes it (and passes, MIT). The licence and source of the new grammar is unchecked by this repo's gate | deny.toml:1-40, crates/quire-code-parse/Cargo.toml:23 |
| FND-003 | low | TC-190's broken input `f( {\n  echo hi\n}\n` is not parsed as a function: the tree is a command `f` plus a subshell with MISSING ")", and the diagnostic is at (4,0), end of file. The comment ("a function whose name is unparseable") misdescribes what is tested, and the test asserts only `is_some()`, not the line | crates/quire-code-parse/tests/integration.rs:221-233 |

## Examined scope

```yaml
scope:
- {id: FR-013-CON-3, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-1, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-3, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-4, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-5, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-6, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-AC-8, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
- {id: FR-013-CON-2, path: spec/functional/FR-013-borrowed-parse-tree-api.md, role: examined}
```

## Verdict

Changes requested (one high). The Rust change itself is minimal, idiomatic and correct: the variant is feature-gated and `#[non_exhaustive]` covers it, the grammar mapping is exhaustive under every cfg, bash is not a default feature, a rust-only build excludes tree-sitter-bash and the Makefile gate genuinely fails when it leaks in. The diagnostic predicate works well for shell on the probed inputs. The gap is gating: fix FND-001 by adding the `bash` feature to the lint and test lanes (for example `cargo clippy -p quire-code-parse --all-targets --features bash -- -D warnings` and `cargo test -p quire-code-parse --features bash` in `make lint`/`make test` and the CI Rust job). Fix FND-002 by making cargo deny see the feature (for example `[graph] features = ["quire-code-parse/bash"]` in deny.toml, or a deny invocation on the member with `--all-features`). FND-003: describe the input accurately (unclosed subshell / malformed header) and assert the diagnostic line.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | 06f7dae adds `cargo clippy -p quire-code-parse --all-targets --features bash -- -D warnings` to `make lint` and `cargo test -p quire-code-parse --features bash` to `make test`, so `make ci` now runs TC-189..TC-192 and the Bash arm of TC-137. The CI Rust job is unchanged: .github/workflows/ci.yml:32 and :35 still run bare `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`, not the make targets, so a PR's own CI still never compiles the `bash` feature. Remaining fix: point those two steps at `make lint` / `make test`, or add the two `--features bash` commands to the job. This is a workflow edit, which may need the owner's clearance first |
| FND-002 | still-open | 06f7dae adds `cargo deny --manifest-path crates/quire-code-parse/Cargo.toml --all-features check licenses` to `make deny` (re-run here: licenses ok, tree-sitter-bash evaluated). The CI License Check step (.github/workflows/ci.yml:57) still runs bare `cargo deny check licenses`, which does not see tree-sitter-bash. Same remaining fix as FND-001: use `make deny` in CI or add the member invocation |
| FND-003 | fixed | 06f7dae: TC-190 now parses an unclosed function and asserts file and line, with an accurate comment and test name |
