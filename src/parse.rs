//! Per-file parsing into structural facts.
//!
//! Implements [FR-001](../spec/functional/FR-001-structural-fact-model.md) and
//! the error policy of
//! [FR-007](../spec/functional/FR-007-parse-error-isolation.md).
//!
//! The error policy keys on *identity recoverability*, not on error presence.
//! A declaration whose name the grammar could read is emitted even when its
//! body is mid-edit; only a construct the grammar could not resolve as a
//! declaration at all contributes nothing. Suppressing more than that —
//! the whole file, or a whole function because of a typo in its body —
//! contradicts the reason ADR-001 chose tree-sitter: extraction has to work on
//! trees that do not compile.

use std::collections::BTreeMap;

use tree_sitter::{Node, Parser, Tree};

use crate::facts::{CodeFact, Diagnostic, LineSpan, ObjectType, Visibility};
use crate::lang::{Language, LanguageConfig, VisibilityStyle};
use crate::naming::{anonymous_segment, child_name, file_name, normalize_path};
use crate::typeenv::{RawBinding, TypeSource, FILE_SCOPE};

/// A source file handed to the library. Content arrives in memory; this library
/// never reads a path from disk (NFR-002).
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub org: String,
    pub repo: String,
    /// Repository-relative path.
    pub path: String,
    pub language: Language,
    pub content: String,
}

impl SourceFile {
    pub fn new(
        org: impl Into<String>,
        repo: impl Into<String>,
        path: impl Into<String>,
        language: Language,
        content: impl Into<String>,
    ) -> Self {
        SourceFile {
            org: org.into(),
            repo: repo.into(),
            path: path.into(),
            language,
            content: content.into(),
        }
    }

    /// The normalized path used in names and evidence.
    pub fn normalized_path(&self) -> String {
        normalize_path(&self.path)
    }
}

/// What parsing one file recovered.
#[derive(Debug, Default)]
pub struct ParsedFile {
    pub facts: Vec<CodeFact>,
    pub diagnostics: Vec<Diagnostic>,
    /// Import specifiers with the line they appear on, for FR-003.
    pub imports: Vec<(String, u32)>,
    /// Comment and attribute text with its line, for FR-005.
    pub comments: Vec<CommentText>,
    /// Qualified name of the file's own `code_file` fact.
    pub file_qualified_name: String,
    /// Local bindings, for the FR-008 type environment.
    pub bindings: Vec<RawBinding>,
    /// Call sites awaiting resolution (FR-008).
    pub calls: Vec<CallSite>,
    /// Scope key -> the type enclosing it, for `self` / `this` receivers.
    pub enclosing_types: BTreeMap<String, String>,
    /// Declared return types: qualified callable name -> simple type name.
    pub return_types: BTreeMap<String, String>,
    /// Declared field types: (simple type name, field) -> simple type name.
    pub field_types: BTreeMap<(String, String), String>,
    /// Type relations: (child simple name, parent simple name, is_trait).
    pub type_relations: Vec<(String, String, bool)>,
}

/// One call site, recorded during the walk and resolved later (FR-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSite {
    /// Qualified name of the callable the site sits in.
    pub caller: String,
    /// Scope key the receiver is resolved in.
    pub scope: String,
    /// The receiver expression's leading identifier, if the call has one.
    pub receiver: Option<String>,
    /// The called function or method's simple name.
    pub callee: String,
    pub line: u32,
}

/// One comment or attribute, with enough context to attribute a mention.
#[derive(Debug, Clone)]
pub struct CommentText {
    pub text: String,
    pub line: u32,
    /// Qualified name of the innermost declaration enclosing this comment, or
    /// the file when it sits outside every declaration (FR-005).
    pub owner: String,
    /// Whether the owning declaration is recognized as a test (FR-005-AC-8).
    pub owner_is_test: bool,
}

/// Maximum accepted file size. Beyond this a diagnostic is returned rather than
/// an unbounded parse (FR-007).
const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;

/// Parse one file into facts, diagnostics, imports and comments.
///
/// Never panics: malformed input, oversized input and grammar failures all come
/// back as diagnostics (FR-007-CON-1).
pub fn parse_file(file: &SourceFile) -> ParsedFile {
    let path = file.normalized_path();
    let file_qualified_name = file_name(&file.org, &file.repo, &path);

    let mut out = ParsedFile {
        file_qualified_name: file_qualified_name.clone(),
        ..Default::default()
    };

    // The code_file fact is emitted for every accepted file, including one that
    // declares nothing and one whose root is an error node (FR-001-AC-4,
    // FR-007-AC-6).
    let line_count = file.content.lines().count().max(1) as u32;
    out.facts.push(CodeFact {
        object_type: ObjectType::CodeFile,
        kind: "file",
        qualified_name: file_qualified_name.clone(),
        simple_name: path.clone(),
        path: path.clone(),
        span: LineSpan {
            start: 1,
            end: line_count,
        },
        parent: None,
        // A file is the unit a consumer addresses from outside (FR-009).
        visibility: Visibility::Public,
        signature: None,
    });

    if file.content.len() > MAX_FILE_BYTES {
        out.diagnostics.push(Diagnostic::file_error(
            &path,
            "file_too_large",
            format!(
                "file is {} bytes, above the {MAX_FILE_BYTES}-byte extraction bound",
                file.content.len()
            ),
        ));
        return out;
    }

    let tree = match parse_tree(file) {
        Some(tree) => tree,
        None => {
            out.diagnostics.push(Diagnostic::file_error(
                &path,
                "parser_unavailable",
                "the grammar failed to produce a syntax tree",
            ));
            return out;
        }
    };

    let root = tree.root_node();
    let config = file.language.config();

    if root.has_error() {
        let (line, column) = first_error_position(root).unwrap_or((1, 0));
        out.diagnostics
            .push(Diagnostic::parse_error(&path, "syntax error", line, column));
    }

    // An error-node root means nothing in the file is trustworthy: emit the
    // code_file fact alone (FR-007-AC-6).
    if root.is_error() {
        return out;
    }

    let mut walker = Walker {
        source: file.content.as_bytes(),
        config,
        path: &path,
        facts: Vec::new(),
        comments: Vec::new(),
        imports: Vec::new(),
        anon_counters: BTreeMap::new(),
        bindings: Vec::new(),
        calls: Vec::new(),
        enclosing_types: BTreeMap::new(),
        return_types: BTreeMap::new(),
        field_types: BTreeMap::new(),
        type_relations: Vec::new(),
    };
    let root_ctx = Context {
        name_parent: file_qualified_name.clone(),
        contain_parent: file_qualified_name.clone(),
        scope: FILE_SCOPE.to_string(),
        owning_type: None,
        in_test: false,
    };
    walker.walk(root, &root_ctx);

    out.facts.extend(walker.facts);
    out.comments = walker.comments;
    out.imports = walker.imports;
    out.bindings = walker.bindings;
    out.calls = walker.calls;
    out.enclosing_types = walker.enclosing_types;
    out.return_types = walker.return_types;
    out.field_types = walker.field_types;
    out.type_relations = walker.type_relations;
    out
}

