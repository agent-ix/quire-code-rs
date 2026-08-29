//! Specification mention harvesting.
//!
//! Implements [FR-005](../spec/functional/FR-005-spec-mention-harvesting.md):
//! `TC-NNN` tracking tags, `FR-NNN`/`NFR-NNN`/`Task-NNN` citations, and `ix://`
//! references, harvested from comments and attributes only — never from string
//! literals, and never from inside a longer token.
//!
//! Two rules here came out of the spec review. A mention edge targets the
//! identifier *exactly as written* (SR-002 FND-002), because this library
//! cannot see the artifact and must not invent a node for it. And a tracking
//! tag counts as a verification claim only inside a declaration the language
//! treats as a test (SR-001 FND-004); elsewhere it is an ordinary citation.

use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::parse::CommentText;

/// What kind of mention was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MentionKind {
    /// A `TC-NNN` tag inside a declaration recognized as a test.
    TrackingTag,
    /// An `FR-NNN`, `NFR-NNN` or `Task-NNN` citation — or a `TC-NNN` outside a
    /// test declaration.
    RequirementCitation,
    /// An `ix://` reference.
    ArtifactReference,
}

impl MentionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MentionKind::TrackingTag => "tracking_tag",
            MentionKind::RequirementCitation => "requirement_citation",
            MentionKind::ArtifactReference => "artifact_reference",
        }
    }
}

/// One harvested mention with its provenance.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Mention {
    /// The identifier exactly as written: `TC-001`, `FR-002`, `ix://org/repo/x`.
    pub identifier: String,
    pub kind: MentionKind,
    /// Qualified name of the enclosing code fact.
    pub source: String,
    pub file: String,
    pub line: u32,
}

fn identifier_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?:TC|NFR|FR|Task)-[0-9]{1,6}").expect("identifier pattern compiles")
    })
}

/// Whether a byte is part of an identifier token.
///
/// Boundaries are checked around the match rather than folded into the pattern:
/// a pattern that *consumes* its delimiters cannot match two identifiers
/// separated by a single space, because the first match eats the separator the
/// second one needs.
fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

/// Whether the match at `[start, end)` stands alone as a whole token
/// (FR-005-AC-5).
fn is_whole_token(text: &str, start: usize, end: usize) -> bool {
    let bytes = text.as_bytes();
    let before_ok = start == 0 || !is_token_byte(bytes[start - 1]);
    let after_ok = end >= bytes.len() || !is_token_byte(bytes[end]);
    before_ok && after_ok
}

fn ix_ref_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"ix://[A-Za-z0-9._\-]+(?:/[A-Za-z0-9._\-]+)+").expect("ix ref pattern compiles")
    })
}

