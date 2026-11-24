//! Sealed generic signature checking without claiming checked function bodies.
use sovra::compiler::v2::*;
fn function(s: &str) -> GenericFunctionSyntax {
    parse_generic_function(s, GenericFunctionLimits::DEFAULT).unwrap()
}
fn empty_types() -> CheckedGenericDeclarations {
    let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    check_generic_declarations(&env, &[], GenericDeclarationLimits::DEFAULT).unwrap()
}
#[test]
fn parameter_return_proofs_preserve_function_scope_and_complete_type_context() {
    let echo = parse_generic_alias("type Echo<T>=T;", GenericAliasLimits::DEFAULT).unwrap();
    let boxed =
        parse_generic_record("struct Box<T>{value:T}", GenericRecordLimits::DEFAULT).unwrap();
    let input = [
        GenericTypeDeclaration::Alias(&echo),
        GenericTypeDeclaration::Record(&boxed),
    ];
    let registry = register_generic_types(&input, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&echo], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT).unwrap();
    let f = function("export fn wrap<T>(value:Echo<T>)->Box<T>{return missing(value);}");
    let signatures =
        check_generic_signatures(&types, &[&f], GenericSignatureLimits::DEFAULT).unwrap();
    let proof = &signatures.signatures()[0];
    assert!(proof.syntax().is_exported());
    assert_eq!(proof.parameter_types().len(), 1);
    let ty = &proof.parameter_types()[0];
    assert!(
        matches!(ty.nodes()[ty.root()].kind,ResolvedGenericTypeKind::Parameter(id)if id==f.parameters().find("T").unwrap())
    );
    let GenericSignatureReturn::Explicit(ret) = proof.return_type() else {
        panic!()
    };
    assert!(
        matches!(ret.nodes()[ret.root()].kind,ResolvedGenericTypeKind::Declared{declaration,..}if types.environment().registry().header(declaration).unwrap().name.as_ref()=="Box")
    );
    assert_eq!(proof.types().definitions().len(), 2);
    assert!(proof.syntax().body_source().contains("missing"));
}
#[test]
fn implicit_unit_creates_no_annotation_node_and_explicit_unit_remains_written() {
    let types = empty_types();
    let implicit = function("fn implicit<T>(){}");
    let explicit = function("fn explicit<T>()->Unit{}");
    let signatures = check_generic_signatures(
        &types,
        &[&implicit, &explicit],
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap();
    assert!(matches!(
        signatures.signatures()[0].return_type(),
        GenericSignatureReturn::ImplicitUnit
    ));
    assert!(matches!(
        signatures.signatures()[1].return_type(),
        GenericSignatureReturn::Explicit(_)
    ));
    assert_eq!(signatures.expanded_nodes(), 1);
    assert!(check_generic_signatures(
        &types,
        &[&implicit],
        GenericSignatureLimits {
            max_expanded_nodes: 0,
            max_expanded_edges: 0,
            ..GenericSignatureLimits::DEFAULT
        }
    )
    .is_ok());
}
#[test]
fn duplicate_builtin_entry_and_primitive_scope_conflicts_refuse_without_partial_tables() {
    let types = empty_types();
    let f = function("fn f<T>(){}");
    assert!(check_generic_signatures(&types, &[&f, &f], GenericSignatureLimits::DEFAULT).is_err());
    for source in [
        "fn print<T>(value:T){}",
        "fn main<T>(){}",
        "// λ😀\nfn unused<Int>(){}",
    ] {
        let f = function(source);
        let GenericSignatureError::Definition { span, .. } =
            check_generic_signatures(&types, &[&f], GenericSignatureLimits::DEFAULT).unwrap_err()
        else {
            panic!()
        };
        assert!(span.end <= source.len());
        if source.contains("<Int>") {
            assert_eq!(&source[span.start..span.end], "Int");
            assert_eq!(span.line, 1);
        }
    }
}
#[test]
fn every_unused_signature_checks_unknown_names_arities_and_symbolic_key_bounds() {
    let keys = parse_generic_record(
        "struct Keys<K:std::Key>{value:K}",
        GenericRecordLimits::DEFAULT,
    )
    .unwrap();
    let input = [GenericTypeDeclaration::Record(&keys)];
    let registry = register_generic_types(&input, GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    let types =
        check_generic_declarations(&env, &input, GenericDeclarationLimits::DEFAULT).unwrap();
    let good = function("fn good<T>(value:T)->T{return value;}");
    for source in [
        "fn bad<T>(value:Missing<T>){}",
        "fn bad<T>()->Missing<T>{}",
        "fn bad<T>(value:Keys<Int,T>){}",
        "fn bad<T>(value:Keys<T>){}",
    ] {
        let bad = function(source);
        assert!(matches!(
            check_generic_signatures(&types, &[&good, &bad], GenericSignatureLimits::DEFAULT),
            Err(GenericSignatureError::Annotation { function: 1, .. })
        ));
    }
    let constrained = function("fn key<K:std::Key>(value:Keys<K>)->Keys<K>{return value;}");
    assert!(
        check_generic_signatures(&types, &[&constrained], GenericSignatureLimits::DEFAULT).is_ok()
    );
}
#[test]
fn table_owned_ids_refuse_foreign_tables_and_cloned_proofs_keep_source_owners() {
    let types = empty_types();
    let f = function("fn identity<T>(x:T)->T{return x;}");
    let a = check_generic_signatures(&types, &[&f], GenericSignatureLimits::DEFAULT).unwrap();
    let b = check_generic_signatures(&types, &[&f], GenericSignatureLimits::DEFAULT).unwrap();
    let id = a.find("identity").unwrap();
    assert_eq!(id.ordinal(), 0);
    assert!(b.signature(id).is_none());
    assert!(a.clone().signature(id).is_some());
    assert!(a.find("missing").is_none());
    let proof = a.signature(id).unwrap().clone();
    drop(a);
    drop(types);
    drop(f);
    assert_eq!(proof.syntax().name(), "identity");
    assert_eq!(proof.parameter_types().len(), 1);
}
#[test]
fn exact_cumulative_function_and_annotation_budgets_admit_and_one_under_refuses() {
    let types = empty_types();
    let f = function("fn f<T>(x:[T])->[T]{return x;}");
    let g = function("fn g<U>(x:U)->U{return x;}");
    let input = [&f, &g];
    let checked =
        check_generic_signatures(&types, &input, GenericSignatureLimits::DEFAULT).unwrap();
    let exact = GenericSignatureLimits {
        max_functions: 2,
        max_value_parameters: 2,
        max_metadata_nodes: input
            .iter()
            .map(|f| {
                1 + f.parameters().parameters().len() + f.value_parameters().len() + f.type_nodes()
            })
            .sum(),
        max_source_bytes: input.iter().map(|f| f.retained_source_bytes()).sum(),
        max_name_bytes: input.iter().map(|f| f.name_bytes() + f.name().len()).sum(),
        max_expanded_nodes: checked.expanded_nodes(),
        max_expanded_edges: checked.expanded_edges(),
        max_work: checked.work(),
        ..GenericSignatureLimits::DEFAULT
    };
    assert!(check_generic_signatures(&types, &input, exact).is_ok());
    for limits in [
        GenericSignatureLimits {
            max_functions: 1,
            ..exact
        },
        GenericSignatureLimits {
            max_value_parameters: 1,
            ..exact
        },
        GenericSignatureLimits {
            max_metadata_nodes: exact.max_metadata_nodes - 1,
            ..exact
        },
        GenericSignatureLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        GenericSignatureLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        GenericSignatureLimits {
            max_expanded_nodes: exact.max_expanded_nodes - 1,
            ..exact
        },
        GenericSignatureLimits {
            max_expanded_edges: exact.max_expanded_edges - 1,
            ..exact
        },
        GenericSignatureLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericSignatureLimits {
            expansion: GenericAliasExpansionLimits {
                max_nodes: 1,
                ..exact.expansion
            },
            ..exact
        },
    ] {
        assert!(
            check_generic_signatures(&types, &input, limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn annotation_errors_preserve_function_position_full_source_range_and_fragment_context() {
    let types = empty_types();
    let f = function("// λ😀\nfn f<T>(x:T)\n -> Missing<T> {return x;}");
    let GenericSignatureError::Annotation {
        function,
        position,
        annotation_range,
        error,
    } = check_generic_signatures(&types, &[&f], GenericSignatureLimits::DEFAULT).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(function, 0);
    assert_eq!(position, GenericSignaturePosition::Return);
    assert_eq!(annotation_range.line, 2);
    assert_eq!(
        &f.source()[annotation_range.start..annotation_range.end],
        "Missing<T>"
    );
    assert!(matches!(
        *error,
        GenericAliasExpansionError::Resolution { alias: None, .. }
    ));
    assert!(check_program_source(
        "strict",
        "fn identity<T>(value:T)->T{return value;}fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
