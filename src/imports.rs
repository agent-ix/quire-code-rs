//! Path resolution for import specifiers.
//!
//! Implements the `imports` half of
//! [FR-003](../spec/functional/FR-003-structural-edges.md). Resolution consults
//! only the batch's path set — never the filesystem, never a package manifest
//! (FR-003-CON-1).
//!
//! Bare and package-absolute specifiers resolve to nothing *and produce no
//! diagnostic*. That came out of the spec review (SR-001 FND-003): diagnosing
//! every third-party import would bury the unresolvable *relative* imports,
//! which are the ones that actually signal an incomplete batch.

use std::collections::BTreeSet;

use crate::lang::Language;

/// Whether a specifier is relative — the only kind this library resolves.
pub fn is_relative(specifier: &str, language: Language) -> bool {
    match language {
        Language::Rust => {
            // Rust `use` paths are module paths, not file paths. `self::`,
            // `super::` and `crate::` are the in-crate forms.
            specifier.starts_with("self::")
                || specifier.starts_with("super::")
                || specifier.starts_with("crate::")
        }
        Language::TypeScript | Language::Tsx => {
            specifier.starts_with("./") || specifier.starts_with("../")
        }
        Language::Python => specifier.starts_with('.'),
    }
}

/// Resolve a relative specifier against the importing file's path, returning a
/// path present in `batch_paths`, or `None`.
pub fn resolve(
    specifier: &str,
    importer_path: &str,
    language: Language,
    batch_paths: &BTreeSet<String>,
) -> Option<String> {
    if !is_relative(specifier, language) {
        return None;
    }
    let config = language.config();
    let dir = parent_dir(importer_path);

    let candidate_stem = match language {
        Language::Rust => rust_module_path(specifier, importer_path)?,
        Language::Python => python_module_path(specifier, importer_path)?,
        Language::TypeScript | Language::Tsx => join_relative(&dir, specifier)?,
    };

    // Exact path, then each configured extension, then each index stem.
    if batch_paths.contains(&candidate_stem) {
        return Some(candidate_stem);
    }
    for ext in config.import_extensions {
        let with_ext = format!("{candidate_stem}.{ext}");
        if batch_paths.contains(&with_ext) {
            return Some(with_ext);
        }
    }
    for stem in config.import_index_stems {
        for ext in config.import_extensions {
            let indexed = format!("{candidate_stem}/{stem}.{ext}");
            if batch_paths.contains(&indexed) {
                return Some(indexed);
            }
        }
    }
    None
}

fn parent_dir(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => String::new(),
    }
}

/// Join a `./` or `../` specifier onto a directory, collapsing `.` and `..`.
fn join_relative(dir: &str, specifier: &str) -> Option<String> {
    let mut segments: Vec<&str> = if dir.is_empty() {
        Vec::new()
    } else {
        dir.split('/').collect()
    };
    for part in specifier.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                segments.pop()?;
            }
            other => segments.push(other),
        }
    }
    Some(segments.join("/"))
}

/// Rust `use` paths name modules, not files. `crate::a::b` maps to `a/b`
/// beneath the crate root; `self::x` and `super::x` map beside the importer.
fn rust_module_path(specifier: &str, importer_path: &str) -> Option<String> {
    let dir = parent_dir(importer_path);
    // Drop the trailing item name: `crate::store::Store` imports the `store`
    // module, and the item within it is not a file.
    let segments: Vec<&str> = specifier.split("::").collect();
    let (head, rest) = segments.split_first()?;
    let module_segments: Vec<&str> = rest
        .iter()
        .copied()
        .filter(|s| !s.is_empty() && *s != "*")
        .collect();
    // Everything but a trailing type-like segment is a module path.
    let module_path: Vec<&str> = module_segments
        .iter()
        .copied()
        .take_while(|s| {
            s.chars()
                .next()
                .is_some_and(|c| c.is_lowercase() || c == '_')
        })
        .collect();
    if module_path.is_empty() {
        return None;
    }
    let base = match *head {
        "crate" => crate_root(importer_path),
        "self" => dir,
        "super" => parent_dir(&dir),
        _ => return None,
    };
    if base.is_empty() {
        Some(module_path.join("/"))
    } else {
        Some(format!("{base}/{}", module_path.join("/")))
    }
}

