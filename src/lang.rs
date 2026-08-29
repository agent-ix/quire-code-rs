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
    /// Node kinds the declaration's `value` must have for it to count.
    ///
    /// Empty means unconditional, which is every ordinary declaration. It is
    /// non-empty for the one form whose node kind does not say what it declares:
    /// a TypeScript `variable_declarator` is a constant, a component or a
    /// handler depending only on what is on the right of the `=`, and only the
    /// callable ones are declarations of a callable (#12).
    pub value_kinds: &'static [&'static str],
}

/// How a language spells visibility (FR-009-CON-2). Classification stays in
/// this table so a new language is added by describing its grammar, never by
/// branching the extraction engine on `Language`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisibilityStyle {
    /// A `visibility_modifier` child whose text is `pub` or `pub(…)`.
    RustModifier,
    /// An enclosing `export_statement`, plus per-member accessibility
    /// modifiers inside a class body.
    TypeScriptExport,
    /// No keyword: the underscore-prefix naming convention.
    PythonUnderscore,
}

/// Per-language configuration. Adding a language means adding one of these.
pub(crate) struct LanguageConfig {
    /// How this language expresses visibility (FR-009).
    pub visibility_style: VisibilityStyle,
    /// Grammar node kind holding an explicit visibility/accessibility modifier,
    /// if the language has one.
    pub visibility_nodes: &'static [&'static str],
    /// Grammar node kinds that declare a trait-like body whose associated items
    /// inherit the declaration's own visibility rather than carrying their own
    /// (FR-009-AC-4).
    pub visibility_inheriting_nodes: &'static [&'static str],
    /// Grammar node kind of a callable's parameter list, for signature
    /// rendering (FR-009).
    pub parameter_list_nodes: &'static [&'static str],
    /// Field names holding a declaration's return type, tried in order.
    pub return_type_fields: &'static [&'static str],
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
    /// Node kinds declaring a local binding (`let`, `const`, assignment).
    pub binding_nodes: &'static [&'static str],
    /// Node kinds for a call expression.
    pub call_nodes: &'static [&'static str],
    /// Node kind for a member/field access used as a call receiver.
    pub member_nodes: &'static [&'static str],
    /// Node kinds for a callable parameter carrying a declared type.
    pub parameter_nodes: &'static [&'static str],
    /// Node kinds declaring a field on a type.
    pub field_nodes: &'static [&'static str],
    /// Nodes on a type declaration that carry its supertypes.
    ///
    /// TypeScript spells them in a heritage clause that says `extends` or
    /// `implements`; Python spells them as a bare argument list, where every
    /// entry is an `extends`. Kept in the table rather than branched on in the
    /// walker, per FR-001-CON-2.
    pub superclass_nodes: &'static [&'static str],
}

