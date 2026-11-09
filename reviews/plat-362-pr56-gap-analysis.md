---
id: SR-11088
title: "PLAT-362 quire-code-rs PR56 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-code-rs PR56; crates/quire-code-parse, spec/functional/FR-013-borrowed-parse-tree-api.md, spec/tests.md; consumer contract agent-ix/quire-rs spec/functional/FR-051-source-symbol-extraction.md AC-34..AC-46, CR-213"
review_set: subset
---

## Summary

Ticket: PLAT-362 (part 1). Plan completion: not assessed (planless). `quoin matrix --json` (quoin 0.29.1) at ff8125c: FR-013-CON-3 tagged (TC-137 `every_compiled_language_loads_its_grammar`, TC-138 `rust_only_build_has_one_language_variant`); FR-013-AC-4 method-without-symbol (Inspection via `make check-single-grammar`, which the PR extends and which the reviewer's mutation shows fails on a bash leak). `scripts/check_coverage.sh`: TC-189 and TC-190 backed; the only failures are TC-133/TC-134, untouched by this PR. No stubs, no untraced production code: the one production change is the gated `Language::Bash` arm.

Consumer contract (quire-rs FR-051, CR-213 note): the shell adapter needs a `bash` feature of `quire-code-parse` (delivered), parses via tree-sitter-bash (delivered), and reads `ParsedFile::diagnostic()` the same way `src/symbols/{rust,python,typescript}.rs` do, skipping the file on `Some`. So FR-051-AC-36 ("no false unterminated-structure diagnostic" for heredocs, quotes, `$( )`, case patterns, CRLF, …) needs `diagnostic()` to stay `None` on those inputs, and FR-051-AC-45 needs it to be `Some` on an unparseable file. Reviewer probes: every AC-36 input gives `diagnostic() == None` and `syntax_error() == None` and every AC-35 spelling yields its `function_definition` node; common broken inputs (unclosed function, `if` without `fi`, `case` without `esac`, unterminated quote or heredoc, unclosed `$(`) give `Some`. Two shapes behave differently from FR-013's stated rule; see the findings. Side note for part 2, not a defect here: `y=$(f() { :; }; f)` yields a real `function_definition` named `f` inside a `$( )`; the AC-36 wording ("a definition-shaped line inside a … `$( )` string") reads as if it should mint nothing, so the adapter or AC-36 must decide.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-013's body-local rule (AC-10) and its Known limitations do not cover Bash, and Bash departs from it. A body-local error inside a shell function (`f() {\n  echo $(\n}\n`) collapses the whole function into a top-level ERROR, so `diagnostic()` is Some (1,1) and the consumer skips the file. A stray `fi`, `done` or `}` gives neither `diagnostic()` nor `syntax_error()`, because the grammar parses it as a command word. Neither behaviour is disclosed or pinned by a test | spec/functional/FR-013-borrowed-parse-tree-api.md:158, spec/functional/FR-013-borrowed-parse-tree-api.md:189-215, crates/quire-code-parse/src/parse.rs:402-433 |
| FND-002 | medium | Coverage inflation: TC-189 and TC-190 are ✅ and count as backed, but no gate executes them (they compile only under `--features bash`, which no Makefile or CI step enables; see SR-11087 FND-001) | spec/tests.md:286-287 |

## Verdict

Changes requested. The delivered feature meets what the quire-rs adapter needs: shell parsing, a gated variant and a working structural diagnostic on the inputs FR-051-AC-35/36/45 name. Close FND-002 with SR-11087 FND-001's gate fix. Close FND-001 by adding a Bash bullet to FR-013 "Known limitations" stating both shapes (a function-body error reads as structural; a stray closing keyword is accepted silently), with a test pinning each, and by updating the `executable_body_field` / `is_declaration_node` doc comments, which still say "all three grammars" although Bash's `function_definition` now flows through the same list.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | TC-191 is tagged `FR-013-AC-10` and so counts as backing it, but it asserts the opposite of AC-10's outcome: `diagnostic()` is Some for a body-local error, where AC-10 requires None. The analogous Python limitation tests (TC-174, TC-175) bind FR-013-AC-3, the criterion their behaviour satisfies. So AC-10 is reported backed by a test that demonstrates an exception to it | crates/quire-code-parse/tests/integration.rs:239-251, spec/tests.md:288 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 06f7dae adds a Bash bullet to FR-013 Known limitations covering both shapes, pins each with a test (TC-191: body-local error gives diagnostic (1,1); TC-192: `fi`/`done`/`}` give neither diagnostic nor syntax_error), and corrects the is_declaration_node / executable_body_field doc comments |
| FND-002 | fixed | 06f7dae: `make test` now runs `cargo test -p quire-code-parse --features bash`, so `make ci` (the merge gate) executes TC-189..TC-192. The CI job still runs bare cargo; that residual is tracked as SR-11087 FND-001 |
