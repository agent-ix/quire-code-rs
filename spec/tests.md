---
id: TM-001
title: "quire-code-rs Test Matrix"
type: TestMatrix
---

# Test Matrix

## Overview

This matrix maps every acceptance criterion in `spec/` to one or more test
cases. Test cases are numbered sequentially within this repository and are never
reused.

Every test carries its `TC-NNN` identifier as a comment adjacent to the test
function. That convention is not decoration: it is the input to
[FR-005](./functional/FR-005-spec-mention-harvesting.md), so this repository is
the first consumer of its own mention-harvesting behavior, and the matrix below
is checkable by extracting this repository with the library it specifies.

## Test Matrix Rules

1. **Coverage Rule** — every acceptance criterion has at least one test case.
2. **Language Permutation Rule** — every structural behavior is exercised for
   Rust, TypeScript/TSX and Python, not for one representative language.
3. **Conservatism Rule** — every resolution tier has both a positive test (the
   edge is emitted with the expected `reason`) and a negative test (ambiguity
   yields no edge).
4. **Determinism Rule** — every emitting path is covered by a golden-file or
   repeat-extraction assertion.
5. **Error Path Rule** — every diagnostic variant has at least one negative
   test.
6. **Edge Case Rule** — empty files, files declaring nothing, anonymous
   declarations, unresolvable imports, invalid UTF-8 and binding cycles have
   dedicated test cases.

---

## Requirements Traceability

### Stakeholder Requirement Coverage

| Stakeholder Req | Trace to US/FR | Test/Validation | Coverage Status |
|---|---|---|---|
| StR-001 Single deterministic engine | US-001, FR-001, FR-006, NFR-001, NFR-002 | TC-001, TC-038, TC-054, TC-059 | ✅ Complete |
| StR-002 Recovered traceability | US-002, US-003, FR-005, FR-008, NFR-004 | TC-026, TC-044, TC-066, TC-069 | ✅ Complete |

### User Story Coverage

| User Story | Trace to FR | Test/Validation | Coverage Status |
|---|---|---|---|
| US-001 Index a code layer | FR-001, FR-002, FR-003, FR-006, FR-007 | TC-001..TC-019, TC-033..TC-043 | ✅ Complete |
| US-002 Trace requirement to code and tests | FR-005 | TC-026..TC-032 | ✅ Complete |
| US-003 Follow call relationships | FR-004, FR-008 | TC-020..TC-025, TC-044..TC-053 | ✅ Complete |

### Functional Requirement Coverage