/// Harvest every mention from one file's comments and attributes.
///
/// Returns mentions in stable order: by line, then by identifier.
pub fn harvest(comments: &[CommentText], file_path: &str) -> Vec<Mention> {
    let mut out = Vec::new();

    for comment in comments {
        // Harvest ix:// references first and record their spans, so the
        // identifier inside `ix://org/repo/FR-002` is not also reported as a
        // bare citation — one mention, not two.
        let mut ix_spans: Vec<(usize, usize)> = Vec::new();
        for found in ix_ref_pattern().find_iter(&comment.text) {
            // A reference at the end of a sentence picks up the sentence's
            // punctuation; trailing separators are never part of a ref.
            let trimmed = found
                .as_str()
                .trim_end_matches(['.', ',', ';', ':', ')', ']', '`']);
            if trimmed.is_empty() {
                continue;
            }
            ix_spans.push((found.start(), found.start() + trimmed.len()));
            out.push(Mention {
                identifier: trimmed.to_string(),
                kind: MentionKind::ArtifactReference,
                source: comment.owner.clone(),
                file: file_path.to_string(),
                line: comment.line,
            });
        }

        for found in identifier_pattern().find_iter(&comment.text) {
            if !is_whole_token(&comment.text, found.start(), found.end()) {
                continue;
            }
            if ix_spans
                .iter()
                .any(|(start, end)| found.start() >= *start && found.end() <= *end)
            {
                continue;
            }
            let identifier = found.as_str().to_string();
            let kind = if identifier.starts_with("TC-") && comment.owner_is_test {
                MentionKind::TrackingTag
            } else {
                MentionKind::RequirementCitation
            };
            out.push(Mention {
                identifier,
                kind,
                source: comment.owner.clone(),
                file: file_path.to_string(),
                line: comment.line,
            });
        }
    }

    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(text: &str, owner_is_test: bool) -> CommentText {
        CommentText {
            text: text.to_string(),
            line: 7,
            owner: "agent-ix/repo/src/lib.rs::thing".to_string(),
            owner_is_test,
        }
    }

    // TC-026, FR-005-AC-1, StR-002-VC-1: a tag in a test declaration is a tracking tag.
    #[test]
    fn a_tag_inside_a_test_is_a_tracking_tag() {
        let found = harvest(
            &[comment("// TC-001 — checks the thing", true)],
            "src/lib.rs",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].identifier, "TC-001");
        assert_eq!(found[0].kind, MentionKind::TrackingTag);
        assert_eq!(found[0].line, 7);
        assert_eq!(found[0].file, "src/lib.rs");
    }

    // TC-072, FR-005-AC-8: the same tag outside a test is a citation.
    #[test]
    fn a_tag_outside_a_test_is_only_a_citation() {
        let found = harvest(
            &[comment("// see TC-001 for the rationale", false)],
            "src/lib.rs",
        );
        assert_eq!(found[0].kind, MentionKind::RequirementCitation);
    }

    // TC-027, FR-005-AC-2: requirement citations are harvested.
    #[test]
    fn requirement_citations_are_harvested() {
        let found = harvest(
            &[comment(
                "//! Implements FR-002 and NFR-001; see Task-150.",
                false,
            )],
            "src/lib.rs",
        );
        let ids: Vec<_> = found.iter().map(|m| m.identifier.as_str()).collect();
        assert!(ids.contains(&"FR-002"), "got {ids:?}");
        assert!(ids.contains(&"NFR-001"), "got {ids:?}");
        assert!(ids.contains(&"Task-150"), "got {ids:?}");
    }

    // TC-028, FR-005-AC-3, StR-002-VC-2: ix:// references are harvested.
    #[test]
    fn ix_references_are_harvested() {
        let found = harvest(
            &[comment(
                "// see ix://agent-ix/filament-ide-rs/FR-072",
                false,
            )],
            "src/lib.rs",
        );
        let reference = found
            .iter()
            .find(|m| m.kind == MentionKind::ArtifactReference)
            .expect("an artifact reference");
        assert_eq!(reference.identifier, "ix://agent-ix/filament-ide-rs/FR-072");
    }

    #[test]
    fn a_reference_ending_a_sentence_drops_the_punctuation() {
        let found = harvest(
            &[comment("// see ix://agent-ix/demo/FR-003.", false)],
            "src/lib.rs",
        );
        assert_eq!(
            found.len(),
            1,
            "one mention, not a ref plus a bare citation"
        );
        assert_eq!(found[0].identifier, "ix://agent-ix/demo/FR-003");
        assert_eq!(found[0].kind, MentionKind::ArtifactReference);
    }

    #[test]
    fn an_identifier_inside_a_reference_is_not_also_a_bare_citation() {
        let found = harvest(
            &[comment(
                "// ix://agent-ix/demo/FR-003 and also FR-004",
                false,
            )],
            "src/lib.rs",
        );
        let ids: Vec<_> = found.iter().map(|m| m.identifier.as_str()).collect();
        assert_eq!(ids, vec!["FR-004", "ix://agent-ix/demo/FR-003"]);
    }

    // TC-030, FR-005-AC-5: an identifier inside a longer token is not a match.
    #[test]
    fn embedded_identifiers_are_not_mentions() {
        for text in [
            "// XTC-001 is not a mention",
            "// TC-001a is not a mention",
            "// prefixFR-002suffix",
            "// snake_TC-001",
        ] {
            let found = harvest(&[comment(text, true)], "src/lib.rs");
            assert!(found.is_empty(), "{text:?} matched: {found:?}");
        }
    }

    #[test]
    fn punctuation_delimited_identifiers_still_match() {
        for text in ["// (TC-001)", "// TC-001.", "//TC-001", "// [FR-002]"] {
            let found = harvest(&[comment(text, true)], "src/lib.rs");
            assert_eq!(found.len(), 1, "{text:?} -> {found:?}");
        }
    }

    // TC-031, FR-005-AC-6: a mention of something absent is still reported,
    // addressed by the identifier exactly as written.
    #[test]
    fn unresolvable_mentions_are_still_reported_as_written() {
        let found = harvest(
            &[comment("// TC-999999 does not exist", true)],
            "src/lib.rs",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].identifier, "TC-999999");
    }

    #[test]
    fn mentions_come_out_in_stable_order() {
        let comments = vec![
            CommentText {
                text: "// FR-003".into(),
                line: 20,
                owner: "o".into(),
                owner_is_test: false,
            },
            CommentText {
                text: "// FR-001 FR-002".into(),
                line: 5,
                owner: "o".into(),
                owner_is_test: false,
            },
        ];
        let found = harvest(&comments, "src/lib.rs");
        let ids: Vec<_> = found.iter().map(|m| m.identifier.as_str()).collect();
        assert_eq!(ids, vec!["FR-001", "FR-002", "FR-003"]);
    }

    #[test]
    fn repeated_identical_mentions_collapse() {
        let comments = vec![comment("// TC-001", true), comment("// TC-001", true)];
        let found = harvest(&comments, "src/lib.rs");
        assert_eq!(found.len(), 1);
    }
}
