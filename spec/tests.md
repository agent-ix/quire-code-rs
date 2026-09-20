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
| StR-001 Single deterministic engine | US-001, FR-001, FR-006, NFR-001, NFR-002, US-005, FR-013 | TC-001, TC-038, TC-054, TC-059, TC-135..TC-150 | ✅ Complete |
| StR-002 Recovered traceability | US-002, US-003, FR-005, FR-008, NFR-004 | TC-026, TC-044, TC-066, TC-069 | ✅ Complete |
| StR-003 Governed extractor-quality observations | US-004, FR-011, FR-012, NFR-005 | TC-108..TC-111 | ✅ Complete |

### User Story Coverage

| User Story | Acceptance Criteria | Test Cases | Coverage Status |
|---|---|---|---|
| US-001 Index a code layer | FR-001, FR-002, FR-003, FR-006, FR-007, FR-009 | TC-001..TC-019, TC-033..TC-043, TC-078..TC-086 | ✅ Complete |
| US-002 Trace requirement to code and tests | FR-005 | TC-026..TC-032 | ✅ Complete |
| US-003 Follow call relationships | FR-004, FR-008 | TC-020..TC-025, TC-044..TC-053 | ✅ Complete |
| US-004 Assess versioned extractor quality | FR-011, FR-012 | TC-108, TC-109, TC-120 | ✅ Complete |
| US-005 Share parse trees with an external consumer | FR-013 | TC-135..TC-150 | ✅ Complete |

### Functional Requirement Coverage

