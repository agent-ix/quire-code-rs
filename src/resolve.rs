//! Receiver-typed call resolution.
//!
//! Implements the resolution half of
//! [FR-008](../spec/functional/FR-008-type-environments-and-call-resolution.md)
//! and is bounded by
//! [NFR-004](../spec/non-functional/NFR-004-conservative-resolution-precision.md).
//!
//! Three tiers, tried in order, each narrower than the last:
//!
//! 1. **receiver-typed** — the receiver's type is known, so the callee is the
//!    method that type declares.
//! 2. **import-scoped** — the receiver's type is unknown, but exactly one type
//!    reachable through the file's resolved imports declares that method.
//! 3. **name-match** — neither holds, but exactly one type in the whole batch
//!    declares that method.
//!
//! At every tier the rule is the same and it is the point of the whole module:
//! **more than one surviving candidate emits nothing**. A missing edge leaves a
//! reader where they already were; a wrong one sends them somewhere irrelevant
//! and discredits every other edge in the view.

use std::collections::{BTreeMap, BTreeSet};

use crate::edges::Reason;
use crate::parse::CallSite;
use crate::typeenv::{Corpus, TypeEnv};

/// A resolved call: the callee's qualified name and the tier that found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub target: String,
    pub reason: Reason,
}

/// What the file being resolved declares itself.
///
/// FR-008 requires same-file resolution to succeed "without consulting other
/// files", so a declaration in the current file wins before batch-wide
/// ambiguity is even considered. Without this, a repository that happens to
/// declare `Store` in both a Rust and a TypeScript file would lose the edges
/// inside *each* of them — the batch ambiguity is real, but it is not the
/// ambiguity the call site has.
#[derive(Debug, Default)]
pub struct LocalScope {
    /// Simple type name -> qualified name, for types declared in this file.
    pub types: BTreeMap<String, String>,
    /// (simple type name, method) -> qualified name, for methods in this file.
    pub methods: BTreeMap<(String, String), String>,
    /// Free function simple name -> qualified name, for this file.
    pub functions: BTreeMap<String, String>,
}

/// Resolve one call site, or return `None` when the source does not determine
/// a unique callee.
pub fn resolve_call(
    call: &CallSite,
    env: &TypeEnv,
    corpus: &Corpus,
    imported_types: &BTreeSet<String>,
    local: &LocalScope,
) -> Option<Resolution> {
    match &call.receiver {
        Some(receiver) => resolve_method_call(call, receiver, env, corpus, imported_types, local),
        None => resolve_free_call(call, corpus, local),
    }
}

fn resolve_method_call(
    call: &CallSite,
    receiver: &str,
    env: &TypeEnv,
    corpus: &Corpus,
    imported_types: &BTreeSet<String>,
    local: &LocalScope,
) -> Option<Resolution> {
    // Tier 1 — the receiver's type is recovered, so the owning type is known.
    if let Some(receiver_type) = env.lookup(&call.scope, receiver) {
        if let Some(target) = method_of(corpus, local, receiver_type, &call.callee) {
            return Some(Resolution {
                target,
                reason: Reason::ReceiverTyped,
            });
        }
        // The receiver's type is known but declares no such method. Falling
        // through to a name-match here would emit an edge that contradicts the
        // type we just recovered, so stop.
        return None;
    }

    // A type-cased receiver is a static call: `Store::open()`.
    if receiver.chars().next().is_some_and(|c| c.is_uppercase()) {
        if let Some(target) = method_of(corpus, local, receiver, &call.callee) {
            return Some(Resolution {
                target,
                reason: Reason::ReceiverTyped,
            });
        }
        return None;
    }

    // Tier 2 — narrow to types this file actually imports.
    let scoped: Vec<String> = imported_types
        .iter()
        .filter_map(|type_name| method_of(corpus, local, type_name, &call.callee))
        .collect();
    if let [only] = scoped.as_slice() {
        return Some(Resolution {
            target: only.clone(),
            reason: Reason::ImportScoped,
        });
    }
    if scoped.len() > 1 {
        // Ambiguous after narrowing: emit nothing (FR-008-CON-1).
        return None;
    }

    // Tier 3 — exactly one type in the batch declares the method.
    let batch: Vec<String> = corpus
        .methods
        .iter()
        .filter(|((_, method), _)| method == &call.callee)
        .flat_map(|(_, targets)| targets.iter().cloned())
        .collect();
    match batch.as_slice() {
        [only] => Some(Resolution {
            target: only.clone(),
            reason: Reason::NameMatch,
        }),
        _ => None,
    }
}

fn resolve_free_call(call: &CallSite, corpus: &Corpus, local: &LocalScope) -> Option<Resolution> {
    if let Some(target) = local.functions.get(&call.callee) {
        return Some(Resolution {
            target: target.clone(),
            reason: Reason::NameMatch,
        });
    }
    let candidates = corpus.functions.get(&call.callee)?;
    let mut iter = candidates.iter();
    let only = iter.next()?;
    if iter.next().is_some() {
        return None;
    }
    Some(Resolution {
        target: only.clone(),
        reason: Reason::NameMatch,
    })
}

