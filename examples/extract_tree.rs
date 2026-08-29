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
    let mut unreadable = Vec::new();
    if let Err(err) = collect(&root, &org, &repo, &mut files, &mut unreadable) {
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

    // A file that could not be read is named and the run fails, after the
    // records are written. Skipping it silently would let a partial tree be
    // scored as a whole one, which is the difference between a measured zero
    // and an absence.
    if !unreadable.is_empty() {
        for (path, why) in &unreadable {
            eprintln!("unreadable: {path}: {why}");
        }
        eprintln!("{} file(s) could not be read", unreadable.len());
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Walk `root`, adding every file whose extension names a supported language.
///
/// A file in an unsupported language is skipped rather than reported: the tree
/// is somebody's repository, and a README is not an extraction failure. A file
/// that *is* a supported language and cannot be read lands in `unreadable`, so
/// the caller can fail rather than score a partial tree.
///
/// Iterative rather than recursive, and symlinked directories are not followed.
/// A repository is somebody else's tree: a deep one overflows the stack and a
/// symlink cycle never terminates, and neither should take out a measurement
/// run.
fn collect(
    root: &Path,
    org: &str,
    repo: &str,
    out: &mut Vec<SourceFile>,
    unreadable: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
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
            // `symlink_metadata` does not follow the link, so a directory
            // pointing at an ancestor is skipped rather than walked forever.
            let meta =
                std::fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                stack.push(path);
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
            match std::fs::read_to_string(&path) {
                Ok(content) => out.push(SourceFile::new(org, repo, relative, language, content)),
                Err(err) => unreadable.push((relative, err.to_string())),
            }
        }
    }
    Ok(())
}
