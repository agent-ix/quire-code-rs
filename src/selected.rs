//! Selected-scope extraction against a typed manifest (FR-014).
//!
//! The manifest, not the supplied files, defines what was selected. `Full` is
//! produced here by walking the manifest, never accepted from a caller: every
//! selected module must be supplied, carry the manifest's content identity and
//! parse without diagnostics, and every selected function must be declared.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::extract::{extract_with, ExtractionResult};
use crate::facts::{Diagnostic, ObjectType};
use crate::naming::{file_name, normalize_path};
use crate::parse::{parse_file, ParsedFile, SourceFile};

/// The content identity of a source file: lowercase-hex SHA-256 of its bytes.
/// Computed by the extractor for every supplied file and compared against the
/// manifest's recorded identity.
pub fn content_identity(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

/// Identity of a file within a repository: org, repo and normalized path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FileKey {
    pub org: String,
    pub repo: String,
    pub path: String,
}

impl FileKey {
    fn of(file: &SourceFile) -> Self {
        FileKey {
            org: file.org.clone(),
            repo: file.repo.clone(),
            path: file.normalized_path(),
        }
    }
}

/// One selected production module and, optionally, selected functions in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedModule {
    pub org: String,
    pub repo: String,
    /// Repository-relative path of the module's file.
    pub path: String,
    /// [`content_identity`] of the file the selection was made against.
    pub identity: String,
    /// Selected function names, as written after the module in a qualified
    /// name (`put`, `Store::put`). Empty selects the module alone.
    pub functions: Vec<String>,
}

impl SelectedModule {
    pub fn new(
        org: impl Into<String>,
        repo: impl Into<String>,
        path: impl Into<String>,
        identity: impl Into<String>,
    ) -> Self {
        SelectedModule {
            org: org.into(),
            repo: repo.into(),
            path: normalize_path(&path.into()),
            identity: identity.into(),
            functions: Vec::new(),
        }
    }

    pub fn with_functions<I, S>(mut self, functions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.functions = functions.into_iter().map(Into::into).collect();
        self
    }

    fn key(&self) -> FileKey {
        FileKey {
            org: self.org.clone(),
            repo: self.repo.clone(),
            path: self.path.clone(),
        }
    }
}

/// The authoritative selection: a selection identity and the selected modules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionManifest {
    pub id: String,
    pub modules: Vec<SelectedModule>,
}

impl SelectionManifest {
    pub fn new(id: impl Into<String>, modules: Vec<SelectedModule>) -> Self {
        SelectionManifest {
            id: id.into(),
            modules,
        }
    }
}

/// Why a selected module or function is not accounted for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaccountedCause {
    /// No supplied file has the module's key.
    NotSupplied,
    /// The supplied file's content identity differs from the manifest's.
    IdentityMismatch { expected: String, actual: String },
    /// The supplied file parsed with diagnostics.
    Unparsable { diagnostics: Vec<Diagnostic> },
    /// The module is accounted for but does not declare the selected function.
    FunctionAbsent,
}

/// A selected module (`function` is `None`) or function not accounted for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnaccountedEntry {
    pub org: String,
    pub repo: String,
    pub path: String,
    pub function: Option<String>,
    pub cause: UnaccountedCause,
}

/// Why no scope could be established at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnavailableReason {
    /// The manifest selects nothing.
    EmptySelection,
    /// The manifest names one file more than once.
    DuplicateModule {
        org: String,
        repo: String,
        path: String,
    },
    /// No selected module could be accounted for.
    NoModuleAccounted { unaccounted: Vec<UnaccountedEntry> },
}

/// How much of the selection the extraction covered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractionScope {
    /// Every selected module and function is accounted for.
    Full,
    /// Some entries are accounted for; `missing` lists the rest.
    Partial {
        missing: Vec<UnaccountedEntry>,
    },
    Unavailable {
        reason: UnavailableReason,
    },
}

/// The outcome of [`extract_selected`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractionOutcome {
    /// The manifest's selection identity, preserved for binding.
    pub selection: String,
    pub scope: ExtractionScope,
    /// Supplied files outside the manifest (or repeating a supplied key).
    /// They contribute no records.
    pub extra: Vec<FileKey>,
    /// Records for the accounted selected modules only.
    pub result: ExtractionResult,
}