/// What the walker knows about where it currently is.
#[derive(Debug, Clone)]
struct Context {
    /// Nearest enclosing declaration contributing a name segment.
    name_parent: String,
    /// Nearest enclosing fact, for containment.
    contain_parent: String,
    /// Scope key for binding lookup: `""` at file level, `name@line` inside a
    /// callable.
    scope: String,
    /// Simple name of the type whose body we are inside, for `self`.
    owning_type: Option<String>,
    in_test: bool,
}

fn parse_tree(file: &SourceFile) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(&file.language.grammar()).ok()?;
    parser.parse(file.content.as_bytes(), None)
}

/// One-based line and zero-based column of the first error node, if any.
fn first_error_position(root: Node<'_>) -> Option<(u32, u32)> {
    let mut cursor = root.walk();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            let pos = node.start_position();
            return Some((pos.row as u32 + 1, pos.column as u32));
        }
        if node.has_error() {
            // Push children in reverse so the leftmost error is found first.
            let children: Vec<_> = node.children(&mut cursor).collect();
            for child in children.into_iter().rev() {
                stack.push(child);
            }
        }
    }
    None
}

struct Walker<'a> {
    source: &'a [u8],
    config: &'static LanguageConfig,
    path: &'a str,
    facts: Vec<CodeFact>,
    comments: Vec<CommentText>,
    imports: Vec<(String, u32)>,
    /// (parent qualified name, kind) -> next ordinal, for anonymous segments.
    anon_counters: BTreeMap<(String, &'static str), usize>,
    bindings: Vec<RawBinding>,
    calls: Vec<CallSite>,
    enclosing_types: BTreeMap<String, String>,
    return_types: BTreeMap<String, String>,
    field_types: BTreeMap<(String, String), String>,
    type_relations: Vec<(String, String, bool)>,
}

impl<'a> Walker<'a> {
    /// Walk `node` in `ctx`, emitting facts and collecting the inputs the
    /// FR-008 solver needs.
    fn walk(&mut self, node: Node<'a>, ctx: &Context) {
        let mut cursor = node.walk();
        let children: Vec<Node<'a>> = node.children(&mut cursor).collect();

        for child in children {
            let kind = child.kind();

            if self.config.is_comment(kind) || self.config.is_attribute(kind) {
                self.record_comment(child, &ctx.contain_parent, ctx.in_test);
                continue;
            }

            if self.config.is_import(kind) {
                self.record_import(child);
                // Fall through: an export statement can also wrap a declaration.
            }

            if self.config.is_string(kind) {
                // Never harvested (FR-005-AC-4), and holds no declarations.
                continue;
            }

            if self.config.binding_nodes.contains(&kind) {
                self.record_binding(child, ctx);
            }

            if self.config.call_nodes.contains(&kind) {
                self.record_call(child, ctx);
            }

            if self.config.field_nodes.contains(&kind) {
                self.record_field(child, ctx);
            }

            match self.config.decl_for(kind) {
                Some(decl) => {
                    let (segment, simple_name) = self.segment_for(child, decl, &ctx.name_parent);
                    let qualified = child_name(&ctx.name_parent, &segment);
                    let span = span_of(child);
                    let is_test = ctx.in_test || self.looks_like_test(child, &simple_name);

                    let visibility = self.visibility_of(child, &simple_name);
                    let signature = if decl.object_type == ObjectType::Function {
                        self.signature_of(child)
                    } else {
                        None
                    };

                    self.facts.push(CodeFact {
                        object_type: decl.object_type,
                        kind: decl.kind,
                        qualified_name: qualified.clone(),
                        simple_name: simple_name.clone(),
                        path: self.path.to_string(),
                        span,
                        parent: Some(ctx.contain_parent.to_string()),
                        visibility,
                        signature,
                    });

                    let mut next = Context {
                        name_parent: if decl.names_children {
                            qualified.clone()
                        } else {
                            ctx.name_parent.clone()
                        },
                        contain_parent: qualified.clone(),
                        scope: ctx.scope.clone(),
                        owning_type: ctx.owning_type.clone(),
                        in_test: is_test,
                    };

                    if decl.object_type == ObjectType::Function {
                        // A callable opens a scope. The key pairs its name with
                        // its start line, so two same-named callables in one
                        // file stay distinct.
                        next.scope = format!("{simple_name}@{}", span.start);
                        if let Some(owner) = &ctx.owning_type {
                            self.enclosing_types
                                .insert(next.scope.clone(), owner.clone());
                        }
                        self.record_signature(child, &qualified);
                        self.record_parameters(child, &next.scope);
                    }

                    if decl.object_type == ObjectType::Type {
                        next.owning_type = Some(simple_name.clone());
                        self.record_type_relations(child, &simple_name);
                    }

                    self.walk(child, &next);
                }
                None => {
                    // Not a declaration in this language's config. Rust `impl`
                    // blocks land here deliberately: FR-002 names a method by
                    // its implementing type, so the impl contributes a name
                    // segment without becoming a fact of its own.
                    match self.impl_type_segment(child) {
                        Some(impl_type) => {
                            self.record_impl_trait(child, &impl_type);
                            let next = Context {
                                name_parent: child_name(&ctx.name_parent, &impl_type),
                                contain_parent: ctx.contain_parent.clone(),
                                scope: ctx.scope.clone(),
                                owning_type: Some(impl_type),
                                in_test: ctx.in_test,
                            };
                            self.walk(child, &next);
                        }
                        None => self.walk(child, ctx),
                    }
                }
            }
        }
    }

    /// Record a local binding and where its type comes from (FR-008).
    fn record_binding(&mut self, node: Node<'a>, ctx: &Context) {
        let Some(name_node) = node
            .child_by_field_name("pattern")
            .or_else(|| node.child_by_field_name("name"))
            .or_else(|| node.child_by_field_name("left"))
        else {
            return;
        };
        let Ok(name) = name_node.utf8_text(self.source) else {
            return;
        };
        if name.is_empty() || !is_plain_identifier(name) {
            return;
        }

        // An explicit annotation wins outright — it is what the source says,
        // not what we inferred.
        if let Some(type_node) = node.child_by_field_name("type") {
            if let Some(type_name) = self.simple_type_name(type_node) {
                self.bindings.push(RawBinding {
                    scope: ctx.scope.clone(),
                    name: name.to_string(),
                    source: TypeSource::Annotation(type_name),
                });
                return;
            }
        }

        let Some(value) = node
            .child_by_field_name("value")
            .or_else(|| node.child_by_field_name("right"))
        else {
            return;
        };
        if let Some(source) = self.source_of_value(value) {
            self.bindings.push(RawBinding {
                scope: ctx.scope.clone(),
                name: name.to_string(),
                source,
            });
        }
    }

