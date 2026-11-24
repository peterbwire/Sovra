//! Nominal plans preserve source order and link exact layout slots/tags.
use sovra::compiler::v2::*;
fn input(root: &str, functions: &[&str]) -> CheckedGenericLiterals {
    let alias = parse_generic_alias("type Same<T>=T;", GenericAliasLimits::DEFAULT).unwrap();
    let first = parse_generic_alias("type First<A,B,C>=A;", GenericAliasLimits::DEFAULT).unwrap();
    let record_sources = [
        "struct Box<T>{value:T}",
        "struct Empty<T>{}",
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

fn plan(root: &str, functions: &[&str]) -> CheckedGenericConstruction {
    let literals = input(root, functions);
    let layouts = plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap();
    plan_generic_construction(&layouts, GenericConstructionLimits::DEFAULT).unwrap()
}
#[test]
fn record_initializers_keep_source_order_while_targeting_declaration_slots() {
    let checked = plan(
        "fn root(){let pair=Pair::<Int,Float>{right:2,left:1};let value=pair.right;}",
        &[],
    );
    let body = &checked.bodies()[0];
    let record = body
        .expressions()
        .iter()
        .find(|e| matches!(e.kind, GenericConstructionKind::Record { .. }))
        .unwrap();
    let GenericConstructionKind::Record { fields } = &record.kind else {
        panic!()
    };
    assert_eq!(fields.iter().map(|f| f.slot).collect::<Vec<_>>(), [1, 0]);
    assert_eq!(
        fields[0].value.conversion,
        GenericCopyConversion::IntToFloat
    );
    assert_ne!(fields[0].value.layout, fields[0].value.target_layout);
    assert_eq!(fields[1].value.conversion, GenericCopyConversion::Identity);
    assert_eq!(fields[1].value.layout, fields[1].value.target_layout);
    assert!(fields[0].value.expression.ordinal() < fields[1].value.expression.ordinal());
    let field = body
        .expressions()
        .iter()
        .find(|e| matches!(e.kind, GenericConstructionKind::Field { .. }))
        .unwrap();
    let GenericConstructionKind::Field {
        slot,
        receiver_layout,
        ..
    } = field.kind
    else {
        panic!()
    };
    assert_eq!(slot, 1);
    assert_eq!(receiver_layout, record.layout);
    assert_eq!(field.layout, fields[0].value.target_layout);
}
#[test]
fn enum_tags_and_nullary_versus_unit_payloads_are_preserved() {
    let checked=plan("fn root(){Choice::<Int>::End();Choice::<Float>::One(1);Choice::<Int>::Unit(unit::<Int>());}",&["fn unit<T>(){}"]);
    let enums = checked.bodies()[0].expressions();
    assert_eq!(enums.len(), 3);
    assert!(matches!(
        enums[0].kind,
        GenericConstructionKind::Enum {
            tag: 0,
            payload: None
        }
    ));
    let GenericConstructionKind::Enum {
        tag: 1,
        payload: Some(value),
    } = enums[1].kind
    else {
        panic!()
    };
    assert_eq!(value.conversion, GenericCopyConversion::IntToFloat);
    let GenericConstructionKind::Enum {
        tag: 3,
        payload: Some(value),
    } = enums[2].kind
    else {
        panic!()
    };
    assert_eq!(
        checked
            .layouts()
            .layout(value.target_layout)
            .unwrap()
            .kind(),
        &GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Unit)
    );
}
#[test]
fn reordered_match_arms_dispatch_by_tag_and_keep_immutable_payload_destinations() {
    let checked=plan("fn root(value:Choice<Int>){match value {Choice::Unit(u)=>{},Choice::Many(xs)=>{},Choice::End()=>{},Choice::One(x)=>{}}}",&[]);
    let body = &checked.bodies()[0];
    let matched = &body.matches()[0];
    assert_eq!(matched.dispatch, [2, 3, 1, 0]);
    assert_eq!(
        matched.arms.iter().map(|a| a.tag).collect::<Vec<_>>(),
        [3, 2, 0, 1]
    );
    assert!(matched.arms[2].payload.is_none());
    assert!(matched.arms[0].payload.is_some());
    let source = body.body().body().body();
    assert!(matches!(
        source.expression(matched.scrutinee).unwrap().kind,
        GenericCopyExpressionKind::Read(_)
    ));
    for arm in &matched.arms {
        if let Some(payload) = arm.payload {
            assert_eq!(
                body.body().binding_layout(payload.binding),
                Some(payload.layout)
            );
            assert_eq!(
                source.binding(payload.binding).unwrap().kind,
                GenericCopyBindingKind::MatchPayload {
                    scrutinee: matched.scrutinee,
                    variant_index: arm.tag
                }
            );
        }
        assert!(arm.statements.start <= arm.statements.end);
    }
    assert_eq!(
        body.match_statement(matched.statement).unwrap().dispatch,
        matched.dispatch
    );
    assert!(body.match_statement(usize::MAX).is_none());
}
#[test]
fn nested_matches_keep_flat_ranges_and_snapshot_origin_when_parent_local_changes() {
    let checked=plan("fn root(input:Choice<Int>){let mut value=input;match value {Choice::End()=>{},Choice::One(x)=>{value=Choice::<Int>::End();match value {Choice::One(y)=>{},Choice::End()=>{},Choice::Many(xs)=>{},Choice::Unit(u)=>{}}},Choice::Many(xs)=>{},Choice::Unit(u)=>{}}}",&[]);
    let body = &checked.bodies()[0];
    assert_eq!(body.matches().len(), 2);
    let outer = &body.matches()[0];
    let nested = &body.matches()[1];
    assert!(outer.arms[1].statements.contains(&nested.statement));
    assert_ne!(outer.scrutinee, nested.scrutinee);
    assert_eq!(nested.dispatch, [1, 0, 2, 3]);
}
#[test]
fn selected_instances_share_source_ids_but_retain_distinct_concrete_slots() {
    let checked = plan(
        r#"fn root(){make::<Int>(1);make::<String>("s");}"#,
        &["fn make<T>(x:T)->T{return x;let boxed=Box::<T>{value:x};let read=boxed.value;}"],
    );
    let left = &checked.bodies()[1];
    let right = &checked.bodies()[2];
    assert_eq!(
        left.expressions()[0].expression,
        right.expressions()[0].expression
    );
    assert_ne!(left.expressions()[0].layout, right.expressions()[0].layout);
    assert_eq!(left.expressions().len(), 2);
    assert_eq!(right.expressions().len(), 2);
    let id = left.expressions()[0].expression;
    assert!(left.expression(id).is_some());
    assert!(right.expression(id).is_some());
    assert!(checked.bodies()[0].expression(id).is_none());
    let foreign = plan("fn root(){let b=Box::<Int>{value:1};}", &[]);
    assert!(left
        .expression(foreign.bodies()[0].expressions()[0].expression)
        .is_none());
    let clone = checked.clone();
    drop(checked);
    assert_eq!(clone.bodies()[1].expressions().len(), 2);
}
#[test]
fn all_cumulative_dimensions_are_exact_and_refuse_one_under_without_partial_plans() {
    let checked=plan("fn root(value:Choice<Int>){let pair=Pair::<Int,Float>{right:2,left:1};let read=pair.right;let one=Choice::<Int>::One(1);match value {Choice::End()=>{},Choice::One(x)=>{},Choice::Many(xs)=>{},Choice::Unit(u)=>{}}}",&[]);
    let usage = checked.usage();
    let exact = GenericConstructionLimits {
        max_instances: usage.instances,
        max_plans: usage.plans,
        max_fields: usage.fields,
        max_operands: usage.operands,
        max_arms: usage.arms,
        max_dispatch: usage.dispatch,
        max_work: usage.work,
    };
    assert_eq!(
        plan_generic_construction(checked.layouts(), exact)
            .unwrap()
            .usage(),
        usage
    );
    for limits in [
        GenericConstructionLimits {
            max_instances: exact.max_instances - 1,
            ..exact
        },
        GenericConstructionLimits {
            max_plans: exact.max_plans - 1,
            ..exact
        },
        GenericConstructionLimits {
            max_fields: exact.max_fields - 1,
            ..exact
        },
        GenericConstructionLimits {
            max_operands: exact.max_operands - 1,
            ..exact
        },
        GenericConstructionLimits {
            max_arms: exact.max_arms - 1,
            ..exact
        },
        GenericConstructionLimits {
            max_dispatch: exact.max_dispatch - 1,
            ..exact
        },
        GenericConstructionLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
    ] {
        assert!(
            plan_generic_construction(checked.layouts(), limits).is_err(),
            "{limits:?}"
        );
    }
}
#[test]
fn non_nominal_expressions_and_empty_bodies_need_no_nominal_plan_slots() {
    let checked = plan("fn root(){let value=1;}", &[]);
    assert!(checked.bodies()[0].expressions().is_empty());
    let literal = checked.layouts().literals().bodies()[0].literals()[0].expression;
    assert!(checked.bodies()[0].expression(literal).is_none());
    let limits = GenericConstructionLimits {
        max_instances: 1,
        max_plans: 0,
        max_fields: 0,
        max_operands: 0,
        max_arms: 0,
        max_dispatch: 0,
        max_work: 1,
    };
    let empty = plan("fn root(){}", &[]);
    assert_eq!(
        plan_generic_construction(empty.layouts(), limits)
            .unwrap()
            .usage()
            .work,
        1
    );
    assert!(check_program_source(
        "strict",
        "fn id<T>(x:T)->T{return x;}fn main(){id::<Int>(1);}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}

#[test]
fn explicit_empty_records_and_nullary_variants_need_no_operand_or_field_slots() {
    let checked = plan(
        "fn root(){let empty=Empty::<Int>{};Choice::<Int>::End();}",
        &[],
    );
    let planned = plan_generic_construction(
        checked.layouts(),
        GenericConstructionLimits {
            max_fields: 0,
            max_operands: 0,
            max_arms: 0,
            max_dispatch: 0,
            ..GenericConstructionLimits::DEFAULT
        },
    )
    .unwrap();
    assert_eq!(planned.usage().plans, 2);
    assert!(
        matches!(&planned.bodies()[0].expressions()[0].kind,GenericConstructionKind::Record{fields} if fields.is_empty())
    );
    assert!(matches!(
        planned.bodies()[0].expressions()[1].kind,
        GenericConstructionKind::Enum { payload: None, .. }
    ));
}
