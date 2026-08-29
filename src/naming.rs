//! Org-qualified symbol identity.
//!
//! Implements [FR-002](../spec/functional/FR-002-symbol-identity.md):
//! `{org}/{repo}/{relative/path}::{Parent}::{symbol}`, with the `{org}/` prefix
//! so identically named repositories in different orgs cannot collide.
//!
//! Anonymous declarations take an *ordinal*-derived segment, not a line-derived
//! one. That distinction came out of the spec review (SR-002 FND-001): a
//! line-derived segment makes the name positional, so an edit elsewhere in the
//! file changes the name, which changes the SHA-256 record id, which breaks the
//! stability FR-006 promises.

/// Normalize a consumer-supplied path to the forward-slash form names use
/// (FR-002-CON-1). Backslashes become slashes and redundant separators
/// collapse, so a Windows-style path and its POSIX spelling name one file.
pub fn normalize_path(path: &str) -> String {
    let unified = path.replace('\\', "/");
    let mut out = String::with_capacity(unified.len());
    let mut prev_slash = false;
    for ch in unified.chars() {
        if ch == '/' {
            if prev_slash {
                continue;
            }
            prev_slash = true;
        } else {
            prev_slash = false;
        }
        out.push(ch);
    }
    out.trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

/// The qualified name of a file itself: org, repo and path, no `::` segment
/// (FR-002-AC-3).
pub fn file_name(org: &str, repo: &str, path: &str) -> String {
    format!("{org}/{repo}/{}", normalize_path(path))
}

/// Append a `::`-separated segment to a qualified name.
pub fn child_name(parent_qualified: &str, segment: &str) -> String {
    format!("{parent_qualified}::{segment}")
}

/// The segment used for an anonymous declaration: its kind plus its zero-based
/// ordinal among anonymous declarations of the same kind under the same parent
/// (FR-002-AC-5). Stable under edits that shift lines.
pub fn anonymous_segment(kind: &str, ordinal: usize) -> String {
    format!("<{kind}#{ordinal}>")
}

/// The `ix://` reference for a qualified name (FR-002-CON-2). The qualified
/// name already begins `{org}/{repo}/…`, so the reference is `ix://` plus the
/// name — three segments at minimum, satisfying last-segment resolution.
pub fn ix_ref(qualified_name: &str) -> String {
    format!("ix://{qualified_name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TC-013, FR-002-AC-6, FR-002-CON-1: Windows-style paths normalize to forward slashes.
    #[test]
    fn windows_paths_normalize_to_forward_slashes() {
        assert_eq!(normalize_path("src\\core\\lib.rs"), "src/core/lib.rs");
        assert_eq!(normalize_path("src//core///lib.rs"), "src/core/lib.rs");
        assert_eq!(normalize_path("./src/lib.rs"), "src/lib.rs");
        assert_eq!(
            file_name("agent-ix", "quire-code-rs", "src\\lib.rs"),
            "agent-ix/quire-code-rs/src/lib.rs"
        );
    }

    // TC-010, FR-002-AC-3: a code_file name carries no `::` segment.
    #[test]
    fn file_names_carry_no_segment_separator() {
        let name = file_name("agent-ix", "quire-code-rs", "src/lib.rs");
        assert_eq!(name, "agent-ix/quire-code-rs/src/lib.rs");
        assert!(!name.contains("::"));
    }

    // TC-008, FR-002-AC-1: a free function's qualified name shape.
    #[test]
    fn free_function_name_has_the_documented_shape() {
        let file = file_name("agent-ix", "quire-code-rs", "src/lib.rs");
        assert_eq!(
            child_name(&file, "extract"),
            "agent-ix/quire-code-rs/src/lib.rs::extract"
        );
    }

    // TC-011, FR-002-AC-4: the same repo under two orgs yields disjoint names.
    #[test]
    fn org_prefix_keeps_same_named_repos_disjoint() {
        let a = file_name("agent-ix", "widgets", "src/lib.rs");
        let b = file_name("other-org", "widgets", "src/lib.rs");
        assert_ne!(a, b);
    }

    // TC-012, FR-002-AC-5: anonymous segments are ordinal-derived, so they
    // survive an edit that shifts the declaration's lines.
    #[test]
    fn anonymous_segments_are_ordinal_not_positional() {
        assert_eq!(anonymous_segment("closure", 0), "<closure#0>");
        assert_ne!(
            anonymous_segment("closure", 0),
            anonymous_segment("closure", 1)
        );
    }

    // TC-014, FR-002-AC-7, FR-002-CON-2: every ix:// reference has at least three segments.
    #[test]
    fn ix_refs_carry_at_least_three_segments() {
        let name = file_name("agent-ix", "quire-code-rs", "src/lib.rs");
        let reference = ix_ref(&name);
        assert!(reference.starts_with("ix://"));
        let segments: Vec<_> = reference
            .trim_start_matches("ix://")
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        assert!(segments.len() >= 3, "got {segments:?}");
    }
}