Every requirement, its acceptance criteria, and the test cases that discharge
them. `Status` is ✅ only while `quire coverage` reports the row's
targets backed; a row whose test is unwritten carries 🚧 and says so.

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
|---|---|---|---|
| FR-001 | FR-001-AC-1, FR-001-AC-2, FR-001-AC-3, FR-001-AC-8, FR-001-AC-4, FR-001-AC-5, FR-001-AC-6, FR-001-AC-7, FR-001-AC-9, FR-001-AC-10 | TC-001, TC-002, TC-003, TC-004, TC-005, TC-006, TC-007, TC-076, TC-089, TC-090, TC-091, TC-100 | ✅ |
| FR-002 | FR-002-AC-1, FR-002-AC-2, FR-002-AC-3, FR-002-AC-4, FR-002-AC-5, FR-002-AC-6, FR-002-AC-7 | TC-008, TC-009, TC-010, TC-011, TC-012, TC-013, TC-014 | ✅ |
| FR-003 | FR-003-AC-1, FR-003-AC-2, FR-003-AC-3, FR-003-AC-4, FR-003-AC-5, FR-003-AC-6 | TC-015, TC-016, TC-017, TC-018, TC-019, TC-092 | ✅ |
| FR-004 | FR-004-AC-1, FR-004-AC-2, FR-004-AC-3, FR-004-AC-4, FR-004-AC-5, FR-004-AC-6, FR-004-CON-1 | TC-020, TC-021, TC-022, TC-023, TC-024, TC-025, TC-088 | ✅ |
| FR-005 | FR-005-AC-1, FR-005-AC-2, FR-005-AC-3, FR-005-AC-4, FR-005-AC-5, FR-005-AC-6, FR-005-AC-7, FR-005-AC-8, FR-005-AC-9, FR-005-AC-10 | TC-026, TC-027, TC-028, TC-029, TC-030, TC-031, TC-032, TC-072, TC-093, TC-094, TC-095, TC-096, TC-098, TC-099 | ✅ |
| FR-006 | FR-006-AC-1, FR-006-AC-2, FR-006-AC-3, FR-006-AC-4, FR-006-AC-5, FR-006-AC-6, FR-006-CON-1, FR-006-AC-7 | TC-033, TC-034, TC-035, TC-036, TC-037, TC-038, TC-087, TC-106 | ✅ |
| FR-007 | FR-007-AC-1, FR-007-AC-2, FR-007-AC-3, FR-007-AC-4, FR-007-AC-5, FR-007-AC-6, FR-007-AC-7 | TC-039, TC-040, TC-041, TC-042, TC-043, TC-071, TC-075 | ✅ |
| FR-008 | FR-008-AC-1, FR-008-AC-2, FR-008-AC-3, FR-008-AC-4, FR-008-AC-5, FR-008-AC-6, FR-008-AC-7, FR-008-AC-8, FR-008-AC-9, FR-008-AC-10, FR-008-AC-11, FR-008-AC-12, FR-008-AC-13 | TC-044, TC-045, TC-046, TC-047, TC-048, TC-049, TC-050, TC-051, TC-052, TC-053, TC-073, TC-074, TC-097, TC-107 | ✅ |
| FR-009 | FR-009-AC-1, FR-009-AC-2, FR-009-AC-3, FR-009-AC-4, FR-009-AC-5, FR-009-AC-6, FR-009-AC-7, FR-009-AC-8, FR-009-AC-9 | TC-078, TC-079, TC-080, TC-081, TC-082, TC-083, TC-084, TC-085, TC-086 | ✅ |
| NFR-001 | NFR-001-AC-1, NFR-001-AC-2, NFR-001-AC-3, NFR-001-AC-4, NFR-001-AC-5 | TC-054, TC-055, TC-056, TC-057, TC-058 | ✅ |
| NFR-002 | NFR-002-AC-1, NFR-002-AC-2, NFR-002-AC-3 | TC-059, TC-060, TC-061 | ✅ |
| NFR-003 | NFR-003-AC-1, NFR-003-AC-2, NFR-003-AC-3, NFR-003-AC-4, NFR-003-AC-5 | TC-062, TC-063, TC-064, TC-065, TC-077 | ✅ |
| NFR-004 | NFR-004-AC-1, NFR-004-AC-2, NFR-004-AC-3, NFR-004-AC-4, NFR-004-AC-5 | TC-066, TC-067, TC-068, TC-069, TC-070 | ✅ |
| FR-010 | FR-010-AC-1, FR-010-AC-2, FR-010-AC-3, FR-010-AC-4, FR-010-AC-5 | TC-101, TC-102, TC-103, TC-104, TC-105 | ✅ |
| FR-011 | FR-011-AC-1, FR-011-AC-2, FR-011-AC-3, FR-011-AC-4, FR-011-AC-5, FR-011-AC-6, FR-011-CON-1, FR-011-CON-2 | TC-112..TC-117, TC-130, TC-131 | ✅ |
| FR-012 | FR-012-AC-1, FR-012-AC-2, FR-012-AC-3, FR-012-AC-4, FR-012-AC-5, FR-012-AC-6, FR-012-AC-7, FR-012-AC-8, FR-012-CON-1, FR-012-CON-2, FR-012-CON-3 | TC-118..TC-125, TC-132, TC-133, TC-134 | ✅ |
| NFR-005 | NFR-005-AC-1, NFR-005-AC-2, NFR-005-AC-3 | TC-126..TC-128 | ✅ |
| FR-013 | FR-013-AC-1, FR-013-AC-2, FR-013-AC-3, FR-013-AC-4, FR-013-AC-5, FR-013-AC-6, FR-013-AC-7, FR-013-AC-8, FR-013-CON-1, FR-013-CON-2, FR-013-CON-3, FR-013-CON-4 | TC-135..TC-150 | ✅ |

### Measurement Plan Coverage

| Measurement Plan | Verification | Test Cases | Coverage Status |
|---|---|---|---|
| MP-001 | Active plan validates; complete census, two repetitions, zero-wrong-edge decision, independent recall, and non-measured handling are enforced | TC-129 | ✅ Complete |