/// Extract the selected modules and functions of `manifest` from `files`.
///
/// Only files that match the manifest by key and content identity and parse
/// without diagnostics contribute records. Pure and deterministic: the outcome
/// does not depend on the order of `files`.
pub fn extract_selected(manifest: &SelectionManifest, files: &[SourceFile]) -> ExtractionOutcome {
    let outcome = |scope, extra, result| ExtractionOutcome {
        selection: manifest.id.clone(),
        scope,
        extra,
        result,
    };
    let empty_result = || extract_with(&[], &mut |f| Arc::new(parse_file(f)));

    let mut selected: BTreeMap<FileKey, &SelectedModule> = BTreeMap::new();
    for module in &manifest.modules {
        let key = module.key();
        if selected.insert(key.clone(), module).is_some() {
            let FileKey { org, repo, path } = key;
            return outcome(
                ExtractionScope::Unavailable {
                    reason: UnavailableReason::DuplicateModule { org, repo, path },
                },
                Vec::new(),
                empty_result(),
            );
        }
    }

    // Order candidates by key then content so duplicates resolve the same way
    // whatever order the caller supplied them in.
    let mut ordered: Vec<(FileKey, String, &SourceFile)> = files
        .iter()
        .map(|f| (FileKey::of(f), content_identity(&f.content), f))
        .collect();
    ordered.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    let mut candidates: BTreeMap<FileKey, (String, &SourceFile)> = BTreeMap::new();
    let mut extra: Vec<FileKey> = Vec::new();
    for (key, identity, file) in ordered {
        if selected.contains_key(&key) && !candidates.contains_key(&key) {
            candidates.insert(key, (identity, file));
        } else {
            extra.push(key);
        }
    }

    if selected.is_empty() {
        return outcome(
            ExtractionScope::Unavailable {
                reason: UnavailableReason::EmptySelection,
            },
            extra,
            empty_result(),
        );
    }

    let mut unaccounted: Vec<UnaccountedEntry> = Vec::new();
    let mut accepted: Vec<(&SourceFile, Arc<ParsedFile>)> = Vec::new();
    for (key, module) in &selected {
        let entry = |function: Option<String>, cause| UnaccountedEntry {
            org: key.org.clone(),
            repo: key.repo.clone(),
            path: key.path.clone(),
            function,
            cause,
        };
        let Some((identity, file)) = candidates.get(key) else {
            unaccounted.push(entry(None, UnaccountedCause::NotSupplied));
            continue;
        };
        if *identity != module.identity {
            unaccounted.push(entry(
                None,
                UnaccountedCause::IdentityMismatch {
                    expected: module.identity.clone(),
                    actual: identity.clone(),
                },
            ));
            continue;
        }
        let parsed = Arc::new(parse_file(file));
        if !parsed.diagnostics.is_empty() {
            unaccounted.push(entry(
                None,
                UnaccountedCause::Unparsable {
                    diagnostics: parsed.diagnostics.clone(),
                },
            ));
            continue;
        }
        let declared: BTreeSet<&str> = parsed
            .facts
            .iter()
            .filter(|f| f.object_type == ObjectType::Function)
            .map(|f| f.qualified_name.as_str())
            .collect();
        let module_name = file_name(&key.org, &key.repo, &key.path);
        for function in &module.functions {
            if !declared.contains(format!("{module_name}::{function}").as_str()) {
                unaccounted.push(entry(
                    Some(function.clone()),
                    UnaccountedCause::FunctionAbsent,
                ));
            }
        }
        accepted.push((file, parsed));
    }

    if accepted.is_empty() {
        return outcome(
            ExtractionScope::Unavailable {
                reason: UnavailableReason::NoModuleAccounted { unaccounted },
            },
            extra,
            empty_result(),
        );
    }

    let batch: Vec<SourceFile> = accepted.iter().map(|(f, _)| (*f).clone()).collect();
    let parses: BTreeMap<FileKey, Arc<ParsedFile>> = accepted
        .iter()
        .map(|(f, p)| (FileKey::of(f), Arc::clone(p)))
        .collect();
    let result = extract_with(&batch, &mut |f| match parses.get(&FileKey::of(f)) {
        Some(parsed) => Arc::clone(parsed),
        None => Arc::new(parse_file(f)),
    });

    let scope = if unaccounted.is_empty() {
        ExtractionScope::Full
    } else {
        ExtractionScope::Partial {
            missing: unaccounted,
        }
    };
    outcome(scope, extra, result)
}