    /// Classify the right-hand side of a binding into one of the pending forms.
    fn source_of_value(&self, value: Node<'a>) -> Option<TypeSource> {
        let kind = value.kind();

        // `new Store()` names its type directly.
        if kind == "new_expression" {
            let ctor = value.child_by_field_name("constructor")?;
            return Some(TypeSource::Constructor(
                self.simple_type_name(ctor)?.to_string(),
            ));
        }

        if self.config.call_nodes.contains(&kind) {
            let function = value.child_by_field_name("function")?;
            let text = function.utf8_text(self.source).ok()?;

            // `Store::new(…)` / `Store()` — a constructor by convention when the
            // callee's leading segment is type-cased.
            if let Some((head, tail)) = split_call_path(text) {
                if starts_uppercase(head) && (tail == "new" || tail.is_empty()) {
                    return Some(TypeSource::Constructor(head.to_string()));
                }
                // `receiver.method(…)`
                if !head.is_empty() && !starts_uppercase(head) && !tail.is_empty() {
                    return Some(TypeSource::MethodCallResult {
                        receiver: head.to_string(),
                        method: tail.to_string(),
                    });
                }
            }
            if starts_uppercase(text) {
                return Some(TypeSource::Constructor(text.to_string()));
            }
            if is_plain_identifier(text) {
                return Some(TypeSource::CallResult(text.to_string()));
            }
            return None;
        }

        // `store.inner` — a typed field access.
        if self.config.member_nodes.contains(&kind) {
            let object = value.child_by_field_name("object")?;
            let field = value
                .child_by_field_name("field")
                .or_else(|| value.child_by_field_name("attribute"))
                .or_else(|| value.child_by_field_name("property"))?;
            let object_text = object.utf8_text(self.source).ok()?;
            let field_text = field.utf8_text(self.source).ok()?;
            if is_plain_identifier(object_text) && is_plain_identifier(field_text) {
                return Some(TypeSource::FieldAccess {
                    receiver: object_text.to_string(),
                    field: field_text.to_string(),
                });
            }
            return None;
        }

        // `let b = a;`
        let text = value.utf8_text(self.source).ok()?;
        if is_plain_identifier(text) && !starts_uppercase(text) {
            return Some(TypeSource::Copy(text.to_string()));
        }
        None
    }

    /// Record a call site for later resolution (FR-008).
    fn record_call(&mut self, node: Node<'a>, ctx: &Context) {
        let Some(function) = node.child_by_field_name("function") else {
            return;
        };
        let Ok(text) = function.utf8_text(self.source) else {
            return;
        };
        let line = node.start_position().row as u32 + 1;

        match split_call_path(text) {
            Some((receiver, method)) if !method.is_empty() => {
                self.calls.push(CallSite {
                    caller: ctx.contain_parent.clone(),
                    scope: ctx.scope.clone(),
                    receiver: Some(receiver.to_string()),
                    callee: method.to_string(),
                    line,
                });
            }
            _ => {
                if is_plain_identifier(text) {
                    self.calls.push(CallSite {
                        caller: ctx.contain_parent.clone(),
                        scope: ctx.scope.clone(),
                        receiver: None,
                        callee: text.to_string(),
                        line,
                    });
                }
            }
        }
    }

    /// Record a callable's declared return type.
    fn record_signature(&mut self, node: Node<'a>, qualified: &str) {
        let return_node = node
            .child_by_field_name("return_type")
            .or_else(|| node.child_by_field_name("return_type_annotation"));
        if let Some(return_node) = return_node {
            if let Some(type_name) = self.simple_type_name(return_node) {
                self.return_types.insert(qualified.to_string(), type_name);
            }
        }
    }

    /// Record typed parameters as bindings in the callable's own scope.
    fn record_parameters(&mut self, node: Node<'a>, scope: &str) {
        let Some(params) = node.child_by_field_name("parameters") else {
            return;
        };
        let mut cursor = params.walk();
        for param in params.children(&mut cursor) {
            if !self.config.parameter_nodes.contains(&param.kind()) {
                continue;
            }
            let name_node = param
                .child_by_field_name("pattern")
                .or_else(|| param.child_by_field_name("name"));
            let type_node = param.child_by_field_name("type");
            if let (Some(name_node), Some(type_node)) = (name_node, type_node) {
                if let (Ok(name), Some(type_name)) = (
                    name_node.utf8_text(self.source),
                    self.simple_type_name(type_node),
                ) {
                    if is_plain_identifier(name) {
                        self.bindings.push(RawBinding {
                            scope: scope.to_string(),
                            name: name.to_string(),
                            source: TypeSource::Annotation(type_name),
                        });
                    }
                }
            }
        }
    }

    /// Record a field's declared type, so field-access chains can resolve.
    fn record_field(&mut self, node: Node<'a>, ctx: &Context) {
        let Some(owner) = &ctx.owning_type else {
            return;
        };
        let name_node = node
            .child_by_field_name("name")
            .or_else(|| node.child_by_field_name("left"));
        let Some(type_node) = node.child_by_field_name("type") else {
            return;
        };
        if let (Some(name_node), Some(type_name)) = (name_node, self.simple_type_name(type_node)) {
            if let Ok(name) = name_node.utf8_text(self.source) {
                if is_plain_identifier(name) {
                    self.field_types
                        .insert((owner.clone(), name.to_string()), type_name);
                }
            }
        }
    }

