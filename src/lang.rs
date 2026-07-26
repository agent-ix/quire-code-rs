//! Language selection and per-language node-kind configuration.
//!
//! Implements the configuration half of
//! [FR-001](../spec/functional/FR-001-structural-fact-model.md): languages are
//! added by describing their grammar's node kinds here, never by forking the
//! extraction engine (FR-001-CON-2). Everything downstream — naming, edges,
//! mentions, resolution — reads this table and stays language-agnostic.

use crate::facts::ObjectType;

/// A source language this library can extract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Language {
    Rust,
    TypeScript,
    Tsx,
    Python,
}

impl Language {
    /// The language's canonical lowercase name, as consumers pass it.
    pub fn as_str(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::TypeScript => "typescript",
            Language::Tsx => "tsx",
            Language::Python => "python",
        }
    }

    /// Parse a consumer-supplied language name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "rust" => Some(Language::Rust),
            "typescript" | "ts" => Some(Language::TypeScript),
            "tsx" => Some(Language::Tsx),
            "python" | "py" => Some(Language::Python),
            _ => None,
        }
    }

    /// Best-effort language selection from a path's extension.
    ///
    /// Consumers own language selection (SR-003 FND-003); this is a
    /// convenience for the common case, not a requirement.
    pub fn from_path(path: &str) -> Option<Self> {
        let ext = path.rsplit_once('.').map(|(_, e)| e)?;
        match ext {
            "rs" => Some(Language::Rust),
            "ts" | "mts" | "cts" => Some(Language::TypeScript),
            "tsx" => Some(Language::Tsx),
            "py" | "pyi" => Some(Language::Python),
            _ => None,
        }
    }

    /// The tree-sitter grammar for this language.
    pub(crate) fn grammar(self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Language::Python => tree_sitter_python::LANGUAGE.into(),
        }
    }

    /// The node-kind configuration driving structural extraction.
    pub(crate) fn config(self) -> &'static LanguageConfig {
        match self {
            Language::Rust => &RUST,
            Language::TypeScript | Language::Tsx => &TYPESCRIPT,
            Language::Python => &PYTHON,
        }
    }
}

/// One declaration form the grammar exposes, and how to treat it.
pub(crate) struct DeclKind {
    /// The grammar's node kind, e.g. `function_item`.
    pub node: &'static str,
    /// The canonical object type this declaration becomes.
    pub object_type: ObjectType,
    /// The `kind` discriminator carried in node data (FR-001).
    pub kind: &'static str,
    /// Whether this declaration contributes a `::` segment to the qualified
    /// names of declarations nested inside it (FR-002).
    pub names_children: bool,
}

/// Per-language configuration. Adding a language means adding one of these.
pub(crate) struct LanguageConfig {
    pub decls: &'static [DeclKind],
    /// Node kinds holding a declaration's name, tried in order.
    pub name_fields: &'static [&'static str],
    /// Grammar node kinds that are comments.
    pub comment_nodes: &'static [&'static str],
    /// Grammar node kinds that are attributes carrying harvestable text.
    pub attribute_nodes: &'static [&'static str],
    /// Grammar node kinds that are string literals — never harvested (FR-005).
    pub string_nodes: &'static [&'static str],
    /// Grammar node kinds holding an import statement.
    pub import_nodes: &'static [&'static str],
    /// File extensions tried when resolving an extensionless specifier.
    pub import_extensions: &'static [&'static str],
    /// Index file stems tried when a specifier names a directory.
    pub import_index_stems: &'static [&'static str],
}

static RUST: LanguageConfig = LanguageConfig {
    decls: &[
        DeclKind {
            node: "mod_item",
            object_type: ObjectType::Module,
            kind: "module",
            names_children: true,
        },
        DeclKind {
            node: "function_item",
            object_type: ObjectType::Function,
            kind: "function",
            names_children: true,
        },
        DeclKind {
            node: "struct_item",
            object_type: ObjectType::Type,
            kind: "struct",
            names_children: true,
        },
        DeclKind {
            node: "enum_item",
            object_type: ObjectType::Type,
            kind: "enum",
            names_children: true,
        },
        DeclKind {
            node: "trait_item",
            object_type: ObjectType::Type,
            kind: "trait",
            names_children: true,
        },
        DeclKind {
            node: "type_item",
            object_type: ObjectType::Type,
            kind: "type_alias",
            names_children: false,
        },
        DeclKind {
            node: "union_item",
            object_type: ObjectType::Type,
            kind: "union",
            names_children: true,
        },
        // `impl_item` is deliberately absent as a fact: FR-002 names a method
        // by its implementing *type*, not by the impl block, and the trait an
        // impl implements is recorded as an edge rather than in the name.
    ],
    name_fields: &["name", "type"],
    comment_nodes: &["line_comment", "block_comment", "doc_comment"],
    attribute_nodes: &["attribute_item", "inner_attribute_item"],
    string_nodes: &["string_literal", "raw_string_literal", "char_literal"],
    import_nodes: &["use_declaration"],
    import_extensions: &["rs"],
    import_index_stems: &["mod"],
};

