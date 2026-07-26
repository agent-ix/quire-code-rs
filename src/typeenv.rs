//! Per-file type environments and the fixpoint binding solver.
//!
//! Implements the environment half of
//! [FR-008](../spec/functional/FR-008-type-environments-and-call-resolution.md).
//!
//! Approach ported from gitnexus's scope-resolution work (`type-env.ts`,
//! `type-resolution-system.md`): collect bindings during a single walk, then
//! iterate the ones that depend on other bindings until nothing new appears.
//! The four pending forms — copy, call-result, field-access and
//! method-call-result — are exactly the chains that a single pass cannot
//! resolve, because the binding they depend on may be declared later in the
//! file than the binding that needs it.
//!
//! This is not a type checker. It recovers what the source states plainly and
//! stops; every remaining ambiguity resolves to nothing, never to a guess
//! (FR-008-CON-1).

use std::collections::{BTreeMap, BTreeSet};

/// The file-level scope key.
pub const FILE_SCOPE: &str = "";

/// Maximum fixpoint iterations (FR-008-CON-2). Reaching the bound degrades to
/// fewer edges — never to unbounded work, and never to a guess.
pub const MAX_ITERATIONS: usize = 10;

/// Where a binding's type comes from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypeSource {
    /// An explicit annotation: `let x: Store`, `x: Store = …`.
    Annotation(String),
    /// A constructor call: `Store::new()`, `new Store()`, `Store()`.
    Constructor(String),
    /// Assignment from another binding: `let y = x`.
    Copy(String),
    /// Assignment from a free function's declared return type: `let x = make()`.
    CallResult(String),
    /// Assignment from a typed field: `let x = store.inner`.
    FieldAccess { receiver: String, field: String },
    /// Assignment from a method's declared return type: `let x = store.get()`.
    MethodCallResult { receiver: String, method: String },
}

/// One binding recovered during the walk, before resolution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawBinding {
    /// Scope key: [`FILE_SCOPE`], or `name@line` for a callable's scope.
    pub scope: String,
    pub name: String,
    pub source: TypeSource,
}

/// Batch-wide declarations the solver consults. Built once per extraction and
/// dropped with it (ADR-002).
#[derive(Debug, Default)]
pub struct Corpus {
    /// Simple type name -> qualified names declaring it. More than one entry
    /// means the name is ambiguous across the batch.
    pub types: BTreeMap<String, BTreeSet<String>>,
    /// (simple type name, method name) -> qualified method names.
    pub methods: BTreeMap<(String, String), BTreeSet<String>>,
    /// Free function simple name -> qualified names.
    pub functions: BTreeMap<String, BTreeSet<String>>,
    /// Qualified callable name -> its declared return type, simple form.
    pub return_types: BTreeMap<String, String>,
    /// (simple type name, field name) -> the field's declared type.
    pub field_types: BTreeMap<(String, String), String>,
}

impl Corpus {
    /// The declared return type of a free function, when exactly one function
    /// of that name exists and it declares one.
    fn function_return(&self, name: &str) -> Option<&str> {
        let candidates = self.functions.get(name)?;
        let only = single(candidates)?;
        self.return_types.get(only).map(|s| s.as_str())
    }

    /// The declared return type of `type::method`, when it declares one.
    fn method_return(&self, type_name: &str, method: &str) -> Option<&str> {
        let candidates = self
            .methods
            .get(&(type_name.to_string(), method.to_string()))?;
        let only = single(candidates)?;
        self.return_types.get(only).map(|s| s.as_str())
    }

    fn field_type(&self, type_name: &str, field: &str) -> Option<&str> {
        self.field_types
            .get(&(type_name.to_string(), field.to_string()))
            .map(|s| s.as_str())
    }
}

/// The unique element of a set, or `None` when the set is empty or ambiguous.
fn single(set: &BTreeSet<String>) -> Option<&String> {
    let mut iter = set.iter();
    let first = iter.next()?;
    match iter.next() {
        Some(_) => None,
        None => Some(first),
    }
}

/// A resolved per-file type environment.
#[derive(Debug, Default)]
pub struct TypeEnv {
    /// scope key -> (binding name -> simple type name).
    scopes: BTreeMap<String, BTreeMap<String, String>>,
    /// Scope key -> the type enclosing it, for `self` / `this` receivers.
    enclosing_type: BTreeMap<String, String>,
    /// Iterations the fixpoint took. Reported for NFR-003-AC-4.
    pub iterations: usize,
    /// Whether the iteration bound was reached with work still pending.
    pub hit_iteration_bound: bool,
}