    /// TypeScript `class X extends Y implements Z`.
    fn record_type_relations(&mut self, node: Node<'a>, simple_name: &str) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "class_heritage" && child.kind() != "extends_type_clause" {
                continue;
            }
            let mut inner = child.walk();
            let mut is_trait = false;
            for part in child.children(&mut inner) {
                match part.kind() {
                    "extends_clause" => {
                        let mut deep = part.walk();
                        for target in part.children(&mut deep) {
                            if let Some(name) = self.simple_type_name(target) {
                                if name != "extends" {
                                    self.type_relations.push((
                                        simple_name.to_string(),
                                        name,
                                        false,
                                    ));
                                }
                            }
                        }
                    }
                    "implements_clause" => {
                        is_trait = true;
                        let mut deep = part.walk();
                        for target in part.children(&mut deep) {
                            if let Some(name) = self.simple_type_name(target) {
                                if name != "implements" {
                                    self.type_relations
                                        .push((simple_name.to_string(), name, true));
                                }
                            }
                        }
                    }
                    _ => {
                        let _ = is_trait;
                    }
                }
            }
        }
    }

    /// Rust `impl Trait for Type` — the trait relation FR-008 emits as
    /// `implements_trait`.
    fn record_impl_trait(&mut self, node: Node<'a>, impl_type: &str) {
        if let Some(trait_node) = node.child_by_field_name("trait") {
            if let Some(trait_name) = self.simple_type_name(trait_node) {
                self.type_relations
                    .push((impl_type.to_string(), trait_name, true));
            }
        }
    }

    /// The simple name of a type node: generics, references and wrappers
    /// stripped, so `&Option<Store>` and `Store` name one type.
    /// Classify a declaration's visibility (FR-009).
    ///
    /// Every branch is keyed off the language's `VisibilityStyle` rather than
    /// off `Language`, so a new language declares how it spells visibility in
    /// `lang.rs` and this stays untouched (FR-009-CON-2).
    fn visibility_of(&self, node: Node<'a>, simple_name: &str) -> Visibility {
        match self.config.visibility_style {
            VisibilityStyle::RustModifier => self.rust_visibility(node),
            VisibilityStyle::TypeScriptExport => self.typescript_visibility(node),
            VisibilityStyle::PythonUnderscore => python_visibility(simple_name),
        }
    }

    /// Rust: an explicit `pub`/`pub(…)` modifier, else the enclosing trait's
    /// visibility, else private.
    fn rust_visibility(&self, node: Node<'a>) -> Visibility {
        if let Some(text) = self.visibility_modifier_text(node) {
            // `pub` alone is unrestricted; every parenthesized form —
            // `pub(crate)`, `pub(super)`, `pub(in path)` — restricts to the
            // defining unit.
            return if text == "pub" {
                Visibility::Public
            } else {
                Visibility::Crate
            };
        }
        // A trait's associated items carry the trait's visibility, not their
        // own: they are reachable wherever the trait is (FR-009-AC-4). The
        // same holds for the items of a trait `impl` — a caller reaches them
        // through the trait, so treating them as private would under-report a
        // change that dependents really do see.
        if self.has_visibility_inheriting_ancestor(node) || self.in_trait_impl(node) {
            return Visibility::Public;
        }
        Visibility::Private
    }

    /// Whether the node sits inside an `impl Trait for Type` block, as opposed
    /// to an inherent `impl Type` block.
    fn in_trait_impl(&self, node: Node<'a>) -> bool {
        let mut current = node.parent();
        while let Some(ancestor) = current {
            if ancestor.kind() == "impl_item" {
                return ancestor.child_by_field_name("trait").is_some();
            }
            if self.config.decl_for(ancestor.kind()).is_some() {
                return false;
            }
            current = ancestor.parent();
        }
        false
    }

    /// TypeScript/TSX: a class member's accessibility modifier, else `export`
    /// on the declaration, else private.
    fn typescript_visibility(&self, node: Node<'a>) -> Visibility {
        // `#name` is hard-private regardless of position.
        if let Some(name) = node.child_by_field_name("name") {
            if name.kind() == "private_property_identifier" {
                return Visibility::Private;
            }
        }
        if let Some(text) = self.visibility_modifier_text(node) {
            return match text {
                "private" => Visibility::Private,
                "protected" => Visibility::Crate,
                _ => Visibility::Public,
            };
        }
        // A member of a class or interface body with no modifier is public;
        // the body's own declaration governs whether the type escapes.
        if self.is_type_member(node) || self.has_visibility_inheriting_ancestor(node) {
            return Visibility::Public;
        }
        if is_exported(node) {
            return Visibility::Public;
        }
        Visibility::Private
    }

    /// The text of a declaration's own visibility/accessibility modifier.
    fn visibility_modifier_text(&self, node: Node<'a>) -> Option<&'a str> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if self.config.visibility_nodes.contains(&child.kind()) {
                if let Ok(text) = child.utf8_text(self.source) {
                    return Some(text.trim());
                }
            }
        }
        None
    }

    /// Whether the declaration sits inside a body whose items inherit the
    /// enclosing declaration's visibility (a Rust trait, a TS interface).
    fn has_visibility_inheriting_ancestor(&self, node: Node<'a>) -> bool {
        let mut current = node.parent();
        while let Some(ancestor) = current {
            if self
                .config
                .visibility_inheriting_nodes
                .contains(&ancestor.kind())
            {
                return true;
            }
            if self.config.decl_for(ancestor.kind()).is_some() {
                // A nearer declaration governs; stop before crossing it.
                return false;
            }
            current = ancestor.parent();
        }
        false
    }

    /// Whether the node is declared directly in a class or interface body.
    fn is_type_member(&self, node: Node<'a>) -> bool {
        node.parent()
            .map(|p| matches!(p.kind(), "class_body" | "interface_body" | "object_type"))
            .unwrap_or(false)
    }

    /// Render a callable's normalized signature (FR-009).
    ///
    /// Only the parameter list's own child nodes are read, so a comment or a
    /// line break inside the parentheses contributes nothing — that is what
    /// makes the rendering stable across reformatting (FR-009-AC-6).
    fn signature_of(&self, node: Node<'a>) -> Option<String> {
        let params = self.parameter_list(node)?;
        let mut rendered: Vec<String> = Vec::new();
        let mut cursor = params.walk();
        for param in params.named_children(&mut cursor) {
            if self.config.is_comment(param.kind()) {
                continue;
            }
            if let Some(text) = self.render_parameter(param) {
                rendered.push(text);
            }
        }

        let mut signature = format!("({})", rendered.join(", "));
        if let Some(return_type) = self.return_type_text(node) {
            signature.push_str(" -> ");
            signature.push_str(&return_type);
        }
        Some(signature)
    }

    /// The callable's parameter-list node, by field name or by node kind.
    fn parameter_list(&self, node: Node<'a>) -> Option<Node<'a>> {
        if let Some(params) = node.child_by_field_name("parameters") {
            return Some(params);
        }
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .find(|child| self.config.parameter_list_nodes.contains(&child.kind()));
        found
    }

    /// One parameter, rendered as its declared type when it has one and as its
    /// binding name when it does not.
    fn render_parameter(&self, param: Node<'a>) -> Option<String> {
        // A receiver is rendered uniformly, so `self`, `&self`, `&mut self`,
        // `this` and `cls` are not mistaken for signature changes.
        if param.kind() == "self_parameter" {
            return Some("self".to_string());
        }
        let name = param
            .child_by_field_name("pattern")
            .or_else(|| param.child_by_field_name("name"))
            .or(if param.kind() == "identifier" {
                Some(param)
            } else {
                None
            })
            .and_then(|n| n.utf8_text(self.source).ok())
            .map(str::trim);

        if matches!(name, Some("self" | "cls" | "this")) {
            return Some("self".to_string());
        }

        if let Some(type_node) = param.child_by_field_name("type") {
            if let Some(text) = self.normalized_type_text(type_node) {
                return Some(text);
            }
        }

        // Unannotated: the name still carries the arity (FR-009-AC-7). A
        // pattern the grammar exposes no name field for — Python's `*rest`,
        // `**kwargs` — falls back to its own source text so the parameter is
        // never silently dropped from the arity.
        name.filter(|n| !n.is_empty())
            .map(collapse_whitespace)
            .or_else(|| {
                param
                    .utf8_text(self.source)
                    .ok()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .map(collapse_whitespace)
            })
    }

    /// The declared return type, normalized, when the declaration states one.
    fn return_type_text(&self, node: Node<'a>) -> Option<String> {
        for field in self.config.return_type_fields {
            if let Some(return_node) = node.child_by_field_name(field) {
                if let Some(text) = self.normalized_type_text(return_node) {
                    return Some(text);
                }
            }
        }
        None
    }

    /// A declared type's source text with its annotation punctuation stripped
    /// and every whitespace run collapsed (FR-009-AC-6).
    fn normalized_type_text(&self, node: Node<'a>) -> Option<String> {
        let text = node.utf8_text(self.source).ok()?;
        let text = text.trim();
        let text = text
            .strip_prefix("->")
            .or_else(|| text.strip_prefix(':'))
            .unwrap_or(text)
            .trim();
        let collapsed = collapse_whitespace(text);
        if collapsed.is_empty() {
            None
        } else {
            Some(collapsed)
        }
    }

    fn simple_type_name(&self, node: Node<'a>) -> Option<String> {
        let text = node.utf8_text(self.source).ok()?;
        simplify_type(text)
    }

    /// The name segment and simple name for a declaration.
    fn segment_for(
        &mut self,
        node: Node<'a>,
        decl: &'static crate::lang::DeclKind,
        name_parent: &str,
    ) -> (String, String) {
        for field in self.config.name_fields {
            if let Some(name_node) = node.child_by_field_name(field) {
                if let Ok(text) = name_node.utf8_text(self.source) {
                    if !text.is_empty() {
                        return (text.to_string(), text.to_string());
                    }
                }
            }
        }
        // Anonymous: take the next ordinal for this kind under this parent
        // (FR-002-AC-5).
        let counter = self
            .anon_counters
            .entry((name_parent.to_string(), decl.kind))
            .or_insert(0);
        let ordinal = *counter;
        *counter += 1;
        let segment = anonymous_segment(decl.kind, ordinal);
        (segment.clone(), segment)
    }

    /// For a Rust `impl` block, the implementing type's name — used as the name
    /// segment for the methods inside it (FR-002-AC-2).
    fn impl_type_segment(&self, node: Node<'a>) -> Option<String> {
        if node.kind() != "impl_item" {
            return None;
        }
        let type_node = node.child_by_field_name("type")?;
        let text = type_node.utf8_text(self.source).ok()?;
        // Strip generic arguments so `Store<T>` and `Store` name one type.
        let base = text.split('<').next().unwrap_or(text).trim();
        if base.is_empty() {
            None
        } else {
            Some(base.to_string())
        }
    }

    /// Whether a declaration is recognized as a test by its language's
    /// convention (FR-005-AC-8).
    fn looks_like_test(&self, node: Node<'a>, simple_name: &str) -> bool {
        if simple_name.starts_with("test_")
            || simple_name.starts_with("test") && simple_name.len() > 4
        {
            return true;
        }
        // Rust: a `#[test]` or `#[tokio::test]` attribute precedes the item.
        let mut sibling = node.prev_sibling();
        while let Some(prev) = sibling {
            if self.config.is_attribute(prev.kind()) {
                if let Ok(text) = prev.utf8_text(self.source) {
                    if text.contains("test") {
                        return true;
                    }
                }
                sibling = prev.prev_sibling();
                continue;
            }
            break;
        }
        false
    }

    fn record_comment(&mut self, node: Node<'a>, owner: &str, owner_is_test: bool) {
        if let Ok(text) = node.utf8_text(self.source) {
            self.comments.push(CommentText {
                text: text.to_string(),
                line: node.start_position().row as u32 + 1,
                owner: owner.to_string(),
                owner_is_test,
            });
        }
    }

    fn record_import(&mut self, node: Node<'a>) {
        let line = node.start_position().row as u32 + 1;
        let mut cursor = node.walk();
        let mut stack: Vec<Node<'a>> = node.children(&mut cursor).collect();
        while let Some(child) = stack.pop() {
            if self.config.is_string(child.kind()) {
                if let Ok(text) = child.utf8_text(self.source) {
                    let specifier = text.trim_matches(['"', '\'', '`'].as_slice());
                    if !specifier.is_empty() {
                        self.imports.push((specifier.to_string(), line));
                    }
                }
                continue;
            }
            // Rust `use` paths and Python dotted names are not string nodes.
            if matches!(
                child.kind(),
                "scoped_identifier" | "identifier" | "dotted_name" | "relative_import"
            ) {
                if let Ok(text) = child.utf8_text(self.source) {
                    if !text.is_empty() {
                        self.imports.push((text.to_string(), line));
                    }
                }
                continue;
            }
            let mut inner = child.walk();
            stack.extend(child.children(&mut inner));
        }
    }
}

