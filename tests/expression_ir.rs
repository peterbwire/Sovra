//! Checked expression register emission; no function/runtime execution claims.
use sovra::compiler::{
    checked::CoercionKind,
    parser::Parser,
    semantic::Type,
    v2::{
        check_enum_types, CallableTarget, CheckedEnumTypes, EnumSource, EnumTypeLimits,
        ExpressionIrError as Error, ExpressionIrLimits as Limits, ExpressionLiteral as Literal,
        ExpressionOperation as Op, MappingLimits, ScopedCallableLimits,
    },
};
fn env() -> CheckedEnumTypes {
    check_enum_types(
        "owner",
        "types",
        &Parser::new().parse_source("struct Item{x:Float}").unwrap(),
        &[EnumSource {
            label: "enum",
            source: "enum E{V(Float),Rows([[Float]]),Empty}",
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
#[test]
fn constructors_use_checked_targets_and_ordered_conversion_registers() {
    let checked = env()
        .check_scoped_callable_function_source(
            "f",
            "fn f()->E{return E::V(1);}",
            ScopedCallableLimits::DEFAULT,
        )
        .unwrap();
    let mapped = checked.map_for_lowering(MappingLimits::DEFAULT).unwrap();
    let ir = mapped.emit_expression_ir(Limits::DEFAULT).unwrap();
    assert_eq!(ir.instructions().len(), 3);
    assert_eq!(ir.values().len(), 3);
    assert_eq!(ir.roots().len(), 1);
    assert!(matches!(
        ir.instructions()[0].operation,
        Op::Literal(Literal::Int("1"))
    ));
    let Op::Convert { value, context } = ir.instructions()[1].operation else {
        panic!("expected widening")
    };
    assert_eq!(ir.value(value).unwrap().ty, &Type::Int);
    assert_eq!(ir.values()[1].ty, &Type::Float);
    assert_eq!(
        checked.typed().callable().evidence().contexts()[context].kind,
        CoercionKind::Argument
    );
    let Op::Call {
        target, arguments, ..
    } = &ir.instructions()[2].operation
    else {
        panic!("expected constructor")
    };
    assert!(matches!(target, CallableTarget::Constructor { .. }));
    assert_eq!(arguments, &vec![ir.values()[1].id]);
    assert_eq!(ir.values()[2].ty, &Type::Named("E".into()));
    assert_eq!(ir.deferred_contexts().len(), 1);
    ir.validate(Limits::DEFAULT.max_validation_steps).unwrap();
}
#[test]
fn each_operand_conversion_precedes_evaluation_of_the_next_sibling() {
    let checked = env()
        .check_scoped_callable_function_source(
            "f",
            "fn f()->Float{let a=[1,f()];return a[0];}",
            ScopedCallableLimits::DEFAULT,
        )
        .unwrap();
    let mapped = checked.map_for_lowering(MappingLimits::DEFAULT).unwrap();
    let ir = mapped.emit_expression_ir(Limits::DEFAULT).unwrap();
    let converted = ir
        .instructions()
        .iter()
        .position(|i| matches!(i.operation, Op::Convert { .. }))
        .unwrap();
    let recursive = ir
        .instructions()
        .iter()
        .position(|i| {
            matches!(
                i.operation,
                Op::Call {
                    target: CallableTarget::SelfFunction,
                    ..
                }
            )
        })
        .unwrap();
    assert!(converted < recursive);
    assert!(matches!(
        ir.instructions()[0].operation,
        Op::Literal(Literal::Int("1"))
    ));
    assert!(ir
        .instructions()
        .iter()
        .any(|i| matches!(i.operation, Op::Array(_))));
    for instruction in ir.instructions() {
        assert!(ir.value(instruction.result).is_some());
    }
}
#[test]
fn nested_arrays_preserve_scalar_recursive_and_identity_contexts() {
    let checked = env()
        .check_scoped_callable_function_source(
            "f",
            "fn f()->E{return E::Rows([[1,2.0],[3,4]]);}",
            ScopedCallableLimits::DEFAULT,
        )
        .unwrap();
    let mapped = checked.map_for_lowering(MappingLimits::DEFAULT).unwrap();
    let ir = mapped.emit_expression_ir(Limits::DEFAULT).unwrap();
    let contexts = checked.typed().callable().evidence().contexts();
    assert!(ir.instructions().iter().any(|i| match i.operation {
        Op::Convert { context, .. } =>
            contexts[context].actual == Type::Array(Box::new(Type::Int))
                && contexts[context].expected == Type::Array(Box::new(Type::Float)),
        _ => false,
    }));
    assert_eq!(
        ir.instructions()
            .iter()
            .filter(|i| matches!(i.operation, Op::Array(_)))
            .count(),
        3
    );
    assert_eq!(
        ir.instructions()
            .iter()
            .filter(|i| matches!(i.operation, Op::Convert { .. }))
            .count()
            + ir.deferred_contexts().len(),
        contexts.len()
    );
}
#[test]
fn records_fields_binary_and_index_operations_use_checked_read_bindings() {
    let checked = env()
        .check_scoped_callable_function_source(
            "f",
            "fn f(n:Int)->Float{let item=Item{x:n};let a=[item.x];return a[0]+n;}",
            ScopedCallableLimits::DEFAULT,
        )
        .unwrap();
    let mapped = checked.map_for_lowering(MappingLimits::DEFAULT).unwrap();
    let ir = mapped.emit_expression_ir(Limits::DEFAULT).unwrap();
    assert!(ir
        .instructions()
        .iter()
        .any(|i| matches!(i.operation, Op::Record(_))));
    assert!(ir
        .instructions()
        .iter()
        .any(|i| matches!(i.operation, Op::Field { field: "x", .. })));
    assert!(ir
        .instructions()
        .iter()
        .any(|i| matches!(i.operation, Op::Index { .. })));
    assert!(ir
        .instructions()
        .iter()
        .any(|i| matches!(i.operation, Op::Binary { operator: "+", .. })));
    for instruction in ir.instructions() {
        if let Op::Read { usage, declaration } = instruction.operation {
            assert_eq!(
                checked.bindings().usage(usage).unwrap().declaration,
                declaration
            );
        }
    }
}
#[test]
fn branch_loop_and_match_roots_remain_separate_from_function_control_flow() {
    let checked=env().check_scoped_callable_function_source("f","fn f(input:E){if true{print(1);}else{print(2);}while false{print(3);}match input{E::V(n)=>{print(n);},E::Rows(r)=>{},E::Empty()=>{}}}",ScopedCallableLimits::DEFAULT).unwrap();
    let mapped = checked.map_for_lowering(MappingLimits::DEFAULT).unwrap();
    let ir = mapped.emit_expression_ir(Limits::DEFAULT).unwrap();
    assert_eq!(ir.roots().len(), 6); // two conditions and four effect-expression roots
    assert_eq!(
        ir.instructions()
            .iter()
            .filter(|i| matches!(i.operation, Op::Call { .. }))
            .count(),
        4
    );
    assert!(ir
        .values()
        .iter()
        .all(|v| v.expression.is_some() || v.context.is_some()));
}
#[test]
fn return_and_assignment_contexts_are_explicitly_deferred() {
    let checked = env()
        .check_scoped_callable_function_source(
            "f",
            "fn f()->Float{let mut a=[1.0];a[0]=2;return 3;}",
            ScopedCallableLimits::DEFAULT,
        )
        .unwrap();
    let mapped = checked.map_for_lowering(MappingLimits::DEFAULT).unwrap();
    let ir = mapped.emit_expression_ir(Limits::DEFAULT).unwrap();
    let kinds: Vec<_> = ir
        .deferred_contexts()
        .iter()
        .map(|c| checked.typed().callable().evidence().contexts()[*c].kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            CoercionKind::Initialization,
            CoercionKind::IndexedAssignment,
            CoercionKind::Return
        ]
    );
}
#[test]
fn limits_are_exact_zero_is_real_and_refusal_never_falls_back() {
    let checked = env()
        .check_scoped_callable_function_source(
            "f",
            "fn f()->E{return E::V(1);}",
            ScopedCallableLimits::DEFAULT,
        )
        .unwrap();
    let mapped = checked.map_for_lowering(MappingLimits::DEFAULT).unwrap();
    let exact = Limits {
        max_instructions: 3,
        max_values: 3,
        max_operands: 2,
        max_roots: 1,
        max_deferred_contexts: 1,
        max_validation_steps: 7,
    };
    assert!(mapped.emit_expression_ir(exact).is_ok());
    for limits in [
        Limits {
            max_instructions: 2,
            ..exact
        },
        Limits {
            max_values: 2,
            ..exact
        },
        Limits {
            max_operands: 1,
            ..exact
        },
        Limits {
            max_roots: 0,
            ..exact
        },
        Limits {
            max_deferred_contexts: 0,
            ..exact
        },
        Limits {
            max_validation_steps: 6,
            ..exact
        },
    ] {
        assert!(matches!(
            mapped.emit_expression_ir(limits),
            Err(Error::Limit(_))
        ));
        assert!(mapped.emit_expression_ir(exact).is_ok());
    }
    let empty = env()
        .check_scoped_callable_function_source("f", "fn f(){}", ScopedCallableLimits::DEFAULT)
        .unwrap();
    let empty_map = empty.map_for_lowering(MappingLimits::DEFAULT).unwrap();
    assert!(empty_map
        .emit_expression_ir(Limits {
            max_instructions: 0,
            max_values: 0,
            max_operands: 0,
            max_roots: 0,
            max_deferred_contexts: 0,
            max_validation_steps: 0
        })
        .is_ok());
}
#[test]
fn graph_register_ids_refuse_other_graphs_and_clones_preserve_identity() {
    let checked = env()
        .check_scoped_callable_function_source(
            "f",
            "fn f()->E{return E::Empty();}",
            ScopedCallableLimits::DEFAULT,
        )
        .unwrap();
    let mapped = checked.map_for_lowering(MappingLimits::DEFAULT).unwrap();
    let a = mapped.emit_expression_ir(Limits::DEFAULT).unwrap();
    let b = mapped.emit_expression_ir(Limits::DEFAULT).unwrap();
    let id = a.values()[0].id;
    assert!(b.value(id).is_none());
    assert!(a.clone().value(id).is_some());
}
