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

#[test]
fn tree_sitter_is_reachable_only_through_the_quire_code_parse_pin() {
    let metadata = cargo_metadata();
    let packages = metadata["packages"].as_array().expect("packages");

    for package in packages {
        let name = package["name"].as_str().unwrap_or_default();
        // Only this workspace's own root package is in scope; third-party
        // packages (including `quire-code-parse` itself, which is entitled
        // to its own dependencies) are not.
        if name != "quire-code-rs" {
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