fn span_of(node: Node<'_>) -> LineSpan {
    LineSpan {
        start: node.start_position().row as u32 + 1,
        end: node.end_position().row as u32 + 1,
    }
}

/// Whether `text` is a single bare identifier — no dots, no calls, no operators.
fn is_plain_identifier(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

fn starts_uppercase(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Split `receiver.method` or `Type::method` into its head and tail.
///
/// Only the *last* separator matters: `a.b.c()` has receiver `b`, because that
/// is the expression the receiver's type must be recovered for. A leading
/// `self.` is kept, since `self` is a resolvable receiver.
fn split_call_path(text: &str) -> Option<(&str, &str)> {
    let (head, tail) = match text.rfind("::") {
        Some(idx) => (&text[..idx], &text[idx + 2..]),
        None => {
            let idx = text.rfind('.')?;
            (&text[..idx], &text[idx + 1..])
        }
    };
    if !is_plain_identifier(tail) {
        return None;
    }
    // Take the innermost segment of the receiver path.
    let receiver = head
        .rsplit(['.', ':'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(head);
    if !is_plain_identifier(receiver) {
        return None;
    }
    Some((receiver, tail))
}

/// Reduce a type expression to the simple name resolution keys on.
///
/// `&mut Option<Store>` → `Store`; `Vec<Row>` → `Row`; `crate::store::Store` →
/// `Store`. Wrappers are unwrapped rather than resolved, which is why
/// `Vec<Row>` yields the element type: a call on an element is far more common
/// than a call on the container, and a wrong container type would produce a
/// wrong edge, which NFR-004 forbids outright.
/// Collapse every whitespace run to a single space (FR-009-AC-6).
fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Python has no visibility keyword; the underscore prefix is the convention
/// the language's own tooling honors (FR-009).
fn python_visibility(simple_name: &str) -> Visibility {
    // A dunder (`__init__`) is a protocol hook, not a private helper.
    if simple_name.starts_with("__") && !simple_name.ends_with("__") {
        return Visibility::Private;
    }
    if simple_name.starts_with('_') && !simple_name.starts_with("__") {
        return Visibility::Crate;
    }
    Visibility::Public
}

/// Whether a TypeScript declaration is wrapped in an `export` statement.
fn is_exported(node: Node<'_>) -> bool {
    node.parent()
        .map(|parent| parent.kind() == "export_statement")
        .unwrap_or(false)
}

fn simplify_type(text: &str) -> Option<String> {
    let mut current = text.trim();
    current = current
        .trim_start_matches("->")
        .trim_start_matches(':')
        .trim();
    current = current.trim_start_matches(['&', '*']).trim();
    for prefix in ["mut ", "dyn ", "impl ", "readonly "] {
        current = current.trim_start_matches(prefix).trim();
    }

    // Unwrap a single generic layer at a time.
    while let Some(open) = current.find('<') {
        let close = current.rfind('>')?;
        if close < open {
            break;
        }
        let outer = current[..open].trim();
        let inner = current[open + 1..close].trim();
        // A multi-argument generic is ambiguous; refuse rather than pick one.
        if inner.contains(',') {
            return simplify_bare(outer);
        }
        if inner.is_empty() {
            return simplify_bare(outer);
        }
        current = inner;
        current = current.trim_start_matches(['&', '*']).trim();
    }

    simplify_bare(current)
}

fn simplify_bare(text: &str) -> Option<String> {
    let last = text
        .rsplit("::")
        .next()
        .unwrap_or(text)
        .rsplit('.')
        .next()
        .unwrap_or(text)
        .trim()
        .trim_end_matches(['?', '!', '[', ']']);
    if last.is_empty() || !is_plain_identifier(last) || !starts_uppercase(last) {
        return None;
    }
    Some(last.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust(content: &str) -> ParsedFile {
        parse_file(&SourceFile::new(
            "agent-ix",
            "quire-code-rs",
            "src/lib.rs",
            Language::Rust,
            content,
        ))
    }

    // TC-078 — FR-009-AC-1: Rust visibility modifiers classify three ways.
    #[test]
    fn rust_visibility_modifiers_classify_public_crate_and_private() {
        let parsed = rust(
            "pub fn exported() {}\n\
             pub(crate) fn unit_wide() {}\n\
             pub(super) fn parent_wide() {}\n\
             fn hidden() {}\n",
        );
        let vis = |name: &str| {
            parsed
                .facts
                .iter()
                .find(|f| f.simple_name == name)
                .unwrap_or_else(|| panic!("no fact for {name}: {:?}", parsed.facts))
                .visibility
        };
        assert_eq!(vis("exported"), Visibility::Public);
        assert_eq!(vis("unit_wide"), Visibility::Crate);
        assert_eq!(vis("parent_wide"), Visibility::Crate);
        assert_eq!(vis("hidden"), Visibility::Private);
        assert!(
            Visibility::Public.is_exported() && !Visibility::Crate.is_exported(),
            "only `public` counts as an export for change tiering"
        );
    }

    // TC-079 — FR-009-AC-2: TypeScript `export` and class access modifiers.
    #[test]
    fn typescript_export_and_access_modifiers_classify() {
        let parsed = parse_file(&SourceFile::new(
            "agent-ix",
            "ui",
            "src/store.ts",
            Language::TypeScript,
            "export function shipped(): void {}\n\
             function internal(): void {}\n\
             export class Store {\n\
            \x20 private secret(): void {}\n\
            \x20 protected shared(): void {}\n\
            \x20 open(): void {}\n\
            \x20 #hard(): void {}\n\
             }\n",
        ));
        let vis = |name: &str| {
            parsed
                .facts
                .iter()
                .find(|f| f.simple_name == name)
                .unwrap_or_else(|| panic!("no fact for {name}: {:?}", parsed.facts))
                .visibility
        };
        assert_eq!(vis("shipped"), Visibility::Public);
        assert_eq!(vis("internal"), Visibility::Private);
        assert_eq!(vis("Store"), Visibility::Public);
        assert_eq!(vis("secret"), Visibility::Private);
        assert_eq!(vis("shared"), Visibility::Crate);
        assert_eq!(vis("open"), Visibility::Public);
        assert_eq!(vis("#hard"), Visibility::Private);
    }

    // TC-080 — FR-009-AC-3: Python's underscore convention is the visibility.
    #[test]
    fn python_underscore_convention_classifies_visibility() {
        let parsed = parse_file(&SourceFile::new(
            "agent-ix",
            "tool",
            "tool/main.py",
            Language::Python,
            "def helper():\n    pass\n\
             def _internal():\n    pass\n\
             def __hidden():\n    pass\n\
             class Store:\n\
            \x20   def __init__(self):\n        pass\n",
        ));
        let vis = |name: &str| {
            parsed
                .facts
                .iter()
                .find(|f| f.simple_name == name)
                .unwrap_or_else(|| panic!("no fact for {name}: {:?}", parsed.facts))
                .visibility
        };
        assert_eq!(vis("helper"), Visibility::Public);
        assert_eq!(vis("_internal"), Visibility::Crate);
        assert_eq!(vis("__hidden"), Visibility::Private);
        // A dunder is a protocol hook, not a private helper.
        assert_eq!(vis("__init__"), Visibility::Public);
    }

    // TC-081 — FR-009-AC-4: trait and trait-impl items inherit the trait's
    // reach; an inherent-impl item keeps its own modifier.
    #[test]
    fn rust_trait_items_are_public_and_inherent_impl_items_are_not() {
        let parsed = rust(
            "pub trait Persist {\n\
            \x20   fn save(&self) {}\n\
             }\n\
             pub struct Store;\n\
             impl Persist for Store {\n\
            \x20   fn save(&self) {}\n\
             }\n\
             impl Store {\n\
            \x20   fn helper(&self) {}\n\
             }\n",
        );
        let by_name: Vec<_> = parsed
            .facts
            .iter()
            .filter(|f| f.simple_name == "save")
            .map(|f| f.visibility)
            .collect();
        assert_eq!(
            by_name,
            vec![Visibility::Public, Visibility::Public],
            "trait default body and trait impl both reach through the trait"
        );
        let helper = parsed
            .facts
            .iter()
            .find(|f| f.simple_name == "helper")
            .expect("inherent impl method");
        assert_eq!(helper.visibility, Visibility::Private);
    }

    // TC-082 — FR-009-AC-5: parameter types and return type, receiver as self.
    #[test]
    fn signature_renders_parameter_types_and_return_type() {
        let parsed = rust(
            "impl Store {\n\
            \x20   pub fn parse(&self, input: &str, limit: u32) -> Result<Doc, Error> {}\n\
             }\n",
        );
        let parse_fn = parsed
            .facts
            .iter()
            .find(|f| f.simple_name == "parse")
            .expect("method fact");
        assert_eq!(
            parse_fn.signature.as_deref(),
            Some("(self, &str, u32) -> Result<Doc, Error>")
        );

        let ts = parse_file(&SourceFile::new(
            "agent-ix",
            "ui",
            "src/store.ts",
            Language::TypeScript,
            "export function render(node: Node, depth: number): string { return ''; }\n",
        ));
        let render = ts
            .facts
            .iter()
            .find(|f| f.simple_name == "render")
            .expect("function fact");
        assert_eq!(
            render.signature.as_deref(),
            Some("(Node, number) -> string")
        );
    }

    // TC-083 — FR-009-AC-6: reformatting and in-list comments change nothing.
    #[test]
    fn signature_is_stable_across_reformatting_and_comments() {
        let compact = rust("pub fn parse(input: &str, limit: u32) -> Doc {}\n");
        let sprawling = rust(
            "pub fn parse(\n\
            \x20   // the source text\n\
            \x20   input:   &str,\n\
            \x20   /* how many */ limit:\n\
            \x20       u32,\n\
             ) -> Doc\n\
             {}\n",
        );
        let signature = |parsed: &ParsedFile| {
            parsed
                .facts
                .iter()
                .find(|f| f.simple_name == "parse")
                .expect("function fact")
                .signature
                .clone()
        };
        assert_eq!(signature(&compact), signature(&sprawling));
        assert_eq!(signature(&compact).as_deref(), Some("(&str, u32) -> Doc"));
    }

    // TC-084 — FR-009-AC-7: unannotated params fall back to names; a
    // declaration with no parameter list carries no signature.
    #[test]
    fn unannotated_parameters_render_names_and_non_callables_have_no_signature() {
        let parsed = parse_file(&SourceFile::new(
            "agent-ix",
            "tool",
            "tool/main.py",
            Language::Python,
            "def merge(left, right, *rest):\n    pass\nclass Store:\n    pass\n",
        ));
        let merge = parsed
            .facts
            .iter()
            .find(|f| f.simple_name == "merge")
            .expect("function fact");
        assert_eq!(merge.signature.as_deref(), Some("(left, right, *rest)"));

        let store = parsed
            .facts
            .iter()
            .find(|f| f.simple_name == "Store")
            .expect("class fact");
        assert_eq!(store.signature, None, "a type has no parameter list");
        let file = parsed
            .facts
            .iter()
            .find(|f| f.object_type == ObjectType::CodeFile)
            .expect("file fact");
        assert_eq!(file.signature, None);
        assert_eq!(file.visibility, Visibility::Public);
    }

    // TC-085 — FR-009-AC-8: a parameter-type change is visible in the
    // signature and invisible to identity; a new private helper disturbs no
    // existing record.
    #[test]
    fn parameter_type_change_moves_the_signature_not_the_identity() {
        let before = rust("pub fn parse(input: &str) -> Doc {}\n");
        let after = rust("pub fn parse(input: &[u8]) -> Doc {}\n");
        let fact_of = |parsed: &ParsedFile| {
            parsed
                .facts
                .iter()
                .find(|f| f.simple_name == "parse")
                .expect("function fact")
                .clone()
        };
        let (a, b) = (fact_of(&before), fact_of(&after));
        assert_eq!(a.qualified_name, b.qualified_name, "identity is name-keyed");
        assert_ne!(
            a.signature, b.signature,
            "a parameter-type change must not read as a body-only edit"
        );

        let with_helper = rust("pub fn parse(input: &str) -> Doc {}\nfn helper() {}\n");
        let helper = with_helper
            .facts
            .iter()
            .find(|f| f.simple_name == "helper")
            .expect("helper fact");
        assert!(
            !helper.visibility.is_exported(),
            "a private helper is not part of the export set"
        );
        assert_eq!(
            fact_of(&with_helper),
            a,
            "adding a private helper leaves the existing declaration untouched"
        );
    }

    // TC-001 — FR-001-AC-1: a Rust fixture yields all four fact types.
    #[test]
    fn rust_fixture_yields_every_fact_type() {
        let parsed = rust(
            r#"
mod store {
    pub struct Store { pub n: u32 }
    pub enum Mode { On, Off }
    pub trait Persist { fn save(&self); }
    impl Store {
        pub fn upsert(&self) {}
    }
    pub fn helper() {}
}
"#,
        );
        let types: Vec<_> = parsed
            .facts
            .iter()
            .map(|f| f.object_type)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        assert_eq!(
            types,
            vec![
                ObjectType::CodeFile,
                ObjectType::Module,
                ObjectType::Function,
                ObjectType::Type
            ]
        );
    }

    // TC-009 — FR-002-AC-2: a method is named with its implementing type.
    #[test]
    fn methods_are_named_by_their_implementing_type() {
        let parsed = rust(
            r#"
pub struct Store;
impl Store {
    pub fn upsert(&self) {}
}
trait Persist { fn save(&self); }
impl Persist for Store {
    fn save(&self) {}
}
"#,
        );
        let names: Vec<_> = parsed
            .facts
            .iter()
            .filter(|f| f.object_type == ObjectType::Function)
            .map(|f| f.qualified_name.as_str())
            .collect();
        assert!(
            names.contains(&"agent-ix/quire-code-rs/src/lib.rs::Store::upsert"),
            "got {names:?}"
        );
        // A trait method shares the naming scheme of an inherent one.
        assert!(
            names.contains(&"agent-ix/quire-code-rs/src/lib.rs::Store::save"),
            "got {names:?}"
        );
    }

    // TC-004 — FR-001-AC-4: an empty file still yields one code_file fact.
    #[test]
    fn an_empty_file_still_yields_its_file_fact() {
        let parsed = rust("");
        assert_eq!(parsed.facts.len(), 1);
        assert_eq!(parsed.facts[0].object_type, ObjectType::CodeFile);
    }

    // TC-005 — FR-001-AC-5: facts carry a kind and one-based inclusive spans.
    #[test]
    fn facts_carry_kind_and_one_based_spans() {
        let parsed = rust("pub fn first() {}\n");
        let func = parsed
            .facts
            .iter()
            .find(|f| f.object_type == ObjectType::Function)
            .expect("function fact");
        assert_eq!(func.kind, "function");
        assert_eq!(func.span.start, 1);
        assert!(func.span.end >= func.span.start);
    }

    // TC-006 — FR-001-AC-6: facts are ordered by declaration start position.
    #[test]
    fn facts_are_ordered_by_start_position() {
        let parsed = rust("fn a() {}\nfn b() {}\nfn c() {}\n");
        let lines: Vec<_> = parsed
            .facts
            .iter()
            .filter(|f| f.object_type == ObjectType::Function)
            .map(|f| f.span.start)
            .collect();
        let mut sorted = lines.clone();
        sorted.sort_unstable();
        assert_eq!(lines, sorted);
    }

    // TC-041 — FR-007-AC-3: intact declarations survive a malformed sibling.
    #[test]
    fn a_malformed_declaration_does_not_sink_its_siblings() {
        let parsed = rust("fn good() {}\nfn broken( {\nfn also_good() {}\n");
        assert!(
            !parsed.diagnostics.is_empty(),
            "a syntax error should be diagnosed"
        );
        let names: Vec<_> = parsed
            .facts
            .iter()
            .map(|f| f.simple_name.as_str())
            .collect();
        assert!(names.contains(&"good"), "got {names:?}");
    }

    // TC-041 — FR-007-AC-3: the unparseable declaration contributes nothing,
    // while its intact siblings survive.
    #[test]
    fn an_unparseable_declaration_contributes_no_fact() {
        let parsed = rust("fn good() {}\nfn broken( {\nfn also_good() {}\n");
        let names: Vec<_> = parsed
            .facts
            .iter()
            .map(|f| f.simple_name.as_str())
            .collect();
        assert!(names.contains(&"good"), "got {names:?}");
        assert!(
            !names.contains(&"broken"),
            "a declaration the grammar could not resolve must yield no fact: {names:?}"
        );
    }

    // TC-075 — FR-007-AC-7: an error inside a body does not cost the
    // declaration its fact. Its signature is readable, so its identity is too.
    #[test]
    fn a_body_error_does_not_suppress_the_declaration() {
        let parsed = rust("fn outer() {\n    let x = ;\n}\nfn after() {}\n");
        let names: Vec<_> = parsed
            .facts
            .iter()
            .map(|f| f.simple_name.as_str())
            .collect();
        assert!(
            names.contains(&"outer"),
            "a typo in the body must not cost the function its node: {names:?}"
        );
        assert!(names.contains(&"after"), "got {names:?}");
        assert!(
            parsed.diagnostics.iter().any(|d| d.code == "parse_error"),
            "the error is still diagnosed"
        );
    }

    // TC-040 — FR-007-AC-2: the diagnostic carries path and first error position.
    #[test]
    fn parse_diagnostics_carry_a_position() {
        let parsed = rust("fn good() {}\nfn broken( {\n");
        let diag = parsed
            .diagnostics
            .iter()
            .find(|d| d.code == "parse_error")
            .expect("parse error diagnostic");
        assert_eq!(diag.path, "src/lib.rs");
        assert!(diag.line.is_some());
    }

    // TC-043 — FR-007-AC-5: arbitrary input yields a diagnostic, never a panic.
    #[test]
    fn arbitrary_input_never_panics() {
        for content in ["", "\0\0\0", "((((((", "🙂🙂🙂", "fn"] {
            let parsed = rust(content);
            assert!(!parsed.facts.is_empty(), "code_file fact is always present");
        }
    }

    // TC-003 — FR-001-AC-3: Python yields class, method and function facts.
    #[test]
    fn python_fixture_yields_classes_and_functions() {
        let parsed = parse_file(&SourceFile::new(
            "agent-ix",
            "tool",
            "tool/main.py",
            Language::Python,
            "class Store:\n    def upsert(self):\n        pass\n\ndef helper():\n    pass\n",
        ));
        let names: Vec<_> = parsed
            .facts
            .iter()
            .map(|f| f.qualified_name.as_str())
            .collect();
        assert!(
            names.contains(&"agent-ix/tool/tool/main.py::Store"),
            "got {names:?}"
        );
        assert!(
            names.contains(&"agent-ix/tool/tool/main.py::Store::upsert"),
            "got {names:?}"
        );
        assert!(
            names.contains(&"agent-ix/tool/tool/main.py::helper"),
            "got {names:?}"
        );
    }

    // TC-076 — FR-001-AC-8: `code_module` denotes a namespace declared *within*
    // a file, never a file that merely happens to be importable.
    #[test]
    fn only_in_file_namespaces_yield_module_facts() {
        let python = parse_file(&SourceFile::new(
            "agent-ix",
            "tool",
            "tool/main.py",
            Language::Python,
            "class Store:\n    pass\n",
        ));
        assert!(
            !python
                .facts
                .iter()
                .any(|f| f.object_type == ObjectType::Module),
            "a Python file is its own module; a second node would give one \
             construct two identities"
        );

        let rust_mod = rust("pub mod inner {\n    pub fn f() {}\n}\n");
        assert!(
            rust_mod
                .facts
                .iter()
                .any(|f| f.object_type == ObjectType::Module),
            "a Rust `mod` block is a namespace within the file"
        );

        let ts_namespace = parse_file(&SourceFile::new(
            "agent-ix",
            "ui",
            "src/app.ts",
            Language::TypeScript,
            "export namespace Inner {\n  export function f(): void {}\n}\n",
        ));
        assert!(
            ts_namespace
                .facts
                .iter()
                .any(|f| f.object_type == ObjectType::Module),
            "a TypeScript namespace is a namespace within the file: {:?}",
            ts_namespace.facts
        );
    }

    // TC-002 — FR-001-AC-2: TypeScript and TSX yield their declaration forms.
    #[test]
    fn typescript_fixture_yields_classes_interfaces_and_aliases() {
        let parsed = parse_file(&SourceFile::new(
            "agent-ix",
            "ui",
            "src/store.ts",
            Language::TypeScript,
            "export interface Shape { n: number }\n\
             export type Alias = Shape;\n\
             export class Store {\n  upsert(): void {}\n}\n",
        ));
        let kinds: Vec<_> = parsed.facts.iter().map(|f| f.kind).collect();
        assert!(kinds.contains(&"interface"), "got {kinds:?}");
        assert!(kinds.contains(&"type_alias"), "got {kinds:?}");
        assert!(kinds.contains(&"class"), "got {kinds:?}");
        assert!(kinds.contains(&"method"), "got {kinds:?}");
    }

    #[test]
    fn tsx_parses_with_its_own_grammar() {
        let parsed = parse_file(&SourceFile::new(
            "agent-ix",
            "ui",
            "src/App.tsx",
            Language::Tsx,
            "export class App {\n  render() { return <div />; }\n}\n",
        ));
        assert!(
            parsed.diagnostics.iter().all(|d| d.code != "parse_error"),
            "TSX should parse cleanly: {:?}",
            parsed.diagnostics
        );
        assert!(parsed.facts.iter().any(|f| f.kind == "class"));
    }
}