static RUST: LanguageConfig = LanguageConfig {
    visibility_style: VisibilityStyle::RustModifier,
    visibility_nodes: &["visibility_modifier"],
    visibility_inheriting_nodes: &["trait_item"],
    parameter_list_nodes: &["parameters"],
    return_type_fields: &["return_type"],
    decls: &[
        DeclKind {
            node: "mod_item",
            object_type: ObjectType::Module,
            kind: "module",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "function_item",
            object_type: ObjectType::Function,
            kind: "function",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "struct_item",
            object_type: ObjectType::Type,
            kind: "struct",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "enum_item",
            object_type: ObjectType::Type,
            kind: "enum",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "trait_item",
            object_type: ObjectType::Type,
            kind: "trait",
            names_children: true,
            value_kinds: &[],
        },
        // A trait method with no body. Without this the declaration a
        // `implements_trait` edge points *into* does not exist, so every
        // traversal that reaches an interface stops there, and FR-009-AC-4's
        // rule for trait-item visibility has no node to apply to (#11).
        DeclKind {
            node: "function_signature_item",
            object_type: ObjectType::Function,
            kind: "function",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "type_item",
            object_type: ObjectType::Type,
            kind: "type_alias",
            names_children: false,
            value_kinds: &[],
        },
        DeclKind {
            node: "union_item",
            object_type: ObjectType::Type,
            kind: "union",
            names_children: true,
            value_kinds: &[],
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
    binding_nodes: &["let_declaration", "const_item", "static_item"],
    call_nodes: &["call_expression"],
    member_nodes: &["field_expression"],
    parameter_nodes: &["parameter", "self_parameter"],
    field_nodes: &["field_declaration"],
    superclass_nodes: &[],
};

static TYPESCRIPT: LanguageConfig = LanguageConfig {
    visibility_style: VisibilityStyle::TypeScriptExport,
    visibility_nodes: &["accessibility_modifier"],
    visibility_inheriting_nodes: &["interface_declaration"],
    parameter_list_nodes: &["formal_parameters"],
    return_type_fields: &["return_type"],
    decls: &[
        DeclKind {
            node: "internal_module",
            object_type: ObjectType::Module,
            kind: "namespace",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "module",
            object_type: ObjectType::Module,
            kind: "namespace",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "function_declaration",
            object_type: ObjectType::Function,
            kind: "function",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "generator_function_declaration",
            object_type: ObjectType::Function,
            kind: "generator_function",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "method_definition",
            object_type: ObjectType::Function,
            kind: "method",
            names_children: true,
            value_kinds: &[],
        },
        // An interface member — the same declaration as a trait method, and
        // absent for the same reason (#11).
        DeclKind {
            node: "method_signature",
            object_type: ObjectType::Function,
            kind: "method",
            names_children: true,
            value_kinds: &[],
        },
        // `const f = () => {}` and `const f = function () {}`. The dominant
        // declaration form in modern TypeScript — components, hooks, handlers,
        // most module-scope helpers — and the one whose node kind says nothing
        // about what it declares, hence the value guard (#12).
        DeclKind {
            node: "variable_declarator",
            object_type: ObjectType::Function,
            kind: "function",
            names_children: true,
            value_kinds: &["arrow_function", "function_expression", "function"],
        },
        DeclKind {
            node: "class_declaration",
            object_type: ObjectType::Type,
            kind: "class",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "abstract_class_declaration",
            object_type: ObjectType::Type,
            kind: "class",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "interface_declaration",
            object_type: ObjectType::Type,
            kind: "interface",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "enum_declaration",
            object_type: ObjectType::Type,
            kind: "enum",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "type_alias_declaration",
            object_type: ObjectType::Type,
            kind: "type_alias",
            names_children: false,
            value_kinds: &[],
        },
    ],
    name_fields: &["name"],
    comment_nodes: &["comment"],
    attribute_nodes: &["decorator"],
    string_nodes: &["string", "template_string"],
    import_nodes: &["import_statement", "export_statement"],
    import_extensions: &["ts", "tsx", "d.ts", "js", "jsx"],
    import_index_stems: &["index"],
    binding_nodes: &[
        "variable_declarator",
        "lexical_declaration",
        "public_field_definition",
    ],
    call_nodes: &["call_expression", "new_expression"],
    member_nodes: &["member_expression"],
    parameter_nodes: &["required_parameter", "optional_parameter"],
    field_nodes: &["property_signature", "public_field_definition"],
    superclass_nodes: &["class_heritage", "extends_type_clause"],
};

static PYTHON: LanguageConfig = LanguageConfig {
    visibility_style: VisibilityStyle::PythonUnderscore,
    visibility_nodes: &[],
    visibility_inheriting_nodes: &[],
    parameter_list_nodes: &["parameters"],
    return_type_fields: &["return_type"],
    decls: &[
        DeclKind {
            node: "function_definition",
            object_type: ObjectType::Function,
            kind: "function",
            names_children: true,
            value_kinds: &[],
        },
        DeclKind {
            node: "class_definition",
            object_type: ObjectType::Type,
            kind: "class",
            names_children: true,
            value_kinds: &[],
        },
    ],
    name_fields: &["name"],
    comment_nodes: &["comment"],
    attribute_nodes: &["decorator"],
    string_nodes: &["string"],
    import_nodes: &["import_statement", "import_from_statement"],
    import_extensions: &["py", "pyi"],
    import_index_stems: &["__init__"],
    binding_nodes: &["assignment"],
    call_nodes: &["call"],
    member_nodes: &["attribute"],
    parameter_nodes: &["typed_parameter", "identifier"],
    field_nodes: &["assignment"],
    superclass_nodes: &["argument_list"],
};

impl LanguageConfig {
    /// The declaration configuration for a grammar node kind, if it is one.
    pub(crate) fn decl_for(&self, node_kind: &str) -> Option<&DeclKind> {
        self.decls
            .iter()
            .find(|d| d.node == node_kind && d.value_kinds.is_empty())
    }

    /// The declaration this node is, including the guarded forms whose node
    /// kind alone does not say (`value_kinds`).
    pub(crate) fn decl_for_node(&self, node: tree_sitter::Node<'_>) -> Option<&DeclKind> {
        self.decls.iter().find(|d| {
            d.node == node.kind()
                && (d.value_kinds.is_empty()
                    || node
                        .child_by_field_name("value")
                        .is_some_and(|value| d.value_kinds.contains(&value.kind())))
        })
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