---

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
|---|---|---|---|---|---|
| TC-001 | Rust fixture yields all four fact types | Unit | P1 | FR-001-AC-1 | ✅ |
| TC-002 | TS/TSX fixture yields class, interface, alias, method, arrow facts | Unit | P1 | FR-001-AC-2 | ✅ |
| TC-003 | Python fixture yields class, method, function facts; file fact is the module | Unit | P1 | FR-001-AC-3 | ✅ |
| TC-004 | Empty file still yields one `code_file` fact | Unit | P1 | FR-001-AC-4 | ✅ |
| TC-005 | Facts carry `kind` and one-based inclusive line spans | Unit | P1 | FR-001-AC-5 | ✅ |
| TC-006 | Facts ordered by start position across runs | Unit | P1 | FR-001-AC-6 | ✅ |
| TC-007 | No filesystem or network crate reachable from extraction | Unit | P1 | FR-001-AC-7 | ✅ |
| TC-008 | Free function qualified name shape | Unit | P1 | FR-002-AC-1 | ✅ |
| TC-009 | Method named with implementing type as parent | Unit | P1 | FR-002-AC-2 | ✅ |
| TC-010 | `code_file` name carries no `::` segment | Unit | P1 | FR-002-AC-3 | ✅ |
| TC-011 | Same repo under two orgs yields disjoint names | Unit | P1 | FR-002-AC-4 | ✅ |
| TC-012 | Anonymous declarations get ordinal segments surviving a line shift | Unit | P1 | FR-002-AC-5 | ✅ |
| TC-013 | Windows-style paths normalize to forward slashes | Unit | P1 | FR-002-AC-6 | ✅ |
| TC-014 | Every `ix://` reference has at least three segments | Unit | P1 | FR-002-AC-7 | ✅ |
| TC-015 | Containment forms a tree rooted at the file | Unit | P1 | FR-003-AC-1 | ✅ |
| TC-016 | Relative import in batch yields `path-resolved` edge | Unit | P1 | FR-003-AC-2 | ✅ |
| TC-017 | Bare import: no edge, no diagnostic; broken relative import: one diagnostic | Unit | P1 | FR-003-AC-3 | ✅ |
| TC-018 | Extensionless import resolves via language conventions | Unit | P1 | FR-003-AC-4 | ✅ |
| TC-019 | Structural edges carry confidence 1.0 | Unit | P1 | FR-003-AC-5 | ✅ |
| TC-020 | Two sites, one triple, `count` 2 | Unit | P1 | FR-004-AC-1 | ✅ |
| TC-021 | Thirty sites yield `count` 30 and 20 evidence entries | Unit | P1 | FR-004-AC-2 | ✅ |
| TC-022 | Highest confidence and its `reason` win | Unit | P1 | FR-004-AC-3 | ✅ |
| TC-023 | `reason` in enum, confidence within [0,1] | Unit | P1 | FR-004-AC-4 | ✅ |
| TC-024 | Evidence ordered by file then line | Unit | P1 | FR-004-AC-5 | ✅ |
| TC-025 | Syntactic reasons carry confidence 1.0 | Unit | P1 | FR-004-AC-6 | ✅ |
| TC-026 | Tracking tag attributed to its test function | Unit | P1 | FR-005-AC-1 | ✅ |
| TC-027 | Requirement citation attributed to file's code fact | Unit | P1 | FR-005-AC-2 | ✅ |
| TC-028 | `ix://` reference harvested | Unit | P1 | FR-005-AC-3 | ✅ |
| TC-029 | String literal yields no mention | Unit | P1 | FR-005-AC-4 | ✅ |
| TC-030 | Embedded token yields no mention | Unit | P1 | FR-005-AC-5 | ✅ |
| TC-031 | Unresolvable mention still reported | Unit | P1 | FR-005-AC-6 | ✅ |
| TC-032 | Self-extraction recovers this suite's own tags | Integration | P1 | FR-005-AC-7 | ✅ |
| TC-033 | Node records carry hex id, `ix://` ref, `kind` | Unit | P1 | FR-006-AC-1 | ✅ |
| TC-034 | Moving a declaration preserves its id | Unit | P1 | FR-006-AC-2 | ✅ |
| TC-035 | Records appear in stable order | Unit | P1 | FR-006-AC-3 | ✅ |
| TC-036 | Edge types drawn from the six-value set | Unit | P1 | FR-006-AC-4 | ✅ |
| TC-037 | No timestamp, absolute path, hostname or process id | Unit | P1 | FR-006-AC-5 | ✅ |
| TC-038 | Fixture output matches golden byte for byte | Integration | P1 | FR-006-AC-6 | ✅ |
| TC-039 | Invalid file yields diagnostic, batch continues | Unit | P1 | FR-007-AC-1 | ✅ |
| TC-040 | Diagnostic carries path and first error position | Unit | P1 | FR-007-AC-2 | ✅ |
| TC-041 | Intact declarations survive a malformed sibling declaration | Unit | P1 | FR-007-AC-3 | ✅ |
| TC-042 | Healthy files unaffected by a malformed sibling | Unit | P1 | FR-007-AC-4 | ✅ |
| TC-043 | Arbitrary bytes yield a diagnostic, never a panic | Unit | P1 | FR-007-AC-5 | ✅ |
| TC-044 | Cross-file receiver-typed call resolves | Unit | P1 | FR-008-AC-1 | ✅ |
| TC-045 | Unrecoverable receiver with many candidates yields nothing | Unit | P1 | FR-008-AC-2 | ✅ |
| TC-046 | Copy binding resolves through fixpoint | Unit | P1 | FR-008-AC-3 | ✅ |
| TC-047 | Call-result binding resolves through fixpoint | Unit | P1 | FR-008-AC-4 | ✅ |
| TC-048 | Local rebinding does not leak to file level | Unit | P1 | FR-008-AC-5 | ✅ |
| TC-049 | Cyclic bindings terminate at the iteration bound | Unit | P1 | FR-008-AC-6 | ✅ |
| TC-050 | `implements_trait` and `extends` edges emitted | Unit | P1 | FR-008-AC-7 | ✅ |
| TC-051 | Rust trait-object call yields no edge | Unit | P1 | FR-008-AC-8 | ✅ |
| TC-052 | Same-file resolution independent of batch composition | Unit | P1 | FR-008-AC-9 | ✅ |
| TC-053 | Resolution tiers stamp their own `reason` in rank order | Unit | P1 | FR-008-AC-10 | ✅ |
| TC-054 | One hundred repeated extractions byte-identical | Integration | P1 | NFR-001-AC-1 | ✅ |
| TC-055 | Eight concurrent extractions byte-identical | Integration | P1 | NFR-001-AC-2 | ✅ |
| TC-056 | Shuffled batch order changes nothing | Integration | P1 | NFR-001-AC-3 | ✅ |
| TC-057 | No order-observable hash iteration | Unit | P1 | NFR-001-AC-4 | ✅ |
| TC-058 | No clock, randomness, process or environment read | Unit | P1 | NFR-001-AC-5 | ✅ |
| TC-059 | No HTTP, RPC or socket crate in the closure | Unit | P1 | NFR-002-AC-1 | ✅ |
| TC-060 | No filesystem, environment or spawn call in extraction | Unit | P1 | NFR-002-AC-2 | ✅ |
| TC-061 | Network-isolated run matches online run | Analysis | P1 | NFR-002-AC-3 | ✅ |
| TC-062 | Full benchmark corpus within 60 s, single core | Benchmark | P1 | NFR-003-AC-1 | ✅ |
| TC-063 | Single-file re-extraction p95 within 50 ms | Benchmark | P1 | NFR-003-AC-2 | ✅ |
| TC-064 | Peak resident memory within 2.0 GB | Benchmark | P1 | NFR-003-AC-3 | ✅ |
| TC-065 | Fixpoint converges within ten iterations | Benchmark | P1 | NFR-003-AC-4 | ✅ |
| TC-066 | Zero wrong edges, Rust precision corpus | Integration | P1 | NFR-004-AC-1 | ✅ |
| TC-067 | Zero wrong edges, TypeScript precision corpus | Integration | P1 | NFR-004-AC-2 | ✅ |
| TC-068 | Zero wrong edges, Python precision corpus | Integration | P1 | NFR-004-AC-3 | ✅ |
| TC-069 | No edge for any ambiguity-corpus call site | Integration | P1 | NFR-004-AC-4 | ✅ |
| TC-070 | Per-language recall computed and reported | Benchmark | P1 | NFR-004-AC-5 | ✅ |
| TC-071 | Error-node root yields the `code_file` fact alone | Unit | P1 | FR-007-AC-6 | ✅ |
| TC-072 | Tag outside a test declaration is a citation, not a verification claim | Unit | P1 | FR-005-AC-8 | ✅ |
| TC-073 | Result reports batch file count and unresolved call-site count | Unit | P1 | FR-008-AC-11 | ✅ |
| TC-074 | A name declared in two files still resolves within each | Integration | P1 | FR-008-AC-12 | ✅ |
| TC-075 | A body error does not cost the declaration its fact | Unit | P1 | FR-007-AC-7 | ✅ |
| TC-076 | Python file yields no `code_module`; Rust `mod` and TS `namespace` do | Unit | P1 | FR-001-AC-8 | ✅ |
| TC-077 | Caller-supplied parses are reused, and change nothing about the records | Integration | P1 | NFR-003-AC-5 | ✅ |
| TC-078 | Rust `pub`, `pub(crate)`/`pub(super)`, and bare declarations classify public/crate/private | Unit | P1 | FR-009-AC-1 | ✅ |
| TC-079 | TypeScript `export`, unexported, and class access modifiers classify correctly | Unit | P1 | FR-009-AC-2 | ✅ |
| TC-080 | Python `__helper`, `_helper`, `helper` and `__init__` classify private/crate/public/public | Unit | P1 | FR-009-AC-3 | ✅ |
| TC-081 | Rust trait and trait-`impl` items are public with no modifier; inherent-`impl` items are not | Unit | P1 | FR-009-AC-4 | ✅ |
| TC-082 | Signature renders declared parameter types and return type, receiver as `self` | Unit | P1 | FR-009-AC-5 | ✅ |
| TC-083 | Reformatting and an in-parameter comment leave the signature byte-identical | Unit | P1 | FR-009-AC-6 | ✅ |
| TC-084 | Unannotated Python callable renders parameter names; no parameter list means no signature | Unit | P1 | FR-009-AC-7 | ✅ |
| TC-085 | A parameter-type change alters the signature but not the node id; a new private helper leaves existing records unchanged | Unit | P1 | FR-009-AC-8 | ✅ |
| TC-086 | Records without the new fields deserialize unchanged | Unit | P1 | FR-009-AC-9 | ✅ |
| TC-087 | Node identity separates object type from qualified name | Unit | P1 | FR-006-CON-1 | ✅ |
| TC-088 | The `reason` vocabulary matches the consumer contract value for value | Unit | P1 | FR-004-CON-1 | ✅ |
| TC-089 | A Rust trait method is a declaration parented by its trait | Unit | P1 | FR-001-AC-9 | ✅ |
| TC-090 | A TypeScript interface member is a declaration parented by its interface | Unit | P1 | FR-001-AC-9 | ✅ |
| TC-091 | Only a const holding a function expression is a callable declaration | Unit | P1 | FR-001-AC-10 | ✅ |
| TC-092 | Containment parent is the declaring type in Rust, TypeScript and Python | Unit | P1 | FR-003-AC-6 | ✅ |
| TC-093 | A comment above a declaration's attributes belongs to that declaration | Unit | P1 | FR-005-AC-9 | ✅ |
| TC-094 | An inner doc comment stays with its enclosing scope | Unit | P1 | FR-005-AC-9 | ✅ |
| TC-095 | Criterion-level identifiers are harvested whole, and not also as their prefix | Unit | P1 | FR-005-AC-10 | ✅ |
| TC-096 | A suffixed criterion identifier is not a mention | Unit | P1 | FR-005-AC-5 | ✅ |
| TC-097 | A Python base class yields an `extends` edge | Integration | P1 | FR-008-AC-7 | ✅ |
| TC-098 | An `export` wrapper does not break leading-comment attribution | Unit | P1 | FR-005-AC-9 | ✅ |
| TC-099 | A trailing comment does not attach to the next declaration | Unit | P1 | FR-005-AC-9 | ✅ |
| TC-100 | An abstract method is a declaration parented by its class | Unit | P1 | FR-001-AC-9 | ✅ |
| TC-101 | The documented invocation writes canonical records on stdout | Integration | P1 | FR-010-AC-1 | ✅ |
| TC-102 | Extraction order does not depend on the filesystem's order | Integration | P1 | FR-010-AC-2 | ✅ |
| TC-103 | An unsupported language is skipped without failing or diagnosing | Integration | P1 | FR-010-AC-3 | ✅ |
| TC-104 | A malformed invocation writes no records to stdout | Integration | P1 | FR-010-AC-4 | ✅ |
| TC-105 | A symbolic link is not followed | Integration | P1 | FR-010-AC-5 | ✅ |
| TC-106 | A self-edge is never emitted | Unit | P1 | FR-006-AC-7 | ✅ |
| TC-107 | One declaration shape resolves at one tier in every language | Integration | P1 | FR-008-AC-13 | ✅ |
| TC-108 | Supported population retains all dimensioned quality observations | Integration | P0 | StR-003-VC-1, US-004-EX-1 | ✅ |
| TC-109 | Non-measured populations cannot become measured zeros | Property | P0 | StR-003-VC-2, US-004-EX-2 | ✅ |
| TC-110 | Measured observation pins the complete producer tuple | Unit | P0 | StR-003-VC-3 | ✅ |
| TC-111 | Raw output, schema, and MeasurementPlan validate together | Integration | P0 | StR-003-VC-4 | ✅ |
| TC-112 | Valid measured observation passes schema version 1 | Unit | P0 | FR-011-AC-1 | ✅ |
| TC-113 | Measured record covers all four dimensions and overall | Property | P0 | FR-011-AC-2 | ✅ |
| TC-114 | Population-state conditional schema rejects result lies | Property | P0 | FR-011-AC-3 | ✅ |
| TC-115 | Missing or malformed provenance and plan identities fail | Property | P0 | FR-011-AC-4 | ✅ |
| TC-116 | Raw scorer output path and digest are strict | Property | P0 | FR-011-AC-5 | ✅ |
| TC-117 | Closed vocabulary and unknown-field rejection | Property | P0 | FR-011-AC-6 | ✅ |
| TC-118 | Real supported corpus emits a governed observation | Integration | P0 | FR-012-AC-1 | ✅ |
| TC-119 | Producer emits exact census and four-dimension results | Integration | P0 | FR-012-AC-2 | ✅ |
| TC-120 | Zero wrong edges and incomplete recall remain separate | Integration | P0 | FR-012-AC-3, US-004-EX-3 | ✅ |
| TC-121 | False-positive heuristic edge fails regardless of recall | Integration | P0 | FR-012-AC-4 | ✅ |
| TC-122 | Empty, unreadable, and unsupported populations fail non-measured | Property | P0 | FR-012-AC-5 | ✅ |
| TC-123 | Missing revision or configuration emits no observation | Property | P0 | FR-012-AC-6 | ✅ |
| TC-124 | Pinned repetitions produce identical record and raw digest | Integration | P0 | FR-012-AC-7 | ✅ |
| TC-125 | Invalid plan or observation is rejected before Quoin intake | Integration | P0 | FR-012-AC-8 | ✅ |
| TC-126 | Two pinned repetitions emit byte-identical records | Integration | P0 | NFR-005-AC-1 | ✅ |
| TC-127 | Filesystem order permutation changes no canonical byte | Property | P0 | NFR-005-AC-2 | ✅ |
| TC-128 | Record contains no host-specific or run-time identity | Static | P0 | NFR-005-AC-3 | ✅ |
| TC-129 | Active MeasurementPlan governs population and decision rule | Integration | P0 | MP-001 | ✅ |
| TC-130 | Observation schema is engine-agnostic JSON data | Static | P0 | FR-011-CON-1 | ✅ |
| TC-131 | Observation validation needs no extractor library | Integration | P0 | FR-011-CON-2 | ✅ |
| TC-132 | Recall has no threshold that relaxes precision | Static | P0 | FR-012-CON-1 | ✅ |
| TC-133 | Measurement producer reads local filesystem inputs only | Static | P0 | FR-012-CON-2 | ✅ |
| TC-134 | Measurement producer has no network dependency or request | Static | P0 | FR-012-CON-3 | ✅ |
| TC-135 | Consumer parses Rust and walks the tree with no fact-model dependency | Integration | P1 | FR-013-AC-1 | ✅ |
| TC-136 | Syntax-error Rust file returns a named diagnostic, not an empty result | Integration | P1 | FR-013-AC-3 | ✅ |
| TC-137 | Every compiled-in language loads its grammar | Unit | P2 | FR-013-AC-4 | ✅ |
| TC-138 | A `rust`-only build has exactly one `Language` variant | Unit | P1 | FR-013-CON-3 | ✅ |
| TC-139 | Every `ParseError` variant's accessors return a file and a line | Unit | P1 | FR-013-AC-3 | ✅ |
| TC-140 | Syntax-error message renders the file and line | Unit | P2 | FR-013-AC-3 | ✅ |
| TC-141 | `ParsedFile` borrows the source rather than cloning it | Unit | P1 | FR-013-AC-2 | ✅ |
| TC-142 | Identical input yields a byte-identical tree, checked two ways | Unit | P1 | FR-013-AC-5 | ✅ |
| TC-143 | Parsing never panics on empty, punctuation-only or NUL-byte input | Unit | P1 | FR-013-AC-6 | ✅ |
| TC-144 | `ParsedFile` is `Send` (compiled static assertion) | Static | P1 | FR-013-AC-7 | ✅ |
| TC-145 | `ParsedFile` is not `Sync` (compiled static assertion) | Static | P1 | FR-013-AC-7 | ✅ |
| TC-146 | Consumer parses Python and walks the tree | Integration | P2 | FR-013-AC-1 | ✅ |
| TC-147 | Consumer parses TypeScript and TSX and walks the tree | Integration | P2 | FR-013-AC-1 | ✅ |
| TC-148 | Syntax-error Python file returns a named diagnostic too | Integration | P2 | FR-013-AC-3 | ✅ |
| TC-149 | Two independent parses of identical bytes render identical trees | Integration | P1 | FR-013-AC-5 | ✅ |
| TC-150 | The parsed source outlives the call, across a function boundary | Integration | P1 | FR-013-AC-2 | ✅ |

