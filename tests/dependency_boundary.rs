//! Dependency-boundary gate (PLAT-849), copied from the pattern
//! `filament-ide-rs` established in `crates/filament-code-extraction` and
//! `quire-rs` copied for PLAT-843
//! (`crates/quire-rust-extraction/tests/dependency_boundary.rs`).
//!
//! `quire-code-parse` (`crates/quire-code-parse`) already exists in this
//! workspace as the shared tree-sitter parse layer (PLAT-841); PLAT-849
//! routes this package's own fact/edge pipeline through it instead of
//! letting the two exist side by side. This is the mechanism that stops a
//! second tree-sitter parser reappearing here — the exact duplication
//! PLAT-849 exists to end. It was deliberately **observed red** before being
//! trusted: a temporary direct `tree-sitter` dependency was added to this
//! package, confirmed to fail this test, then reverted (see the PR body).
//!
//! **Observed red a second time** after being strengthened to scope by
//! `workspace_members` instead of one hardcoded package name (PR #27 review
//! B1): a hardcoded `if name != "quire-code-rs"` check only re-proves the
//! one case it already covered — it does not evidence that a *third*
//! workspace member added later, naming `tree-sitter` directly, would be
//! caught. Confirmed by temporarily adding a third workspace member with a
//! direct `tree-sitter = { workspace = true }` dependency, observing this
//! test fail, then reverting (recorded in the PR body rather than repeated
//! here).

// TC-176: no workspace member other than `quire-code-parse` names
// `tree-sitter` (or a `tree-sitter-*` grammar) as a direct dependency, and
// `quire-code-parse` itself does — the mechanism PLAT-849 exists to add.
// No owning acceptance criterion yet (PR #27 review N3); tagged here so this
// mechanism is visible to the Test Matrix rather than invisible to deletion.
#[test]
fn tree_sitter_is_reachable_only_through_the_quire_code_parse_pin() {
    let metadata = cargo_metadata();
    let packages = metadata["packages"].as_array().expect("packages");
    // The in-scope set is *every current workspace member*, read from
    // `workspace_members` rather than hardcoded by name (review B1): a gate
    // that names "quire-code-rs" literally checks nothing about a third
    // workspace member added later — the same defect class as a criterion
    // too weak to fail. `workspace_members` is a list of package ids;
    // cross-reference against each package's own `id` rather than parsing
    // the id string, so this holds across cargo's id format.
    let workspace_members: std::collections::HashSet<&str> = metadata["workspace_members"]
        .as_array()
        .expect("workspace_members")
        .iter()
        .map(|id| id.as_str().expect("workspace member id is a string"))
        .collect();
    assert!(
        !workspace_members.is_empty(),
        "expected at least quire-code-rs and quire-code-parse as workspace members, got none \
         — an empty set would make the loop below vacuously pass"
    );

    for package in packages {
        let id = package["id"].as_str().unwrap_or_default();
        // Only this workspace's own crates are in scope; third-party
        // packages are entitled to their own dependencies.
        if !workspace_members.contains(id) {
            continue;
        }
        let name = package["name"].as_str().unwrap_or_default();
        // `quire-code-parse` itself is the one workspace member allowed to
        // name `tree-sitter` — it is the pin every other workspace member is
        // required to reach it through.
        if name == "quire-code-parse" {
            continue;
        }
        for dependency in package["dependencies"].as_array().expect("dependencies") {
            let dep_name = dependency["name"].as_str().unwrap_or_default();
            assert!(
                !dep_name.starts_with("tree-sitter"),
                "crate `{name}` depends on `{dep_name}` directly; the tree-sitter \
                 boundary must live behind the quire-code-parse dependency \
                 (see src/parse.rs, src/lang.rs)"
            );
        }
    }

    // ...and the pin really is the path by which it is reachable at all.
    let quire_code_parse = packages
        .iter()
        .find(|package| package["name"].as_str() == Some("quire-code-parse"))
        .expect("quire-code-parse is a workspace member");
    assert!(
        quire_code_parse["dependencies"]
            .as_array()
            .expect("dependencies")
            .iter()
            .any(|dependency| dependency["name"]
                .as_str()
                .unwrap_or_default()
                .starts_with("tree-sitter")),
        "the pin is what owns the tree-sitter dependency"
    );
}

fn cargo_metadata() -> serde_json::Value {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let output = std::process::Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--manifest-path",
            manifest,
        ])
        .output()
        .expect("cargo metadata must run");
    assert!(output.status.success(), "cargo metadata failed");
    serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON")
}
