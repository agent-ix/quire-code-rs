---
id: NFR-004
title: "Conservative resolution: zero wrong edges on known-binding corpora"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quire-code-rs/FR-008"
    type: "constrains"
  - target: "ix://agent-ix/quire-code-rs/StR-002"
    type: "traces_to"
---

# [NFR-004] Conservative resolution: zero wrong edges on known-binding corpora

## Statement

The library SHALL emit no incorrect `calls` edge on the known-binding precision
corpora, resolving every ambiguous call site to no edge rather than to a
best-guess target.

## Scope

- Applies to: every heuristically resolved edge — `calls`, `implements_trait`,
  `extends` and `references` — produced by
  [FR-008](../functional/FR-008-type-environments-and-call-resolution.md).
- Excludes: `contains`, `imports` and mention edges, which are read directly from
  syntax and are therefore correct by construction.

## Rationale

Precision and recall trade against each other here, and the trade is not
symmetric. A missing edge leaves an engineer where they already were — reading
the code. A wrong edge sends them somewhere irrelevant, and once they find one,
they discount every other edge in the view. A code graph that is 70% complete and
always right is useful; one that is 95% complete and occasionally invents
relationships is not consulted twice.

Stating precision as an absolute on curated corpora, and recall as a reported
number with no threshold, encodes that asymmetry in the only place it survives
contact with implementation pressure: the acceptance criteria. When a language
surface proves too hard to resolve safely, the correct response is to emit
fewer edges, never to relax this requirement.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Incorrect `calls` edges on the Rust precision corpus | 0 | 0 | Test |
| Incorrect `calls` edges on the TypeScript precision corpus | 0 | 0 | Test |
| Incorrect `calls` edges on the Python precision corpus | 0 | 0 | Test |
| Edges emitted for ambiguous call sites in the ambiguity corpus | 0 | 0 | Test |
| Recall against known bindings, per language | reported | none | Test |

## Verification

Per-language precision corpora, each a small program whose complete set of
correct call bindings is enumerated in a committed fixture, are extracted and
compared against that enumeration: every emitted edge must appear in the known
set. A separate ambiguity corpus contains call sites deliberately constructed to
be unresolvable, and asserts that no edge is emitted for any of them. Recall —
the proportion of known bindings actually recovered — is computed and reported
on every run as a regression signal, and does not gate.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-004-AC-1 | Every `calls` edge emitted on the Rust precision corpus appears in the known-binding set | Test (TC-066) |
| NFR-004-AC-2 | Every `calls` edge emitted on the TypeScript precision corpus appears in the known-binding set | Test (TC-067) |
| NFR-004-AC-3 | Every `calls` edge emitted on the Python precision corpus appears in the known-binding set | Test (TC-068) |
| NFR-004-AC-4 | No edge is emitted for any call site in the ambiguity corpus | Test (TC-069) |
| NFR-004-AC-5 | Per-language recall is computed and reported on every benchmark run | Test (TC-070) |

## Dependencies

- **Upstream**: [FR-008](../functional/FR-008-type-environments-and-call-resolution.md),
  whose tiering and unique-match rule this requirement bounds
- **Downstream**: [StR-002](../stakeholder/StR-002-requirement-code-test-traceability.md),
  whose trust argument rests on this property
