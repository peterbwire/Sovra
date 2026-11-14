//! Function control-flow emission and bounded definite-availability checking.
use sovra::compiler::{
    parser::Parser,
    v2::{
        check_enum_types, CheckedEnumTypes, EnumSource, EnumTypeLimits, ExpressionIrLimits,
        FunctionInstruction as Op, FunctionIrError as Error, FunctionIrLimits as Limits,
        MappingLimits, ScopedCallableLimits,
    },
};
fn env() -> CheckedEnumTypes {
    check_enum_types(
        "o",
        "c",
        &Parser::new().parse_source("").unwrap(),
        &[EnumSource {
            label: "enum",
            source: "enum E{V(Int),Empty}",
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
macro_rules! ir {
    ($checked:ident,$map:ident,$expr:ident,$ir:ident,$source:expr) => {
        let $checked = env()
            .check_scoped_callable_function_source("f", $source, ScopedCallableLimits::DEFAULT)
            .unwrap();
        let $map = $checked.map_for_lowering(MappingLimits::DEFAULT).unwrap();
        let $expr = $map
            .emit_expression_ir(ExpressionIrLimits::DEFAULT)
            .unwrap();
        let $ir = $expr.emit_function_ir(Limits::DEFAULT).unwrap();
    };
}
#[test]
fn locals_stores_and_returns_have_explicit_scope_cleanup_and_boundary_conversions() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(n:Int)->Float{let mut x=n;x=x+1;return x;}"
    );
    assert!(matches!(ir.instructions()[0], Op::EnterScope(0)));
    assert!(ir
        .instructions()
        .iter()
        .any(|i| matches!(i, Op::Parameter { index: 0, .. })));
    assert!(ir
        .instructions()
        .iter()
        .any(|i| matches!(i, Op::Bind { .. })));
    assert!(ir
        .instructions()
        .iter()
        .any(|i| matches!(i, Op::Store { .. })));
    assert!(ir
        .instructions()
        .iter()
        .any(|i| matches!(i, Op::Return(Some(_)))));
    ir.validate(Limits::DEFAULT).unwrap();
}
#[test]
fn while_and_if_emit_real_branches_back_edges_and_balanced_scopes() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(n:Int)->Int{let mut x=0;while(x<n){x=x+1;}if x>0{return x;}else{return 0;}}"
    );
    assert!(
        ir.instructions()
            .iter()
            .filter(|i| matches!(i, Op::BranchFalse { .. }))
            .count()
            >= 2
    );
    assert!(ir
        .instructions()
        .iter()
        .enumerate()
        .any(|(pc, i)| matches!(i,Op::Jump(target) if *target<pc)));
    assert!(ir.instructions().iter().all(|i| match i {
        Op::Jump(t) | Op::BranchFalse { target: t, .. } => *t < ir.instructions().len(),
        _ => true,
    }));
}
#[test]
fn matches_capture_once_dispatch_exhaustively_and_bind_payloads_with_tag_guards() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(input:E)->Int{match input{E::Empty()=>{return 0;},E::V(n)=>{return n;}}}"
    );
    assert_eq!(
        ir.instructions()
            .iter()
            .filter(|i| matches!(i, Op::SnapshotMatch { .. }))
            .count(),
        1
    );
    let arms = ir
        .instructions()
        .iter()
        .find_map(|i| match i {
            Op::Switch { arms, .. } => Some(arms),
            _ => None,
        })
        .unwrap();
    assert_eq!(arms.iter().map(|a| a.0).collect::<Vec<_>>(), vec![1, 0]);
    assert_eq!(
        ir.instructions()
            .iter()
            .filter(|i| matches!(i, Op::Payload { .. }))
            .count(),
        1
    );
    ir.validate(Limits::DEFAULT).unwrap();
}
#[test]
fn indexed_assignment_evaluates_base_index_and_rhs_before_store_and_releases_them() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f()->Float{let mut a=[1.0];a[0]=2;return a[0];}"
    );
    let pc = ir
        .instructions()
        .iter()
        .position(|i| matches!(i, Op::StoreIndex { .. }))
        .unwrap();
    assert!(
        ir.instructions()[..pc]
            .iter()
            .filter(|i| matches!(i, Op::Evaluate { .. }))
            .count()
            >= 4
    );
    assert!(ir.instructions()[pc + 1..pc + 4]
        .iter()
        .all(|i| matches!(i, Op::Release { .. })));
}
#[test]
fn unit_completion_discards_effect_roots_and_cleans_match_snapshots() {
    ir!(
        checked,
        map,
        expr,
        ir,
        "fn f(input:E){match input{E::V(n)=>{print(n);},E::Empty()=>{print(0);}}print(1);}"
    );
    assert_eq!(
        ir.instructions()
            .iter()
            .filter(|i| matches!(i, Op::ReleaseMatch(0)))
            .count(),
        2
    );
    assert!(matches!(ir.instructions().last(), Some(Op::End)));
    ir.validate(Limits::DEFAULT).unwrap();
}
#[test]
fn shadowing_and_unreachable_checked_source_preserve_distinct_declarations() {
    ir!(checked,map,expr,ir,"fn f(n:Int)->Int{if true{let n=1;print(n);}else{let n=2;print(n);}return n;let x=1;print(x);}");
    let slots: Vec<_> = ir
        .instructions()
        .iter()
        .filter_map(|i| match i {
            Op::Bind { declaration, .. } => Some(*declaration),
            _ => None,
        })
        .collect();
    assert_eq!(slots.len(), 3);
    assert_ne!(slots[0], slots[1]);
}
#[test]
fn limits_refuse_without_partial_ir_and_validation_work_is_exact() {
    ir!(checked, map, expr, ir, "fn f(){}");
    assert_eq!(ir.instructions().len(), 3);
    let report = ir.validate(Limits::DEFAULT).unwrap();
    let exact = Limits {
        max_instructions: 3,
        max_edges: 0,
        max_scope_frames: 1,
        max_validation_steps: report.steps,
        max_state_items: report.state_items,
        max_expression_validation_steps: 0,
    };
    assert!(expr.emit_function_ir(exact).is_ok());
    for limits in [
        Limits {
            max_instructions: 2,
            ..exact
        },
        Limits {
            max_scope_frames: 0,
            ..exact
        },
        Limits {
            max_validation_steps: report.steps - 1,
            ..exact
        },
        Limits {
            max_state_items: report.state_items - 1,
            ..exact
        },
    ] {
        assert!(matches!(
            expr.emit_function_ir(limits),
            Err(Error::Limit(_))
        ));
        assert!(expr.emit_function_ir(exact).is_ok());
    }
}
#[test]
fn multiple_matches_keep_snapshots_and_payload_slots_separate() {
    ir!(checked,map,expr,ir,"fn f(input:E){match input{E::V(n)=>{print(n);},E::Empty()=>{}}match input{E::Empty()=>{},E::V(n)=>{print(n);}}}");
    let indices: Vec<_> = ir
        .instructions()
        .iter()
        .filter_map(|i| match i {
            Op::SnapshotMatch { index, .. } => Some(*index),
            _ => None,
        })
        .collect();
    assert_eq!(indices, vec![0, 1]);
}

#[test]
fn branch_edge_and_active_scope_allowances_refuse_one_under() {
    ir!(checked, map, expr, ir, "fn f(){if true{print(1);}}");
    let exact = Limits {
        max_edges: 2,
        max_scope_frames: 2,
        ..Limits::DEFAULT
    };
    assert!(expr.emit_function_ir(exact).is_ok());
    assert!(matches!(
        expr.emit_function_ir(Limits {
            max_edges: 1,
            ..exact
        }),
        Err(Error::Limit("edges"))
    ));
    assert!(matches!(
        expr.emit_function_ir(Limits {
            max_scope_frames: 1,
            ..exact
        }),
        Err(Error::Limit("scope frames"))
    ));
    ir.validate(exact).unwrap();
}