---

## Option Permutation Matrix

| Test Case | Population State | Results Present | Process Result | Expected Behavior |
|---|---|---|---|---|
| TC-118 | measured | yes | zero when precision invariant holds | Observation validates |
| TC-122 | empty | no | non-zero | Explicit empty state |
| TC-122 | unreadable | no | non-zero | Explicit unreadable state |
| TC-122 | unsupported | no | non-zero | Explicit unsupported state |
| TC-114 | measured | no | validation failure | Missing results rejected |
| TC-114 | non-measured | yes | validation failure | Misleading results rejected |

## Constraint Boundary Tests

| Constraint | Boundary Type | Test Value | Test Case | Expected |
|---|---|---|---|---|
| Supported measured population | Min | 1 supported readable file | TC-112 | Pass |
| Supported measured population | Below Min | 0 supported files | TC-114 | Fail as measured |
| Confusion-matrix dimension coverage | Min | 4 required dimensions plus overall | TC-113 | Pass |
| Confusion-matrix dimension coverage | Below Min | Any required dimension absent | TC-113 | Fail producer contract |
| Precision invariant | Max | 0 false-positive heuristic edges | TC-120 | Pass decision |
| Precision invariant | Above Max | 1 false-positive heuristic edge | TC-121 | Fail decision |
| Recall | Min | 0 recovered of positive expected count | TC-120 | Report without relaxing precision |
| Recall | Max | Expected count fully recovered | TC-120 | Report without changing precision rule |