impl TypeEnv {
    /// Build the environment: seed the direct bindings, then iterate the
    /// pending ones to a fixed point.
    pub fn build(
        bindings: &[RawBinding],
        enclosing_types: &BTreeMap<String, String>,
        corpus: &Corpus,
    ) -> Self {
        let mut env = TypeEnv {
            enclosing_type: enclosing_types.clone(),
            ..Default::default()
        };

        // Pass one: everything that resolves without consulting another
        // binding. Annotations and constructors are the two direct forms.
        let mut pending: Vec<&RawBinding> = Vec::new();
        for binding in bindings {
            match &binding.source {
                TypeSource::Annotation(type_name) | TypeSource::Constructor(type_name) => {
                    env.insert(&binding.scope, &binding.name, type_name);
                }
                _ => pending.push(binding),
            }
        }

        // Pass two: iterate the rest. Each round resolves whatever became
        // resolvable in the previous one, so a chain written in reverse order
        // still converges.
        for iteration in 1..=MAX_ITERATIONS {
            env.iterations = iteration;
            let mut progressed = false;
            let mut still_pending = Vec::with_capacity(pending.len());

            for binding in pending {
                match env.resolve_source(&binding.scope, &binding.source, corpus) {
                    Some(type_name) => {
                        env.insert(&binding.scope, &binding.name, &type_name);
                        progressed = true;
                    }
                    None => still_pending.push(binding),
                }
            }

            pending = still_pending;
            if pending.is_empty() || !progressed {
                return env;
            }
        }

        // The bound was reached with bindings still unresolved. Emit what
        // resolved; the rest stay unknown, which costs edges rather than
        // correctness (FR-008-CON-2).
        env.hit_iteration_bound = !pending.is_empty();
        env
    }

    fn resolve_source(&self, scope: &str, source: &TypeSource, corpus: &Corpus) -> Option<String> {
        match source {
            TypeSource::Annotation(t) | TypeSource::Constructor(t) => Some(t.clone()),
            TypeSource::Copy(name) => self.lookup(scope, name).map(str::to_string),
            TypeSource::CallResult(func) => corpus.function_return(func).map(str::to_string),
            TypeSource::FieldAccess { receiver, field } => {
                let receiver_type = self.lookup(scope, receiver)?;
                corpus.field_type(receiver_type, field).map(str::to_string)
            }
            TypeSource::MethodCallResult { receiver, method } => {
                let receiver_type = self.lookup(scope, receiver)?;
                corpus
                    .method_return(receiver_type, method)
                    .map(str::to_string)
            }
        }
    }

    fn insert(&mut self, scope: &str, name: &str, type_name: &str) {
        // First writer wins: an explicit annotation set in pass one is never
        // overwritten by a weaker inference in pass two.
        self.scopes
            .entry(scope.to_string())
            .or_default()
            .entry(name.to_string())
            .or_insert_with(|| type_name.to_string());
    }

    /// Resolve a receiver name in a scope.
    ///
    /// Order: special receivers (`self`, `this`, `Self`), then the local scope,
    /// then file scope. A name rebound locally never leaks outward
    /// (FR-008-AC-5).
    pub fn lookup(&self, scope: &str, name: &str) -> Option<&str> {
        if matches!(name, "self" | "this" | "Self") {
            if let Some(owner) = self.enclosing_type.get(scope) {
                return Some(owner.as_str());
            }
        }
        if let Some(found) = self.scopes.get(scope).and_then(|s| s.get(name)) {
            return Some(found.as_str());
        }
        self.scopes
            .get(FILE_SCOPE)
            .and_then(|s| s.get(name))
            .map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        let mut corpus = Corpus::default();
        corpus
            .types
            .entry("Store".into())
            .or_default()
            .insert("demo/src/store.rs::Store".into());
        corpus
            .methods
            .entry(("Store".into(), "get".into()))
            .or_default()
            .insert("demo/src/store.rs::Store::get".into());
        corpus
            .functions
            .entry("make_store".into())
            .or_default()
            .insert("demo/src/store.rs::make_store".into());
        corpus
            .return_types
            .insert("demo/src/store.rs::make_store".into(), "Store".into());
        corpus
            .return_types
            .insert("demo/src/store.rs::Store::get".into(), "Row".into());
        corpus
            .field_types
            .insert(("Store".into(), "inner".into()), "Inner".into());
        corpus
    }

    fn binding(scope: &str, name: &str, source: TypeSource) -> RawBinding {
        RawBinding {
            scope: scope.to_string(),
            name: name.to_string(),
            source,
        }
    }

    #[test]
    fn annotations_resolve_directly() {
        let env = TypeEnv::build(
            &[binding("", "s", TypeSource::Annotation("Store".into()))],
            &BTreeMap::new(),
            &corpus(),
        );
        assert_eq!(env.lookup("", "s"), Some("Store"));
    }

