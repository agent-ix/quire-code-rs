//! The structural fact model.
//!
//! Implements the fact half of
//! [FR-001](../spec/functional/FR-001-structural-fact-model.md) and the
//! diagnostics of [FR-007](../spec/functional/FR-007-parse-error-isolation.md).

use serde::{Deserialize, Serialize};

/// The canonical object type of a structural fact (FR-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    CodeFile,
    Module,
    Function,
    Type,
}

impl ObjectType {
    /// The wire name, matching the node types consumers register.
    pub fn as_str(self) -> &'static str {
        match self {
            ObjectType::CodeFile => "code_file",
            ObjectType::Module => "code_module",
            ObjectType::Function => "code_function",
            ObjectType::Type => "code_type",
        }
    }
}

/// How widely a declaration is visible, as its own language expresses it
/// (FR-009).
///
/// Three values rather than a boolean because the middle tier is real in every
/// language this library parses — Rust's `pub(crate)`, TypeScript's
/// `protected`, Python's single-underscore convention — and a consumer tiering
/// an export-set change needs to tell "visible to my dependents" from "visible
/// within this unit only".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Visible outside the defining module: Rust `pub`, TS `export`, a Python
    /// name with no underscore prefix.
    Public,
    /// Visible within the defining unit only: Rust `pub(crate)`/`pub(super)`,
    /// TS `protected`, a Python `_name`.
    Crate,
    /// Not visible outside its declaration: a bare Rust item, an unexported TS
    /// declaration, a Python `__name`.
    Private,
}

impl Visibility {
    /// The wire name.
    pub fn as_str(self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Crate => "crate",
            Visibility::Private => "private",
        }
    }

    /// Whether a consumer outside the defining module can refer to this
    /// declaration — the question export-set change tiering actually asks.
    pub fn is_exported(self) -> bool {
        matches!(self, Visibility::Public)
    }
}

impl Default for Visibility {
    /// Records written before FR-009 carried no visibility. Reading them back
    /// as `public` keeps the pre-FR-009 behavior — every declaration counts as
    /// an export — so an old record never silently *loses* a dependent
    /// (FR-009-AC-9).
    fn default() -> Self {
        Visibility::Public
    }
}

/// A one-based, inclusive line span (FR-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LineSpan {
    pub start: u32,
    pub end: u32,
}

/// One structural declaration recovered from a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeFact {
    /// Canonical object type.
    pub object_type: ObjectType,
    /// The concrete declaration form, e.g. `struct`, `class`, `method`.
    pub kind: &'static str,
    /// Fully qualified name per FR-002.
    pub qualified_name: String,
    /// The declaration's own simple name, as written.
    pub simple_name: String,
    /// Repository-relative, forward-slash separated path of the owning file.
    pub path: String,
    /// One-based inclusive line span.
    pub span: LineSpan,
    /// Qualified name of the immediately enclosing fact, if any. Drives
    /// `contains` edges (FR-003) without a re-parse.
    pub parent: Option<String>,
    /// How widely this declaration is visible (FR-009).
    #[serde(default)]
    pub visibility: Visibility,
    /// Normalized parameter/return summary for a callable, absent for every
    /// declaration whose grammar exposes no parameter list (FR-009).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// Severity of an extraction diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

/// A per-file extraction diagnostic (FR-007). Never a panic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub severity: Severity,
}

impl Diagnostic {
    pub(crate) fn parse_error(
        path: &str,
        message: impl Into<String>,
        line: u32,
        column: u32,
    ) -> Self {
        Diagnostic {
            code: "parse_error".to_string(),
            message: message.into(),
            path: path.to_string(),
            line: Some(line),
            column: Some(column),
            severity: Severity::Error,
        }
    }

    pub(crate) fn file_error(path: &str, code: &str, message: impl Into<String>) -> Self {
        Diagnostic {
            code: code.to_string(),
            message: message.into(),
            path: path.to_string(),
            line: None,
            column: None,
            severity: Severity::Error,
        }
    }

    pub(crate) fn unresolved_import(path: &str, specifier: &str, line: u32) -> Self {
        Diagnostic {
            code: "unresolved_import".to_string(),
            message: format!("relative import `{specifier}` names no file in the batch"),
            path: path.to_string(),
            line: Some(line),
            column: None,
            severity: Severity::Warning,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_type_wire_names_match_the_consumer_contract() {
        assert_eq!(ObjectType::CodeFile.as_str(), "code_file");
        assert_eq!(ObjectType::Module.as_str(), "code_module");
        assert_eq!(ObjectType::Function.as_str(), "code_function");
        assert_eq!(ObjectType::Type.as_str(), "code_type");
    }
}
