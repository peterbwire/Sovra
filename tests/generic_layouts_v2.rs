//! Finite backend-neutral layout closure, recursive nominal edges and shared budgets.
use sovra::compiler::v2::*;
fn input(root: &str, functions: &[&str]) -> CheckedGenericLiterals {
    let alias = parse_generic_alias("type Same<T>=T;", GenericAliasLimits::DEFAULT).unwrap();
    let first = parse_generic_alias("type First<A,B,C>=A;", GenericAliasLimits::DEFAULT).unwrap();
    let record_sources = [
        "struct Box<T>{value:T}",
        "struct Phantom<T>{value:First<T,T,T>}",
        "struct Other<T>{value:T}",
        "struct Pair<A,B>{left:A,right:B}",
        "struct Entry<K:std::Key,V>{key:K,value:V}",
        "struct Node<T>{next:[Node<T>],value:T}",
        "struct Recursive<T>{next:Recursive<T>}",
        "struct Grow<T>{next:Grow<[T]>}",
    ];
    let records: Vec<_> = record_sources
        .iter()
        .map(|s| parse_generic_record(s, GenericRecordLimits::DEFAULT).unwrap())
        .collect();
    let enum_sources = [
        "enum Choice<T>{End,One(T),Many([T]),Unit(Unit)}",
        "enum Tree<T>{End,Value(T),Next([Tree<T>])}",
    ];
    let enums: Vec<_> = enum_sources
        .iter()
        .map(|s| parse_generic_enum(s, GenericEnumLimits::DEFAULT).unwrap())
        .collect();
    let mut definitions = vec![
        GenericTypeDeclaration::Alias(&alias),
        GenericTypeDeclaration::Alias(&first),
    ];
    definitions.extend(records.iter().map(GenericTypeDeclaration::Record));
    definitions.extend(enums.iter().map(GenericTypeDeclaration::Enum));
    let registry = register_generic_types(&definitions, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(
        &registry,
        &[&alias, &first],
        GenericAliasEnvironmentLimits::DEFAULT,
    )
    .unwrap();
    let types =
        check_generic_declarations(&env, &definitions, GenericDeclarationLimits::DEFAULT).unwrap();
    let mut syntax = vec![parse_specialization_root(root, GenericFunctionLimits::DEFAULT).unwrap()];
    syntax.extend(
        functions
            .iter()
            .map(|s| parse_generic_function(s, GenericFunctionLimits::DEFAULT).unwrap()),
    );
    let signatures = check_generic_signatures(
        &types,
        &syntax.iter().collect::<Vec<_>>(),
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap();
    let bodies = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    let graph = discover_generic_specializations(
        &bodies,
        signatures.find("root").unwrap(),
        GenericSpecializationLimits::DEFAULT,
    )
    .unwrap();
    let concrete =
        recheck_generic_concrete_bodies(&graph, GenericConcreteBodyLimits::DEFAULT).unwrap();
    decode_generic_literals(&concrete, GenericLiteralLimits::DEFAULT).unwrap()
}
fn plan(input: &CheckedGenericLiterals) -> CheckedGenericLayouts {
    plan_generic_layouts(input, GenericLayoutLimits::DEFAULT).unwrap()
}
fn primitive(layouts: &CheckedGenericLayouts, index: usize, expected: GenericPrimitive) {
    assert_eq!(
        layouts.layout(index).unwrap().kind(),
        &GenericRuntimeLayoutKind::Primitive(expected)
    );
}
#[test]
fn records_enums_and_nested_arrays_keep_declaration_order_and_unit_shape() {
    let planned = plan(&input(
        "fn root(pair:Pair<Int,String>,choice:Choice<Int>){}",
        &[],
    ));
    let root = &planned.bodies()[0];
    let GenericRuntimeLayoutKind::Record {
        arguments, fields, ..
    } = planned.layout(root.parameter_layouts()[0]).unwrap().kind()
    else {
        panic!()
    };
    assert_eq!(arguments, fields);
    primitive(&planned, fields[0], GenericPrimitive::Int);
    primitive(&planned, fields[1], GenericPrimitive::String);
    let GenericRuntimeLayoutKind::Enum {
        arguments,
        variants,
        ..
    } = planned.layout(root.parameter_layouts()[1]).unwrap().kind()
    else {
        panic!()
    };
    assert_eq!(variants.len(), 4);
    assert_eq!(variants[0], None);
    assert_eq!(variants[1], Some(arguments[0]));
    let GenericRuntimeLayoutKind::Array { element } =
        planned.layout(variants[2].unwrap()).unwrap().kind()
    else {
        panic!()
    };
    primitive(&planned, *element, GenericPrimitive::Int);
    assert_eq!(variants[3], Some(root.return_layout()));
    primitive(&planned, root.return_layout(), GenericPrimitive::Unit);
}
#[test]
fn aliases_reuse_layouts_but_nominal_heads_and_argument_order_stay_distinct() {
    let planned=plan(&input("fn root(a:Box<Int>,b:Same<Box<Int>>,c:Other<Int>,left:Pair<Int,String>,right:Pair<String,Int>){}",&[]));
    let parameters = planned.bodies()[0].parameter_layouts();
    assert_eq!(parameters[0], parameters[1]);
    assert_ne!(parameters[0], parameters[2]);
    assert_ne!(parameters[3], parameters[4]);
    let GenericRuntimeLayoutKind::Record { declaration: a, .. } =
        planned.layout(parameters[0]).unwrap().kind()
    else {
        panic!()
    };
    let GenericRuntimeLayoutKind::Record { declaration: b, .. } =
        planned.layout(parameters[2]).unwrap().kind()
    else {
        panic!()
    };
    assert_ne!(a, b);
}
#[test]
fn recursive_records_and_enum_payloads_close_as_finite_graph_edges() {
    let input = input(
        "fn root(node:Node<Int>,tree:Tree<String>,direct:Recursive<Bool>){}",
        &[],
    );
    let planned = plan_generic_layouts(
        &input,
        GenericLayoutLimits {
            max_depth: 3,
            ..GenericLayoutLimits::DEFAULT
        },
    )
    .unwrap();
    let parameters = planned.bodies()[0].parameter_layouts();
    let GenericRuntimeLayoutKind::Record { fields, .. } =
        planned.layout(parameters[0]).unwrap().kind()
    else {
        panic!()
    };
    let GenericRuntimeLayoutKind::Array { element } = planned.layout(fields[0]).unwrap().kind()
    else {
        panic!()
    };
    assert_eq!(*element, parameters[0]);
    let GenericRuntimeLayoutKind::Enum { variants, .. } =
        planned.layout(parameters[1]).unwrap().kind()
    else {
        panic!()
    };
    let GenericRuntimeLayoutKind::Array { element } =
        planned.layout(variants[2].unwrap()).unwrap().kind()
    else {
        panic!()
    };
    assert_eq!(*element, parameters[1]);
    let GenericRuntimeLayoutKind::Record { fields, .. } =
        planned.layout(parameters[2]).unwrap().kind()
    else {
        panic!()
    };
    assert_eq!(fields[0], parameters[2]);
    assert!(planned.layouts().len() < 20);
}
#[test]
fn expanding_nominal_recursion_refuses_on_combined_instantiations_or_depth() {
    let input = input("fn root(value:Grow<Int>){}", &[]);
    assert!(matches!(
        plan_generic_layouts(
            &input,
            GenericLayoutLimits {
                max_instantiations: 4,
                ..GenericLayoutLimits::DEFAULT
            }
        ),
        Err(GenericLayoutError::Limit("layout instantiations"))
    ));
    assert!(plan_generic_layouts(
        &input,
        GenericLayoutLimits {
            max_depth: 2,
            ..GenericLayoutLimits::DEFAULT
        }
    )
    .is_err());
}
#[test]
fn function_and_nominal_instantiations_share_the_approved_budget_including_phantoms() {
    let input = input(
        "fn root(){unit::<Box<Int>>();unit::<Box<String>>();}",
        &["fn unit<T>(){}"],
    );
    let planned = plan(&input);
    assert_eq!(planned.usage().instantiations, 5);
    assert_ne!(
        planned.bodies()[1].type_arguments()[0],
        planned.bodies()[2].type_arguments()[0]
    );
    assert_eq!(
        planned.bodies()[1].return_layout(),
        planned.bodies()[2].return_layout()
    );
    assert!(plan_generic_layouts(
        &input,
        GenericLayoutLimits {
            max_instantiations: 4,
            ..GenericLayoutLimits::DEFAULT
        }
    )
    .is_err());
    assert_eq!(GenericLayoutLimits::DEFAULT.max_instantiations, 1024);
    assert_eq!(GenericLayoutLimits::PACKAGE.max_instantiations, 4096);
}
#[test]
fn source_fact_mappings_keep_arenas_literals_and_concrete_key_constraints() {
    let input=input("fn root(){let pair=Pair::<Int,String>{right:\"text\",left:1};let entry=Entry::<String,Bool>{key:pair.right,value:true};}",&[]);
    let planned = plan(&input);
    let body = &planned.bodies()[0];
    for binding in body.body().body().bindings() {
        assert!(planned
            .layout(body.binding_layout(binding.id).unwrap())
            .is_some());
    }
    for expression in body.body().body().expressions() {
        assert!(planned
            .layout(body.expression_layout(expression.id).unwrap())
            .is_some());
    }
    for literal in planned.literals().bodies()[0].literals() {
        primitive(
            &planned,
            body.expression_layout(literal.expression).unwrap(),
            literal.value.primitive(),
        );
    }
    let foreign = super_foreign();
    assert!(body
        .expression_layout(foreign.literals().bodies()[0].literals()[0].expression)
        .is_none());
    let cloned = planned.clone();
    drop(planned);
    drop(input);
    assert!(!cloned.layouts().is_empty());
}
fn super_foreign() -> CheckedGenericLayouts {
    plan(&input("fn root(){let one=1;}", &[]))
}
#[test]
fn cumulative_layout_dimensions_and_independent_profiles_refuse_one_under() {
    let input = input("fn root(value:Pair<Box<Int>,Choice<[String]>>){}", &[]);
    let usage = plan(&input).usage();
    let exact = GenericLayoutLimits {
        max_instantiations: usage.instantiations,
        max_layouts: usage.layouts,
        max_members: usage.members,
        max_edges: usage.edges,
        max_references: usage.references,
        max_generated_nodes: usage.generated_nodes,
        max_generated_edges: usage.generated_edges,
        max_key_nodes: usage.key_nodes,
        max_key_edges: usage.key_edges,
        max_work: usage.work,
        ..GenericLayoutLimits::DEFAULT
    };
    assert_eq!(plan_generic_layouts(&input, exact).unwrap().usage(), usage);
    for limits in [
        GenericLayoutLimits {
            max_instantiations: exact.max_instantiations - 1,
            ..exact
        },
        GenericLayoutLimits {
            max_layouts: exact.max_layouts - 1,
            ..exact
        },
        GenericLayoutLimits {
            max_members: exact.max_members - 1,
            ..exact
        },
        GenericLayoutLimits {
            max_edges: exact.max_edges - 1,
            ..exact
        },
        GenericLayoutLimits {
            max_references: exact.max_references - 1,
            ..exact
        },
        GenericLayoutLimits {
            max_generated_nodes: exact.max_generated_nodes - 1,
            ..exact
        },
        GenericLayoutLimits {
            max_generated_edges: exact.max_generated_edges - 1,
            ..exact
        },
        GenericLayoutLimits {
            max_key_nodes: exact.max_key_nodes - 1,
            ..exact
        },
        GenericLayoutLimits {
            max_key_edges: exact.max_key_edges - 1,
            ..exact
        },
        GenericLayoutLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericLayoutLimits {
            max_nodes_per_type: 1,
            ..exact
        },
        GenericLayoutLimits {
            max_edges_per_type: 0,
            ..exact
        },
        GenericLayoutLimits {
            expansion: GenericAliasExpansionLimits {
                max_work: 0,
                ..exact.expansion
            },
            ..exact
        },
        GenericLayoutLimits {
            keys: GenericTypeKeyLimits {
                max_work: 0,
                ..exact.keys
            },
            ..exact
        },
        GenericLayoutLimits {
            max_depth: 0,
            ..exact
        },
        GenericLayoutLimits {
            max_depth: 129,
            ..exact
        },
    ] {
        assert!(plan_generic_layouts(&input, limits).is_err(), "{limits:?}");
    }
}
#[test]
fn empty_unit_layout_requires_no_generated_graphs_and_layout_ids_are_deterministic() {
    let input = input("fn root(){}", &[]);
    let limits = GenericLayoutLimits {
        max_instantiations: 1,
        max_layouts: 1,
        max_members: 0,
        max_edges: 0,
        max_references: 1,
        max_generated_nodes: 0,
        max_generated_edges: 0,
        max_key_nodes: 0,
        max_key_edges: 0,
        ..GenericLayoutLimits::DEFAULT
    };
    let planned = plan_generic_layouts(&input, limits).unwrap();
    assert_eq!(planned.layouts().len(), 1);
    primitive(
        &planned,
        planned.bodies()[0].return_layout(),
        GenericPrimitive::Unit,
    );
    assert_eq!(
        planned.usage(),
        plan_generic_layouts(&input, limits).unwrap().usage()
    );
    assert!(planned.layout(usize::MAX).is_none());
    assert!(check_program_source(
        "strict",
        "fn id<T>(x:T)->T{return x;}fn main(){id::<Int>(1);}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}

#[test]
fn member_template_arenas_are_admitted_even_when_aliases_discard_most_nodes() {
    let input = input("fn root(value:Phantom<Int>){}", &[]);
    assert!(plan_generic_layouts(
        &input,
        GenericLayoutLimits {
            max_nodes_per_type: 3,
            ..GenericLayoutLimits::DEFAULT
        }
    )
    .is_ok());
    assert!(matches!(
        plan_generic_layouts(
            &input,
            GenericLayoutLimits {
                max_nodes_per_type: 2,
                ..GenericLayoutLimits::DEFAULT
            }
        ),
        Err(GenericLayoutError::Limit("layout member type arena"))
    ));
}
