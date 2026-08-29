//! Read a source tree and write its canonical records on stdout.
//!
//! The library is filesystem-free by design (NFR-002): it takes source *text*
//! and never opens a file. Something still has to read the tree before a corpus
//! can grade the output, and that something belongs outside the boundary — an
//! example, not the crate.
//!
//! ```text
//! cargo run --example extract_tree -- --org agent-ix --repo demo path/to/tree
//! ```
//!
//! The invocation and the stdout contract are pinned by
//! `agent-ix/quire-corpus` (`corpus.yaml`, `producer_contract.version: 1`), so
//! a change here is a change to what every recorded observation was measured
//! with.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use quire_code_rs::{extract, Language, SourceFile};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut org = None;
    let mut repo = None;
    let mut root: Option<PathBuf> = None;

    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--org" => org = rest.next().cloned(),
            "--repo" => repo = rest.next().cloned(),
            "-h" | "--help" => {
                eprintln!("usage: extract_tree --org <org> --repo <repo> <dir>");
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("unknown flag {other}");
                return ExitCode::FAILURE;
            }
            other => root = Some(PathBuf::from(other)),
        }
    }

    let (Some(org), Some(repo), Some(root)) = (org, repo, root) else {
        eprintln!("usage: extract_tree --org <org> --repo <repo> <dir>");
        return ExitCode::FAILURE;
    };

    let mut files = Vec::new();
    if let Err(err) = collect(&root, &root, &org, &repo, &mut files) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }

    // Sorted before extraction, so the output does not depend on the order the
    // filesystem happened to hand back (NFR-001).
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let result = extract(&files);
    let json = match serde_json::to_string_pretty(&result) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("serialization failed: {err}");
            return ExitCode::FAILURE;
        }
    };
    let mut stdout = std::io::stdout().lock();
    if writeln!(stdout, "{json}").is_err() {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Walk `dir`, adding every file whose extension names a supported language.
///
/// A file in an unsupported language is skipped rather than reported: the tree
/// is somebody's repository, and a README is not an extraction failure.
fn collect(
    root: &Path,
    dir: &Path,
    org: &str,
    repo: &str,
    out: &mut Vec<SourceFile>,
) -> Result<(), String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, org, repo, out)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        let Some(language) = Language::from_path(&relative) else {
            continue;
        };
        // Unreadable bytes are reported, never silently dropped: a skipped file
        // and an empty one are different populations.
        let content =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        out.push(SourceFile::new(org, repo, relative, language, content));
    }
    Ok(())
}
