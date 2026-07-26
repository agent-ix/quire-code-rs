//! Per-file parsing into structural facts.
//!
//! Implements [FR-001](../spec/functional/FR-001-structural-fact-model.md) and
//! the error policy of
//! [FR-007](../spec/functional/FR-007-parse-error-isolation.md).
//!
//! The error policy is per *declaration*, not per file. That came out of the
//! spec review (SR-001 FND-002): suppressing a whole file because one function
//! is mid-edit contradicts the reason ADR-001 chose tree-sitter in the first
//! place — extraction has to work on trees that do not compile.

use std::collections::BTreeMap;

use tree_sitter::{Node, Parser, Tree};

use crate::facts::{CodeFact, Diagnostic, LineSpan, ObjectType};
use crate::lang::{Language, LanguageConfig};
use crate::naming::{anonymous_segment, child_name, file_name, normalize_path};

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
    };
    walker.walk(root, &file_qualified_name, &file_qualified_name, false);

    out.facts.extend(walker.facts);
    out.comments = walker.comments;
    out.imports = walker.imports;
    out
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
}

impl<'a> Walker<'a> {
    /// Walk `node`, attributing declarations to `name_parent` (the nearest
    /// enclosing declaration that contributes a name segment) and containment
    /// to `contain_parent` (the nearest enclosing fact).
    fn walk(&mut self, node: Node<'a>, name_parent: &str, contain_parent: &str, in_test: bool) {
        let mut cursor = node.walk();
        let children: Vec<Node<'a>> = node.children(&mut cursor).collect();

        for child in children {
            let kind = child.kind();

            if self.config.is_comment(kind) || self.config.is_attribute(kind) {
                self.record_comment(child, contain_parent, in_test);
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

            match self.config.decl_for(kind) {
                Some(decl) => {
                    let (segment, simple_name) = self.segment_for(child, decl, name_parent);
                    let qualified = child_name(name_parent, &segment);
                    let span = span_of(child);
                    let is_test = in_test || self.looks_like_test(child, &simple_name);

                    self.facts.push(CodeFact {
                        object_type: decl.object_type,
                        kind: decl.kind,
                        qualified_name: qualified.clone(),
                        simple_name,
                        path: self.path.to_string(),
                        span,
                        parent: Some(contain_parent.to_string()),
                    });

                    let next_name_parent = if decl.names_children {
                        qualified.as_str()
                    } else {
                        name_parent
                    };
                    // A fresh borrow is needed because next_name_parent may
                    // point into `qualified`.
                    let next_name_parent = next_name_parent.to_string();
                    self.walk(child, &next_name_parent, &qualified, is_test);
                }
                None => {
                    // Not a declaration in this language's config. Rust `impl`
                    // blocks land here deliberately: FR-002 names a method by
                    // its implementing type, so the impl contributes a name
                    // segment without becoming a fact of its own.
                    if let Some(impl_type) = self.impl_type_segment(child) {
                        let qualified = child_name(name_parent, &impl_type);
                        self.walk(child, &qualified, contain_parent, in_test);
                    } else {
                        self.walk(child, name_parent, contain_parent, in_test);
                    }
                }
            }
        }
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
