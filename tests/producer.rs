//! The `extract_tree` binary's contract (FR-010).
//!
//! `agent-ix/quire-corpus` pins this invocation as
//! `producer_contract.version: 1`, so every governed observation recorded
//! against this extractor was measured through it. That is why the contract is
//! tested here rather than left to the corpus: by the time the corpus notices,
//! the evidence has already been recorded through the changed shape.

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_extract_tree");

/// A scratch tree under the target directory, removed when the test ends.
struct Tree(PathBuf);

impl Tree {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch tree");
        Tree(root)
    }

    fn write(&self, relative: &str, content: &str) -> &Self {
        let path = self.0.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(path, content).expect("write");
        self
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .output()
        .expect("the binary runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// TC-101, FR-010-AC-1: the documented invocation writes canonical records.
#[test]
fn the_documented_invocation_writes_records() {
    let tree = Tree::new("tc101");
    tree.write("src/lib.rs", "pub struct Store;\n");

    let (ok, stdout, stderr) = run(&[
        "--org",
        "agent-ix",
        "--repo",
        "demo",
        tree.path().to_str().expect("utf-8 path"),
    ]);
    assert!(ok, "stderr: {stderr}");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is JSON");
    let names: Vec<&str> = value["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .map(|n| n["name"].as_str().expect("a name"))
        .collect();
    assert!(
        names.contains(&"agent-ix/demo/src/lib.rs::Store"),
        "got {names:?}"
    );
}

// TC-102, FR-010-AC-2: the order files are read in is not the order the
// filesystem hands them back.
#[test]
fn extraction_order_does_not_depend_on_the_filesystem() {
    let tree = Tree::new("tc102");
    for name in ["zeta", "alpha", "mid"] {
        tree.write(&format!("src/{name}.rs"), &format!("pub struct {name};\n"));
    }
    let args = [
        "--org",
        "agent-ix",
        "--repo",
        "demo",
        tree.path().to_str().expect("utf-8 path"),
    ];
    let (ok, first, _) = run(&args);
    assert!(ok);
    let (_, second, _) = run(&args);
    assert_eq!(
        first, second,
        "two runs over one tree must serialize identically"
    );

    let value: serde_json::Value = serde_json::from_str(&first).expect("JSON");
    let files: Vec<&str> = value["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|n| n["object_type"] == "code_file")
        .map(|n| n["name"].as_str().expect("a name"))
        .collect();
    let mut sorted = files.clone();
    sorted.sort_unstable();
    assert_eq!(files, sorted, "file records come out in path order");
}

// TC-103, FR-010-AC-3: a file in an unsupported language is skipped, silently
// and successfully — a README is not an extraction failure.
#[test]
fn an_unsupported_language_is_skipped_without_failing() {
    let tree = Tree::new("tc103");
    tree.write("README.md", "# not source\n");
    tree.write("data.json", "{}\n");
    tree.write("src/lib.rs", "pub struct Store;\n");

    let (ok, stdout, stderr) = run(&[
        "--org",
        "agent-ix",
        "--repo",
        "demo",
        tree.path().to_str().expect("utf-8 path"),
    ]);
    assert!(ok, "stderr: {stderr}");
    assert!(
        !stdout.contains("README.md") && !stdout.contains("data.json"),
        "an unsupported file must not become a node"
    );
    assert!(
        stderr.is_empty(),
        "and must not become a diagnostic either: {stderr}"
    );
}

// TC-104, FR-010-AC-4: a malformed invocation fails before writing anything a
// caller could mistake for records.
#[test]
fn a_malformed_invocation_writes_no_records() {
    for args in [
        vec!["--org", "agent-ix"],
        vec!["--org", "agent-ix", "--repo", "demo"],
        vec!["--unknown-flag", "x"],
    ] {
        let (ok, stdout, stderr) = run(&args);
        assert!(!ok, "{args:?} should fail");
        assert!(
            stdout.trim().is_empty(),
            "{args:?} wrote {stdout:?} to stdout; a caller parsing that gets an \
             empty graph instead of an error"
        );
        assert!(!stderr.trim().is_empty(), "{args:?} explained nothing");
    }
}

// TC-105, FR-010-AC-5: a symbolic link is not followed. A link to an ancestor
// is the shape that makes a naive walk non-terminating.
#[cfg(unix)]
#[test]
fn a_symbolic_link_is_not_followed() {
    let tree = Tree::new("tc105");
    tree.write("src/lib.rs", "pub struct Store;\n");
    std::os::unix::fs::symlink(tree.path(), tree.path().join("src/loop")).expect("symlink");

    let (ok, stdout, stderr) = run(&[
        "--org",
        "agent-ix",
        "--repo",
        "demo",
        tree.path().to_str().expect("utf-8 path"),
    ]);
    assert!(ok, "stderr: {stderr}");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("JSON");
    let files: Vec<&str> = value["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|n| n["object_type"] == "code_file")
        .map(|n| n["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(
        files,
        vec!["agent-ix/demo/src/lib.rs"],
        "the link contributed a second copy of the tree"
    );
}
