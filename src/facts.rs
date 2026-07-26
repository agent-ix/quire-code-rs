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