    // TC-046 — FR-008-AC-3: a copy binding resolves through the fixpoint, even
    // when written before the binding it copies.
    #[test]
    fn copy_chains_resolve_in_either_order() {
        let reverse_order = [
            binding("", "c", TypeSource::Copy("b".into())),
            binding("", "b", TypeSource::Copy("a".into())),
            binding("", "a", TypeSource::Annotation("Store".into())),
        ];
        let env = TypeEnv::build(&reverse_order, &BTreeMap::new(), &corpus());
        assert_eq!(env.lookup("", "a"), Some("Store"));
        assert_eq!(env.lookup("", "b"), Some("Store"));
        assert_eq!(env.lookup("", "c"), Some("Store"));
    }

    // TC-047 — FR-008-AC-4: a call-result binding resolves through the fixpoint.
    #[test]
    fn call_result_bindings_resolve_from_declared_return_types() {
        let env = TypeEnv::build(
            &[binding(
                "",
                "s",
                TypeSource::CallResult("make_store".into()),
            )],
            &BTreeMap::new(),
            &corpus(),
        );
        assert_eq!(env.lookup("", "s"), Some("Store"));
    }

    #[test]
    fn field_and_method_chains_resolve() {
        let bindings = [
            binding("", "s", TypeSource::CallResult("make_store".into())),
            binding(
                "",
                "i",
                TypeSource::FieldAccess {
                    receiver: "s".into(),
                    field: "inner".into(),
                },
            ),
            binding(
                "",
                "r",
                TypeSource::MethodCallResult {
                    receiver: "s".into(),
                    method: "get".into(),
                },
            ),
        ];
        let env = TypeEnv::build(&bindings, &BTreeMap::new(), &corpus());
        assert_eq!(env.lookup("", "i"), Some("Inner"));
        assert_eq!(env.lookup("", "r"), Some("Row"));
    }

    // TC-048 — FR-008-AC-5: a local rebinding does not leak to file level.
    #[test]
    fn local_bindings_do_not_leak_to_file_scope() {
        let bindings = [
            binding("", "s", TypeSource::Annotation("Store".into())),
            binding("f@10", "s", TypeSource::Annotation("Other".into())),
        ];
        let env = TypeEnv::build(&bindings, &BTreeMap::new(), &corpus());
        assert_eq!(env.lookup("f@10", "s"), Some("Other"));
        assert_eq!(env.lookup("", "s"), Some("Store"));
        // A scope with no local binding falls back to file scope.
        assert_eq!(env.lookup("g@20", "s"), Some("Store"));
    }

    // TC-049 — FR-008-AC-6: a cycle terminates at the bound and emits only what
    // resolved, rather than looping or guessing.
    #[test]
    fn cyclic_bindings_terminate_without_guessing() {
        let bindings = [
            binding("", "a", TypeSource::Copy("b".into())),
            binding("", "b", TypeSource::Copy("a".into())),
        ];
        let env = TypeEnv::build(&bindings, &BTreeMap::new(), &corpus());
        assert_eq!(env.lookup("", "a"), None);
        assert_eq!(env.lookup("", "b"), None);
        assert!(env.iterations <= MAX_ITERATIONS);
    }

    #[test]
    fn a_long_chain_converges_well_inside_the_bound() {
        let mut bindings = vec![binding("", "v0", TypeSource::Annotation("Store".into()))];
        for i in 1..8 {
            bindings.push(binding(
                "",
                &format!("v{i}"),
                TypeSource::Copy(format!("v{}", i - 1)),
            ));
        }
        bindings.reverse();
        let env = TypeEnv::build(&bindings, &BTreeMap::new(), &corpus());
        assert_eq!(env.lookup("", "v7"), Some("Store"));
        assert!(!env.hit_iteration_bound);
    }

    #[test]
    fn self_resolves_to_the_enclosing_type() {
        let mut enclosing = BTreeMap::new();
        enclosing.insert("upsert@5".to_string(), "Store".to_string());
        let env = TypeEnv::build(&[], &enclosing, &corpus());
        assert_eq!(env.lookup("upsert@5", "self"), Some("Store"));
        assert_eq!(env.lookup("upsert@5", "this"), Some("Store"));
        assert_eq!(env.lookup("elsewhere@9", "self"), None);
    }

    #[test]
    fn an_ambiguous_function_name_yields_no_binding() {
        let mut c = corpus();
        c.functions
            .entry("make_store".into())
            .or_default()
            .insert("demo/src/other.rs::make_store".into());
        let env = TypeEnv::build(
            &[binding(
                "",
                "s",
                TypeSource::CallResult("make_store".into()),
            )],
            &BTreeMap::new(),
            &c,
        );
        assert_eq!(env.lookup("", "s"), None, "ambiguity must not guess");
    }
}
