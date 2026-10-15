---
id: SR-11081
title: "spec review of FR-014 selected-scope extraction (PR #41)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-code-rs@17cfb4e6a9e7655a1b76162b9555345982a7f573; spec/functional/FR-014-selected-scope-extraction.md, spec/functional/index.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-014
    type: reviews
---

## Summary

Ticket: QSL-775. Spec review with EARS analysis of FR-014. FR-014 agrees with
ticket contract items 1 to 3. The manifest is authoritative. `Full` requires
every manifest entry to be accounted for and to parse clean. Extra and
mismatched files are typed dispositions and are never included. An extra file
does not demote `Full`, which matches item 3: `Full` is defined over manifest
entries. "Selection naming an absent module" gives `Unavailable` for a sole
module. With other modules accounted for it gives `Partial`, which matches the
ticket's "one selected file missing" vector. The spec has no contradiction.
`quire validate` reports two grammar warnings at line 52.

## Verdict

**PASS.** The spec has no contradiction and no failed criterion. One low finding
is recorded.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | low | The extra-file Behavior bullet joins two SHALLs in one statement and uses an agentless passive ("shall be listed"). `quire validate` reports `ears:non-singular` and `quality:agentless-passive`. The wording is weak but the meaning is correct. | spec/functional/FR-014-selected-scope-extraction.md:52 |
