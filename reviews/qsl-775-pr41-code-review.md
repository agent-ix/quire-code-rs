---
id: SR-11079
title: "code review of selected-scope extraction (PR #41)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-code-rs@17cfb4e6a9e7655a1b76162b9555345982a7f573; src/selected.rs, src/lib.rs, tests/selected_extraction.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-code-rs/FR-014
    type: reviews
---

## Summary

Ticket: QSL-775. Code review with the rust-review lane of `extract_selected`.
The manifest is authoritative. `Full` comes only from the extractor's walk of the
manifest. A module counts only when a supplied file has its key, the computed
SHA-256 identity matches, and the parse has zero diagnostics. Parse diagnostics
are all `Severity::Error`, so a warning-only parse cannot occur. The only
warning, `unresolved_import`, is raised at batch level. Extra and mismatched
files contribute no records. The outcome is deterministic in file order. The
change has no panic surface and no vendored copy. All seven tests assert typed
fields and are not vacuous. The content identity binds the selected module to
the supplied bytes. It is the identity the ticket's contract requires, not a pin.

## Verdict

**PASS.** No finding produces a false `Full` or another wrong scope, a panic, or
a failed acceptance criterion. Two medium findings and one low finding are
recorded and do not block.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | medium | Two supplied files for one selected key, one matching and one not: the candidate is the lower SHA-256 hex, so the scope is `Full` (the mismatching copy goes to `extra`) or `Partial` with `IdentityMismatch` (the matching copy goes to `extra`) depending on hash order. The result is conservative, not a false `Full`, but a matching copy should win. | src/selected.rs:205-218 |
| FND-002 | medium | A function selection returns the whole module's records, not records narrowed to the selected functions. FR-014-AC-1 defines records per file, so this does not violate it. The IR-740 adapter must filter rows using the manifest it holds. | src/selected.rs:290-299 |
| FND-003 | low | `SelectedModule` has pub fields and derives `Deserialize`, so a deserialized path bypasses `normalize_path`. The result is conservative: `NotSupplied` plus `extra`, and duplicates are not detected. | src/selected.rs:46-56 |