/// The crate root directory of a Rust file — the `src/` it lives beneath.
fn crate_root(importer_path: &str) -> String {
    match importer_path.split_once("src/") {
        Some((prefix, _)) => format!("{prefix}src"),
        None => parent_dir(importer_path),
    }
}

/// Python relative imports: leading dots count levels up from the importer.
fn python_module_path(specifier: &str, importer_path: &str) -> Option<String> {
    let dots = specifier.chars().take_while(|c| *c == '.').count();
    if dots == 0 {
        return None;
    }
    let mut dir = parent_dir(importer_path);
    for _ in 1..dots {
        dir = parent_dir(&dir);
    }
    let tail = specifier.trim_start_matches('.').replace('.', "/");
    if tail.is_empty() {
        Some(dir)
    } else if dir.is_empty() {
        Some(tail)
    } else {
        Some(format!("{dir}/{tail}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(entries: &[&str]) -> BTreeSet<String> {
        entries.iter().map(|s| s.to_string()).collect()
    }

    // TC-016, FR-003-AC-2: a relative import naming a batch file resolves.
    #[test]
    fn relative_typescript_import_resolves_to_a_batch_path() {
        let batch = paths(&["src/store.ts", "src/app.ts"]);
        assert_eq!(
            resolve("./store", "src/app.ts", Language::TypeScript, &batch),
            Some("src/store.ts".to_string())
        );
    }

    // TC-018, FR-003-AC-4: extensionless specifiers use the language's
    // extension and index conventions.
    #[test]
    fn extensionless_specifiers_try_extensions_then_index_stems() {
        let batch = paths(&["src/widgets/index.ts", "src/app.ts"]);
        assert_eq!(
            resolve("./widgets", "src/app.ts", Language::TypeScript, &batch),
            Some("src/widgets/index.ts".to_string())
        );

        let py = paths(&["pkg/sub/__init__.py", "pkg/main.py"]);
        assert_eq!(
            resolve(".sub", "pkg/main.py", Language::Python, &py),
            Some("pkg/sub/__init__.py".to_string())
        );
    }

    #[test]
    fn parent_relative_specifiers_walk_up() {
        let batch = paths(&["src/shared/util.ts", "src/views/app.ts"]);
        assert_eq!(
            resolve(
                "../shared/util",
                "src/views/app.ts",
                Language::TypeScript,
                &batch
            ),
            Some("src/shared/util.ts".to_string())
        );
    }

    #[test]
    fn rust_module_paths_resolve_beneath_the_crate_root() {
        let batch = paths(&["src/lib.rs", "src/store.rs", "src/net/client.rs"]);
        assert_eq!(
            resolve("crate::store::Store", "src/lib.rs", Language::Rust, &batch),
            Some("src/store.rs".to_string())
        );
        assert_eq!(
            resolve(
                "crate::net::client::Client",
                "src/lib.rs",
                Language::Rust,
                &batch
            ),
            Some("src/net/client.rs".to_string())
        );
    }

    #[test]
    fn rust_mod_rs_index_stem_resolves() {
        let batch = paths(&["src/lib.rs", "src/net/mod.rs"]);
        assert_eq!(
            resolve("crate::net::Client", "src/lib.rs", Language::Rust, &batch),
            Some("src/net/mod.rs".to_string())
        );
    }

    // TC-017, FR-003-AC-3: bare specifiers are not relative, so they resolve
    // to nothing and (in the caller) produce no diagnostic.
    #[test]
    fn bare_specifiers_are_not_relative() {
        assert!(!is_relative("react", Language::TypeScript));
        assert!(!is_relative("std::collections::BTreeMap", Language::Rust));
        assert!(!is_relative("os.path", Language::Python));

        assert!(is_relative("./store", Language::TypeScript));
        assert!(is_relative("crate::store", Language::Rust));
        assert!(is_relative(".sibling", Language::Python));
    }

    #[test]
    fn an_unresolvable_relative_specifier_returns_none() {
        let batch = paths(&["src/app.ts"]);
        assert_eq!(
            resolve("./missing", "src/app.ts", Language::TypeScript, &batch),
            None
        );
    }
}
