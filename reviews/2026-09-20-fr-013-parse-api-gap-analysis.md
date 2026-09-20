---
id: SR-008
title: "gap analysis of FR-013 borrowed parse-tree API (PR #22)"
type: SpecReview
analysis: gap-analysis
scope: "spec/functional/FR-013-borrowed-parse-tree-api.md, spec/usecase/US-005-share-parse-trees-with-external-consumers.md, spec/tests.md TC-135..TC-156, crates/quire-code-parse/ at 86acf7d"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-013
    type: reviews
  - target: ix://agent-ix/quire-code-rs/TM-001
    type: references
---

## Summary

FR-013's implementation is substantially complete and every TC-135..TC-156 row
resolves to a real, non-vacuous test symbol. Three gaps remain. The
declaration-structure predicate implements a positional rule (root and its
direct children) while FR-013's Behavior section states a semantic one (the
declaration's own kind, name and signature remain resolvable); the two disagree
for every declaration nested one level down, and no test covers the disagreement.
FR-013-AC-9 and FR-013-AC-10 are cited by the FR but appear in no test's
tracking tag, so `quire coverage` reports both unbacked while the gate passes
by crediting them through the TC ids they cite. TC-137's in-source tag still
names FR-013-AC-4 after the matrix re-pointed that row to FR-013-CON-3.

## Verdict

**FAIL** — FND-001 is a high-severity behavior/requirement disagreement, and
FND-002 and FND-003 are matrix rows whose tracking tags do not bind what the
row claims.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | high | `has_declaration_structure_error` inspects only the root and its direct children, so a declaration tree-sitter could not resolve at all one level down — a Python method in a class, a Rust `fn` in `mod tests`, a TypeScript function in a namespace — returns `Ok` with no diagnostic; FR-013's Behavior clause instead states a semantic condition the code does not implement. | FR-013, crates/quire-code-parse/src/parse.rs:190 |
| FND-002 | medium | FR-013-AC-9 and FR-013-AC-10 appear in no test's tracking tag; TC-151, TC-152 and TC-154 still carry `FR-013-AC-3`. `quire coverage` reports both criteria unbacked while the gate passes via the cited TC ids. | FR-013, crates/quire-code-parse/tests/integration.rs:80 |
| FND-003 | medium | TC-137's in-source tag names `FR-013-AC-4` while spec/tests.md re-pointed the row to `FR-013-CON-3`; this makes `FR-013-AC-4` report backed, contradicting the FR's own statement that the criterion mints no TC id. | FR-013, crates/quire-code-parse/src/language.rs:56 |
| FND-004 | low | `ParseError<'src>` cannot satisfy `'static`, so a consumer cannot box it, convert it with `anyhow`, or retain it past the source borrow. No requirement owns this consequence. | FR-013, crates/quire-code-parse/src/error.rs:30 |
| FND-005 | low | No plan bundle targets FR-013, so this gate's plan-completion step has nothing to assert against; the repository's own precedent is `plan/Plan-001-governed-graph-quality`. | FR-013 |

## Coverage

- Targeted plan bundle: none exists for FR-013; plan completion not assertable.
- Matrix Test Cases backed by tracking tags: 22 / 22 for TC-135..TC-156.
- Acceptance criteria with a direct in-source tracking tag: 7 / 10 for FR-013
  (AC-8 is `Inspection` by declaration; AC-9 and AC-10 are FND-002).
- Untraced production behavior: the `file` identifier's borrowed lifetime and
  the `u32::MAX` saturation sentinel for an unrepresentable row position.
- Production stubs and internal-logic mocks: 0.
- Semantic review: performed for FR-013-AC-3 and FR-013-AC-10 only, which is
  where FND-001 was found.