/// The method `type_name::method` names — the file's own declaration first,
/// then the batch when exactly one exists.
fn method_of(corpus: &Corpus, local: &LocalScope, type_name: &str, method: &str) -> Option<String> {
    let key = (type_name.to_string(), method.to_string());
    if let Some(target) = local.methods.get(&key) {
        return Some(target.clone());
    }
    let candidates = corpus.methods.get(&key)?;
    let mut iter = candidates.iter();
    let only = iter.next()?;
    if iter.next().is_some() {
        return None;
    }
    Some(only.clone())
}

/// Resolve a type relation to a qualified edge target: the file's own
/// declaration first, then the batch when exactly one exists.
///
/// A relation naming a type the batch does not contain resolves to nothing —
/// the same conservatism as call resolution.
pub fn resolve_type_relation(
    corpus: &Corpus,
    local: &LocalScope,
    target_type: &str,
) -> Option<String> {
    if let Some(target) = local.types.get(target_type) {
        return Some(target.clone());
    }
    let candidates = corpus.types.get(target_type)?;
    let mut iter = candidates.iter();
    let only = iter.next()?;
    if iter.next().is_some() {
        return None;
    }
    Some(only.clone())
}

/// The simple type names a file can reach through its resolved imports, used
/// for tier-2 narrowing.
pub fn imported_types(
    importer: &str,
    import_edges: &BTreeMap<String, BTreeSet<String>>,
    types_by_file: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    // A file always reaches its own declarations.
    if let Some(own) = types_by_file.get(importer) {
        out.extend(own.iter().cloned());
    }
    if let Some(targets) = import_edges.get(importer) {
        for target in targets {
            if let Some(declared) = types_by_file.get(target) {
                out.extend(declared.iter().cloned());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typeenv::{RawBinding, TypeSource};

    fn corpus_with(methods: &[(&str, &str, &str)]) -> Corpus {
        let mut corpus = Corpus::default();
        for (type_name, method, qualified) in methods {
            corpus
                .methods
                .entry((type_name.to_string(), method.to_string()))
                .or_default()
                .insert(qualified.to_string());
            corpus
                .types
                .entry(type_name.to_string())
                .or_default()
                .insert(format!("demo/{type_name}"));
        }
        corpus
    }

    fn call(receiver: Option<&str>, callee: &str) -> CallSite {
        CallSite {
            caller: "demo/caller".to_string(),
            scope: "f@1".to_string(),
            receiver: receiver.map(str::to_string),
            callee: callee.to_string(),
            line: 3,
        }
    }

    fn env_with(name: &str, type_name: &str) -> TypeEnv {
        TypeEnv::build(
            &[RawBinding {
                scope: "f@1".to_string(),
                name: name.to_string(),
                source: TypeSource::Annotation(type_name.to_string()),
            }],
            &BTreeMap::new(),
            &Corpus::default(),
        )
    }

    // TC-044, FR-008-AC-1: a typed receiver resolves, marked receiver-typed.
    #[test]
    fn a_typed_receiver_resolves_to_its_types_method() {
        let corpus = corpus_with(&[
            ("Store", "save", "demo/store.rs::Store::save"),
            ("Repo", "save", "demo/repo.rs::Repo::save"),
        ]);
        let env = env_with("s", "Store");
        let resolved = resolve_call(
            &call(Some("s"), "save"),
            &env,
            &corpus,
            &BTreeSet::new(),
            &LocalScope::default(),
        )
        .expect("resolves");
        assert_eq!(resolved.target, "demo/store.rs::Store::save");
        assert_eq!(resolved.reason, Reason::ReceiverTyped);
    }

    // TC-045, FR-008-AC-2, FR-008-CON-1, StR-002-VC-3: an unrecoverable receiver with several candidates
    // yields nothing.
    #[test]
    fn an_ambiguous_receiver_yields_no_edge() {
        let corpus = corpus_with(&[
            ("Store", "save", "demo/store.rs::Store::save"),
            ("Repo", "save", "demo/repo.rs::Repo::save"),
        ]);
        let env = TypeEnv::default();
        assert_eq!(
            resolve_call(
                &call(Some("x"), "save"),
                &env,
                &corpus,
                &BTreeSet::new(),
                &LocalScope::default()
            ),
            None
        );
    }

    #[test]
    fn a_known_receiver_type_lacking_the_method_does_not_fall_back() {
        // Falling through to a name match here would contradict the type we
        // just recovered.
        let corpus = corpus_with(&[("Repo", "save", "demo/repo.rs::Repo::save")]);
        let env = env_with("s", "Store");
        assert_eq!(
            resolve_call(
                &call(Some("s"), "save"),
                &env,
                &corpus,
                &BTreeSet::new(),
                &LocalScope::default()
            ),
            None
        );
    }

    #[test]
    fn import_scoping_narrows_before_name_matching() {
        let corpus = corpus_with(&[
            ("Store", "save", "demo/store.rs::Store::save"),
            ("Repo", "save", "demo/repo.rs::Repo::save"),
        ]);
        let env = TypeEnv::default();
        let imported: BTreeSet<String> = ["Store".to_string()].into_iter().collect();
        let resolved = resolve_call(
            &call(Some("x"), "save"),
            &env,
            &corpus,
            &imported,
            &LocalScope::default(),
        )
        .expect("resolves");
        assert_eq!(resolved.target, "demo/store.rs::Store::save");
        assert_eq!(resolved.reason, Reason::ImportScoped);
    }

    #[test]
    fn import_scoping_that_stays_ambiguous_yields_nothing() {
        let corpus = corpus_with(&[
            ("Store", "save", "demo/store.rs::Store::save"),
            ("Repo", "save", "demo/repo.rs::Repo::save"),
        ]);
        let env = TypeEnv::default();
        let imported: BTreeSet<String> = ["Store".to_string(), "Repo".to_string()]
            .into_iter()
            .collect();
        assert_eq!(
            resolve_call(
                &call(Some("x"), "save"),
                &env,
                &corpus,
                &imported,
                &LocalScope::default()
            ),
            None
        );
    }

    #[test]
    fn a_unique_method_name_in_the_batch_resolves_by_name_match() {
        let corpus = corpus_with(&[("Store", "upsert", "demo/store.rs::Store::upsert")]);
        let env = TypeEnv::default();
        let resolved = resolve_call(
            &call(Some("x"), "upsert"),
            &env,
            &corpus,
            &BTreeSet::new(),
            &LocalScope::default(),
        )
        .expect("resolves");
        assert_eq!(resolved.reason, Reason::NameMatch);
    }

    #[test]
    fn a_static_call_resolves_through_the_type_name() {
        let corpus = corpus_with(&[("Store", "open", "demo/store.rs::Store::open")]);
        let env = TypeEnv::default();
        let resolved = resolve_call(
            &call(Some("Store"), "open"),
            &env,
            &corpus,
            &BTreeSet::new(),
            &LocalScope::default(),
        )
        .expect("resolves");
        assert_eq!(resolved.reason, Reason::ReceiverTyped);
    }

    #[test]
    fn a_free_call_resolves_only_when_the_name_is_unique() {
        let mut corpus = Corpus::default();
        corpus
            .functions
            .entry("helper".into())
            .or_default()
            .insert("demo/a.rs::helper".into());
        let env = TypeEnv::default();
        assert!(resolve_call(
            &call(None, "helper"),
            &env,
            &corpus,
            &BTreeSet::new(),
            &LocalScope::default()
        )
        .is_some());

        corpus
            .functions
            .entry("helper".into())
            .or_default()
            .insert("demo/b.rs::helper".into());
        assert_eq!(
            resolve_call(
                &call(None, "helper"),
                &env,
                &corpus,
                &BTreeSet::new(),
                &LocalScope::default()
            ),
            None
        );
    }

    #[test]
    fn an_ambiguous_type_relation_resolves_to_nothing() {
        let mut corpus = Corpus::default();
        corpus
            .types
            .entry("Shape".into())
            .or_default()
            .insert("demo/a.rs::Shape".into());
        assert!(resolve_type_relation(&corpus, &LocalScope::default(), "Shape").is_some());

        corpus
            .types
            .entry("Shape".into())
            .or_default()
            .insert("demo/b.rs::Shape".into());
        assert_eq!(
            resolve_type_relation(&corpus, &LocalScope::default(), "Shape"),
            None
        );
    }

    #[test]
    fn imported_types_include_own_and_imported_declarations() {
        let mut import_edges = BTreeMap::new();
        import_edges.insert(
            "a.ts".to_string(),
            ["b.ts".to_string()].into_iter().collect::<BTreeSet<_>>(),
        );
        let mut types_by_file = BTreeMap::new();
        types_by_file.insert(
            "a.ts".to_string(),
            ["Local".to_string()].into_iter().collect::<BTreeSet<_>>(),
        );
        types_by_file.insert(
            "b.ts".to_string(),
            ["Shared".to_string()].into_iter().collect::<BTreeSet<_>>(),
        );
        types_by_file.insert(
            "c.ts".to_string(),
            ["Unreached".to_string()]
                .into_iter()
                .collect::<BTreeSet<_>>(),
        );

        let reachable = imported_types("a.ts", &import_edges, &types_by_file);
        assert!(reachable.contains("Local"));
        assert!(reachable.contains("Shared"));
        assert!(!reachable.contains("Unreached"));
    }
}