| AC | Criteria summary | TC | Type | Status |
|---|---|---|---|---|
| FR-001-AC-1 | Rust fixture yields all four fact types | TC-001 | Unit | ✅ |
| FR-001-AC-2 | TS/TSX fixture yields class, interface, alias, method, arrow facts | TC-002 | Unit | ✅ |
| FR-001-AC-3 | Python fixture yields module, class, method, function facts | TC-003 | Unit | ✅ |
| FR-001-AC-4 | Empty file still yields one `code_file` fact | TC-004 | Unit | ✅ |
| FR-001-AC-5 | Facts carry `kind` and one-based inclusive line spans | TC-005 | Unit | ✅ |
| FR-001-AC-6 | Facts ordered by start position across runs | TC-006 | Unit | ✅ |
| FR-001-AC-7 | No filesystem or network crate reachable from extraction | TC-007 | Unit | ✅ |
| FR-002-AC-1 | Free function qualified name shape | TC-008 | Unit | ✅ |
| FR-002-AC-2 | Method named with implementing type as parent | TC-009 | Unit | ✅ |
| FR-002-AC-3 | `code_file` name carries no `::` segment | TC-010 | Unit | ✅ |
| FR-002-AC-4 | Same repo under two orgs yields disjoint names | TC-011 | Unit | ✅ |
| FR-002-AC-5 | Anonymous declarations get ordinal segments surviving a line shift | TC-012 | Unit | ✅ |
| FR-002-AC-6 | Windows-style paths normalize to forward slashes | TC-013 | Unit | ✅ |
| FR-002-AC-7 | Every `ix://` reference has at least three segments | TC-014 | Unit | ✅ |
| FR-003-AC-1 | Containment forms a tree rooted at the file | TC-015 | Unit | ✅ |
| FR-003-AC-2 | Relative import in batch yields `path-resolved` edge | TC-016 | Unit | ✅ |
| FR-003-AC-3 | Bare import: no edge, no diagnostic; broken relative import: one diagnostic | TC-017 | Unit | ✅ |
| FR-003-AC-4 | Extensionless import resolves via language conventions | TC-018 | Unit | ✅ |
| FR-003-AC-5 | Structural edges carry confidence 1.0 | TC-019 | Unit | ✅ |
| FR-004-AC-1 | Two sites, one triple, `count` 2 | TC-020 | Unit | ✅ |
| FR-004-AC-2 | Thirty sites yield `count` 30 and 20 evidence entries | TC-021 | Unit | ✅ |
| FR-004-AC-3 | Highest confidence and its `reason` win | TC-022 | Unit | ✅ |
| FR-004-AC-4 | `reason` in enum, confidence within [0,1] | TC-023 | Unit | ✅ |
| FR-004-AC-5 | Evidence ordered by file then line | TC-024 | Unit | ✅ |
| FR-004-AC-6 | Syntactic reasons carry confidence 1.0 | TC-025 | Unit | ✅ |
| FR-005-AC-1 | Tracking tag attributed to its test function | TC-026 | Unit | ✅ |
| FR-005-AC-2 | Requirement citation attributed to file's code fact | TC-027 | Unit | ✅ |
| FR-005-AC-3 | `ix://` reference harvested | TC-028 | Unit | ✅ |
| FR-005-AC-4 | String literal yields no mention | TC-029 | Unit | ✅ |
| FR-005-AC-5 | Embedded token yields no mention | TC-030 | Unit | ✅ |
| FR-005-AC-6 | Unresolvable mention still reported | TC-031 | Unit | ✅ |
| FR-005-AC-7 | Self-extraction recovers this suite's own tags | TC-032 | Integration | ✅ |
| FR-005-AC-8 | Tag outside a test declaration is a citation, not a verification claim | TC-072 | Unit | ✅ |
| FR-006-AC-1 | Node records carry hex id, `ix://` ref, `kind` | TC-033 | Unit | ✅ |
| FR-006-AC-2 | Moving a declaration preserves its id | TC-034 | Unit | ✅ |
| FR-006-AC-3 | Records appear in stable order | TC-035 | Unit | ✅ |
| FR-006-AC-4 | Edge types drawn from the six-value set | TC-036 | Unit | ✅ |
| FR-006-AC-5 | No timestamp, absolute path, hostname or process id | TC-037 | Unit | ✅ |
| FR-006-AC-6 | Fixture output matches golden byte for byte | TC-038 | Integration | ✅ |
| FR-007-AC-1 | Invalid file yields diagnostic, batch continues | TC-039 | Unit | ✅ |
| FR-007-AC-2 | Diagnostic carries path and first error position | TC-040 | Unit | ✅ |
| FR-007-AC-3 | Intact declarations survive a malformed sibling declaration | TC-041 | Unit | ✅ |
| FR-007-AC-4 | Healthy files unaffected by a malformed sibling | TC-042 | Unit | ✅ |
| FR-007-AC-5 | Arbitrary bytes yield a diagnostic, never a panic | TC-043 | Unit | ✅ |
| FR-007-AC-6 | Error-node root yields the `code_file` fact alone | TC-071 | Unit | ✅ |
| FR-008-AC-1 | Cross-file receiver-typed call resolves | TC-044 | Unit | ✅ |
| FR-008-AC-2 | Unrecoverable receiver with many candidates yields nothing | TC-045 | Unit | ✅ |
| FR-008-AC-3 | Copy binding resolves through fixpoint | TC-046 | Unit | ✅ |
| FR-008-AC-4 | Call-result binding resolves through fixpoint | TC-047 | Unit | ✅ |
| FR-008-AC-5 | Local rebinding does not leak to file level | TC-048 | Unit | ✅ |
| FR-008-AC-6 | Cyclic bindings terminate at the iteration bound | TC-049 | Unit | ✅ |
| FR-008-AC-7 | `implements_trait` and `extends` edges emitted | TC-050 | Unit | ✅ |
| FR-008-AC-8 | Rust trait-object call yields no edge | TC-051 | Unit | ✅ |
| FR-008-AC-9 | Same-file resolution independent of batch composition | TC-052 | Unit | ✅ |
| FR-008-AC-10 | Resolution tiers stamp their own `reason` in rank order | TC-053 | Unit | ✅ |
| FR-008-AC-11 | Result reports batch file count and unresolved call-site count | TC-073 | Unit | ✅ |
| FR-008-AC-12 | A name declared in two files still resolves within each | TC-074 | Integration | ✅ |