static TYPESCRIPT: LanguageConfig = LanguageConfig {
    decls: &[
        DeclKind {
            node: "internal_module",
            object_type: ObjectType::Module,
            kind: "namespace",
            names_children: true,
        },
        DeclKind {
            node: "module",
            object_type: ObjectType::Module,
            kind: "namespace",
            names_children: true,
        },
        DeclKind {
            node: "function_declaration",
            object_type: ObjectType::Function,
            kind: "function",
            names_children: true,
        },
        DeclKind {
            node: "generator_function_declaration",
            object_type: ObjectType::Function,
            kind: "generator_function",
            names_children: true,
        },
        DeclKind {
            node: "method_definition",
            object_type: ObjectType::Function,
            kind: "method",
            names_children: true,
        },
        DeclKind {
            node: "class_declaration",
            object_type: ObjectType::Type,
            kind: "class",
            names_children: true,
        },
        DeclKind {
            node: "abstract_class_declaration",
            object_type: ObjectType::Type,
            kind: "class",
            names_children: true,
        },
        DeclKind {
            node: "interface_declaration",
            object_type: ObjectType::Type,
            kind: "interface",
            names_children: true,
        },
        DeclKind {
            node: "enum_declaration",
            object_type: ObjectType::Type,
            kind: "enum",
            names_children: true,
        },
        DeclKind {
            node: "type_alias_declaration",
            object_type: ObjectType::Type,
            kind: "type_alias",
            names_children: false,
        },
    ],
    name_fields: &["name"],
    comment_nodes: &["comment"],
    attribute_nodes: &["decorator"],
    string_nodes: &["string", "template_string"],
    import_nodes: &["import_statement", "export_statement"],
    import_extensions: &["ts", "tsx", "d.ts", "js", "jsx"],
    import_index_stems: &["index"],
};

static PYTHON: LanguageConfig = LanguageConfig {
    decls: &[
        DeclKind {
            node: "function_definition",
            object_type: ObjectType::Function,
            kind: "function",
            names_children: true,
        },
        DeclKind {
            node: "class_definition",
            object_type: ObjectType::Type,
            kind: "class",
            names_children: true,
        },
    ],
    name_fields: &["name"],
    comment_nodes: &["comment"],
    attribute_nodes: &["decorator"],
    string_nodes: &["string"],
    import_nodes: &["import_statement", "import_from_statement"],
    import_extensions: &["py", "pyi"],
    import_index_stems: &["__init__"],
};

impl LanguageConfig {
    /// The declaration configuration for a grammar node kind, if it is one.
    pub(crate) fn decl_for(&self, node_kind: &str) -> Option<&DeclKind> {
        self.decls.iter().find(|d| d.node == node_kind)
    }

    pub(crate) fn is_comment(&self, node_kind: &str) -> bool {
        self.comment_nodes.contains(&node_kind)
    }

    pub(crate) fn is_attribute(&self, node_kind: &str) -> bool {
        self.attribute_nodes.contains(&node_kind)
    }

    pub(crate) fn is_string(&self, node_kind: &str) -> bool {
        self.string_nodes.contains(&node_kind)
    }

    pub(crate) fn is_import(&self, node_kind: &str) -> bool {
        self.import_nodes.contains(&node_kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_names_round_trip() {
        for lang in [
            Language::Rust,
            Language::TypeScript,
            Language::Tsx,
            Language::Python,
        ] {
            assert_eq!(Language::from_name(lang.as_str()), Some(lang));
        }
    }

    #[test]
    fn language_from_path_reads_the_extension() {
        assert_eq!(Language::from_path("src/lib.rs"), Some(Language::Rust));
        assert_eq!(Language::from_path("ui/app.tsx"), Some(Language::Tsx));
        assert_eq!(Language::from_path("tool.py"), Some(Language::Python));
        assert_eq!(Language::from_path("README.md"), None);
        assert_eq!(Language::from_path("no-extension"), None);
    }

    #[test]
    fn every_grammar_loads() {
        for lang in [
            Language::Rust,
            Language::TypeScript,
            Language::Tsx,
            Language::Python,
        ] {
            let mut parser = tree_sitter::Parser::new();
            parser
                .set_language(&lang.grammar())
                .expect("grammar loads for the configured language");
        }
    }
}