## Integration Test Matrix

### Cross-Project Integrations

| Integration ID | Purpose | Target Project | Type | Test Cases | Status |
|---|---|---|---|---|---|
| INT-001 | Read pinned population and truth records | quire-corpus | service | TC-118, TC-119, TC-124 | ✅ |
| INT-002 | Validate MP-001 and authored observation contract | quire-rs | service | TC-111, TC-125, TC-129 | ✅ |
| INT-003 | Retain and render engine-agnostic observations | quoin | service | TC-111, TC-125 | ✅ |

### Integration Test Details

| Test Case | Integration | Scenario | Input | Expected | Priority |
|---|---|---|---|---|---|
| TC-118 | INT-001 | Complete supported population | Pinned corpus and truth | Schema-valid measured observation | P0 |
| TC-119 | INT-001 | Dimensioned complete census | Truth across four dimensions | Exact matrices and censuses | P0 |
| TC-124 | INT-001 | Repeated pinned collection | Same corpus, reversed creation order | Identical bytes and raw digest | P0 |
| TC-129 | INT-002 | Measurement governance | Active MP-001 | Quire-valid plan and enforced decision rule | P0 |
| TC-125 | INT-002, INT-003 | Invalid plan or record | Malformed governed input | Rejected before evidence intake | P0 |
| TC-111 | INT-002, INT-003 | End-to-end retained observation | Valid measured record and raw output | Quoin retains inspectable dimensions | P0 |