### Non-Functional Requirement Coverage

| AC | Criteria summary | TC | Type | Status |
|---|---|---|---|---|
| NFR-001-AC-1 | One hundred repeated extractions byte-identical | TC-054 | Integration | ✅ |
| NFR-001-AC-2 | Eight concurrent extractions byte-identical | TC-055 | Integration | ✅ |
| NFR-001-AC-3 | Shuffled batch order changes nothing | TC-056 | Integration | ✅ |
| NFR-001-AC-4 | No order-observable hash iteration | TC-057 | Unit | ✅ |
| NFR-001-AC-5 | No clock, randomness, process or environment read | TC-058 | Unit | ✅ |
| NFR-002-AC-1 | No HTTP, RPC or socket crate in the closure | TC-059 | Unit | ✅ |
| NFR-002-AC-2 | No filesystem, environment or spawn call in extraction | TC-060 | Unit | ✅ |
| NFR-002-AC-3 | Network-isolated run matches online run | TC-061 | Integration | ✅ |
| NFR-003-AC-1 | Full benchmark corpus within 60 s, single core | TC-062 | Benchmark | ⬜ |
| NFR-003-AC-2 | Single-file re-extraction p95 within 50 ms | TC-063 | Benchmark | ⬜ |
| NFR-003-AC-3 | Peak resident memory within 2.0 GB | TC-064 | Benchmark | ⬜ |
| NFR-003-AC-4 | Fixpoint converges within ten iterations | TC-065 | Benchmark | ⬜ |
| NFR-004-AC-1 | Zero wrong edges, Rust precision corpus | TC-066 | Integration | ✅ |
| NFR-004-AC-2 | Zero wrong edges, TypeScript precision corpus | TC-067 | Integration | ✅ |
| NFR-004-AC-3 | Zero wrong edges, Python precision corpus | TC-068 | Integration | ✅ |
| NFR-004-AC-4 | No edge for any ambiguity-corpus call site | TC-069 | Integration | ✅ |
| NFR-004-AC-5 | Per-language recall computed and reported | TC-070 | Benchmark | ⬜ |

---

## Test Case Summary

| Range | Owning requirement | Delivery slice |
|---|---|---|
| TC-001..TC-007 | FR-001 structural fact model | Structural extraction |
| TC-008..TC-014 | FR-002 symbol identity | Structural extraction |
| TC-015..TC-019 | FR-003 structural edges | Structural extraction |
| TC-020..TC-025 | FR-004 provenance and deduplication | Structural extraction |
| TC-026..TC-032 | FR-005 mention harvesting | Mention linker |
| TC-033..TC-038 | FR-006 canonical emission | Canonical emission |
| TC-039..TC-043 | FR-007 parse-error isolation | Structural extraction |
| TC-044..TC-053 | FR-008 type environments and call resolution | Receiver-typed resolution |
| TC-054..TC-058 | NFR-001 determinism | Canonical emission |
| TC-059..TC-061 | NFR-002 no-network boundary | Structural extraction |
| TC-062..TC-065 | NFR-003 extraction time budget | Receiver-typed resolution |
| TC-066..TC-070 | NFR-004 conservative-resolution precision | Receiver-typed resolution |
| TC-071 | FR-007 error-node root handling | Structural extraction |
| TC-072 | FR-005 tag context restriction | Mention linker |
| TC-073 | FR-008 batch-bound resolution reporting | Receiver-typed resolution |
| TC-074 | FR-008 same-file preference over batch ambiguity | Receiver-typed resolution |

## Coverage Notes

- **Status legend**: ⬜ Planned (matrix row authored, test not yet written),
  ✅ Complete (test written, tagged and green).
- Benchmark-verified criteria (TC-062..TC-065, TC-070) report measurements on
  every run and gate on threshold only in the performance lane, so that
  shared-runner variance does not make ordinary CI flaky.
- TC-032 extracts this repository's own test suite and serves as the dogfooding
  check that the tracking tags this matrix claims are really present in the
  tests.
- The five benchmark rows (TC-062..TC-065, TC-070) remain ⬜: the committed
  benchmark corpus they measure against is not yet authored. TC-070's recall
  reporting is exercised today by the precision suite, which prints per-language
  recall on every run; the corpus-scale figure is what remains.
