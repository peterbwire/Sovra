//! Exact graph equivalence and bounded normalized specialization-key groundwork.
use sovra::compiler::v2::*;
use std::collections::HashSet;
fn scope() -> GenericParameters {
    parse_generic_parameters("<T>", GenericParameterLimits::DEFAULT).unwrap()
}
fn environment() -> GenericAliasEnvironment {
    let pair =
        parse_generic_record("struct Pair<A,B>{a:A,b:B}", GenericRecordLimits::DEFAULT).unwrap();
    let dup = parse_generic_alias("type Dup<T>=Pair<T,T>;", GenericAliasLimits::DEFAULT).unwrap();
    let first = parse_generic_alias("type First<A,B>=A;", GenericAliasLimits::DEFAULT).unwrap();
    let registry = register_generic_types(
        &[
            GenericTypeDeclaration::Record(&pair),
            GenericTypeDeclaration::Alias(&dup),
            GenericTypeDeclaration::Alias(&first),
        ],
        GenericRegistryLimits::DEFAULT,
    )
    .unwrap();
    prepare_generic_aliases(
        &registry,
        &[&dup, &first],
        GenericAliasEnvironmentLimits::DEFAULT,
    )
    .unwrap()
}
fn expand(
    env: &GenericAliasEnvironment,
    scope: &GenericParameters,
    s: &str,
) -> ExpandedGenericType {
    let ty = parse_generic_type(s, GenericTypeLimits::DEFAULT).unwrap();
    env.expand(&ty, scope, GenericAliasExpansionLimits::DEFAULT)
        .unwrap()
}
fn compare(a: &ExpandedGenericType, b: &ExpandedGenericType) -> bool {
    compare_generic_types(a, b, GenericTypeEquivalenceLimits::DEFAULT)
        .unwrap()
        .equivalent()
}
fn key(a: &ExpandedGenericType) -> GenericTypeKeyResult {
    generic_type_key(a, GenericTypeKeyLimits::DEFAULT).unwrap()
}
#[test]
fn transparent_aliases_and_different_dag_sharing_compare_and_hash_as_the_same_type() {
    let env = environment();
    let scope = scope();
    let shared = expand(&env, &scope, "Dup<[Int]>");
    let repeated = expand(&env, &scope, "Pair<[Int],[Int]>");
    assert!(compare(&shared, &repeated));
    assert_ne!(shared.nodes().len(), repeated.nodes().len());
    let a = key(&shared);
    let b = key(&repeated);
    assert_eq!(a.key(), b.key());
    assert_eq!(a.nodes(), 3);
    assert_eq!(b.nodes(), 3);
    assert_ne!(a.visited_nodes(), b.visited_nodes());
    let mut keys = HashSet::new();
    keys.insert(a.key().clone());
    keys.insert(b.key().clone());
    assert_eq!(keys.len(), 1);
}
#[test]
fn explicit_root_ignores_unused_eager_argument_nodes_but_numeric_types_stay_distinct() {
    let env = environment();
    let scope = scope();
    let aliased = expand(&env, &scope, "First<Int,String>");
    let direct = expand(&env, &scope, "Int");
    assert_eq!(aliased.root(), 0);
    assert_eq!(aliased.nodes().len(), 2);
    assert!(compare(&aliased, &direct));
    assert_eq!(key(&aliased).key(), key(&direct).key());
    assert_eq!(key(&aliased).visited_nodes(), 1);
    let float = expand(&env, &scope, "Float");
    assert!(!compare(&direct, &float));
    assert_ne!(key(&direct).key(), key(&float).key());
}
#[test]
fn equal_local_indices_cannot_hide_different_child_types_or_argument_order() {
    let env = environment();
    let scope = scope();
    let a = expand(&env, &scope, "Pair<Int,String>");
    let b = expand(&env, &scope, "Pair<String,Int>");
    assert_eq!(a.root(), b.root());
    assert!(!compare(&a, &b));
    assert_ne!(key(&a).key(), key(&b).key());
    let a = expand(&env, &scope, "[Int]");
    let b = expand(&env, &scope, "[String]");
    assert_eq!(a.root(), b.root());
    assert!(!compare(&a, &b));
}
#[test]
fn nominal_registry_and_parameter_owners_remain_significant_while_clones_keep_identity() {
    let env = environment();
    let other = environment();
    let scope = scope();
    let scope2 = scope.clone();
    let a = expand(&env, &scope, "Pair<Int,Int>");
    let b = expand(&other, &scope, "Pair<Int,Int>");
    assert!(!compare(&a, &b));
    assert_ne!(key(&a).key(), key(&b).key());
    assert!(compare(&a, &a.clone()));
    let a = expand(&env, &scope, "T");
    let b = expand(&env, &scope2, "T");
    assert!(compare(&a, &b));
    let foreign = super_scope();
    let c = expand(&env, &foreign, "T");
    assert!(!compare(&a, &c));
    assert_ne!(key(&a).key(), key(&c).key());
}
fn super_scope() -> GenericParameters {
    scope()
}
#[test]
fn exact_comparison_pair_edge_and_work_caps_admit_and_one_under_refuses() {
    let env = environment();
    let scope = scope();
    let a = expand(&env, &scope, "Dup<[Int]>");
    let b = expand(&env, &scope, "Pair<[Int],[Int]>");
    let result = compare_generic_types(&a, &b, GenericTypeEquivalenceLimits::DEFAULT).unwrap();
    let exact = GenericTypeEquivalenceLimits {
        max_pairs: result.pairs(),
        max_edges: result.edges(),
        max_work: result.work(),
    };
    assert!(compare_generic_types(&a, &b, exact).unwrap().equivalent());
    for limits in [
        GenericTypeEquivalenceLimits {
            max_pairs: exact.max_pairs - 1,
            ..exact
        },
        GenericTypeEquivalenceLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericTypeEquivalenceLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
    ] {
        assert!(compare_generic_types(&a, &b, limits).is_err());
    }
    assert!(compare_generic_types(
        &a,
        &a,
        GenericTypeEquivalenceLimits {
            max_pairs: 0,
            ..exact
        }
    )
    .is_err());
}
#[test]
fn normalization_visited_canonical_edge_pending_and_work_caps_are_independent() {
    let env = environment();
    let scope = scope();
    let a = expand(&env, &scope, "Pair<[Int],[Int]>");
    let result = key(&a);
    let exact = GenericTypeKeyLimits {
        max_visited_nodes: result.visited_nodes(),
        max_nodes: result.nodes(),
        max_edges: result.edges(),
        max_pending: result.peak_pending(),
        max_work: result.work(),
    };
    assert_eq!(generic_type_key(&a, exact).unwrap().key(), result.key());
    for limits in [
        GenericTypeKeyLimits {
            max_visited_nodes: exact.max_visited_nodes - 1,
            ..exact
        },
        GenericTypeKeyLimits {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        GenericTypeKeyLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericTypeKeyLimits {
            max_pending: exact.max_pending - 1,
            ..exact
        },
        GenericTypeKeyLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
    ] {
        assert!(generic_type_key(&a, limits).is_err(), "{limits:?}");
    }
}
#[test]
fn deep_arrays_are_compared_and_normalized_iteratively_and_keys_keep_no_source_text() {
    let env = environment();
    let scope = scope();
    let source = format!("{}Int{}", "[".repeat(127), "]".repeat(127));
    let a = expand(&env, &scope, &source);
    let b = a.clone();
    assert!(compare(&a, &b));
    let akey = key(&a).key().clone();
    let bkey = key(&b).key().clone();
    drop(a);
    drop(b);
    drop(env);
    drop(scope);
    assert_eq!(akey, bkey);
}
#[test]
fn repeated_normalization_is_deterministic_and_does_not_enable_generic_execution() {
    let env = environment();
    let scope = scope();
    let a = expand(&env, &scope, "Pair<[T],Pair<T,Int>>");
    for _ in 0..16 {
        assert_eq!(key(&a).key(), key(&a).key());
    }
    assert!(check_program_source(
        "strict",
        "fn identity<T>(x:T)->T{return x;}fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