## Edge Cases

| ID | Description | Related Req | Test Case | Risk if Untested |
|---|---|---|---|---|
| EC-001 | Empty declared corpus | FR-012 | TC-122 | Absence appears as perfect precision and recall |
| EC-002 | Supported file cannot be read | FR-012 | TC-122 | Partial population is scored as complete |
| EC-003 | Population has only unsupported files | FR-012 | TC-122 | Unsupported work appears measured |
| EC-004 | One wrong heuristic edge with full recall | FR-012 | TC-121 | Recall masks a precision invariant failure |
| EC-005 | Zero wrong edges with incomplete recall | FR-012 | TC-120 | Precision and recall collapse into one score |
| EC-006 | Missing grammar or corpus revision | FR-011 | TC-115 | Incomparable observations appear equivalent |
| EC-007 | Absolute raw-output path | FR-011 | TC-116 | Host identity leaks into evidence and breaks determinism |

---

## Coverage Notes

- **Status legend**: 🚧 Planned (matrix row authored, test not yet written),
  ✅ Complete (test written, tagged and green). Historical `⬜` references in
  prose describe the repository's prior convention; new rows use `🚧`.
- **The matrix is gated, not asserted.** `make coverage` runs
  `quire coverage` over this repository and fails when a row claims a passing
  status it cannot back, when an unbacked row's declared verification method
  does not explain the absence, or when a declaration matches nothing and
  reports a confident zero. The last case is why the gate exists: this table
  claimed ✅ on all 102 of its rows for as long as its Test Case Summary minted
  no `TC-NNN` id at all, and 95.8% line coverage said nothing about it
  (agent-ix/quire-code-rs#8).
- **Rows no symbol can back.** TC-061 and the four `Inspection` constraints
  (FR-001-CON-1, FR-001-CON-2, FR-005-CON-1, FR-009-CON-2) carry a verification
  method that mints no test symbol, so they are reported unbacked by
  construction rather than as an overclaim. TC-061 is an `Analysis`: the
  dependency closure holds no network client (TC-059) and extraction reads no
  ambient state (TC-060), so an offline run and an online run are the same run
  — there is no second condition to compare against. FR-013-AC-8,
  FR-013-CON-1, FR-013-CON-2 and FR-013-CON-4 are the same shape: whether
  `Language`/`ParseError` are `#[non_exhaustive]`, whether ADR-002 binds this
  crate, whether the crate depends on the fact model, and where classification
  belongs are all read from the crate's own source and `Cargo.toml` rather
  than exercised by a test symbol.
- Benchmark-verified criteria (TC-062..TC-065, TC-070) report measurements on
  every run and gate on threshold only in the performance lane, so that
  shared-runner variance does not make ordinary CI flaky.
- TC-032 extracts this repository's own test suite and serves as the dogfooding
  check that the tracking tags this matrix claims are really present in the
  tests.
- The benchmark rows (TC-062..TC-065, TC-070) run in the performance lane —
  `make bench`, which is `cargo test --test perf_lane -- --ignored` — not in the
  ordinary gate, since benchmark timings on a shared runner would make the
  normal suite flaky. The lane is an `#[ignore]`d test rather than a
  `harness = false` bench so that each row binds a real symbol; a bare `fn` in a
  custom harness carries the tag and backs nothing. Their
  corpus is generated deterministically rather than committed: a 500,000-line
  fixture would dominate the repository, would still not resemble a real
  codebase, and could not be regenerated at a different size when the budget
  changes.
- Measured on an M-series laptop at 5,000 files / 705,000 lines: full extraction
  1.2 s (budget 60 s), single-file p95 0.2 ms (budget 50 ms), peak RSS 0.58 GB
  (budget 2.0 GB), fixpoint converging in 1 iteration (bound 10), and recall
  1.00 with zero wrong edges.
