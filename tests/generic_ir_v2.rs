//! Typed generic instructions, control flow, source order and finite admission.
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

fn lower(root: &str, functions: &[&str]) -> CheckedGenericIr {
    lower_generic_ir(&plan(root, functions), GenericIrLimits::DEFAULT).unwrap()
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scalar {
    Int(i64),
    Bool(bool),
    Unit,
}
// Independent scalar-only instruction probe. Production value/runtime integration
// is a separate stage; unsupported instructions deliberately fail this test probe.
fn probe(ir: &CheckedGenericIr, index: usize, args: Vec<Scalar>, steps: &mut usize) -> Scalar {
    let function = &ir.functions()[index];
    let mut locals = vec![None; function.slot_layouts().len()];
    for (slot, value) in function.parameter_slots().iter().zip(args) {
        locals[*slot] = Some(value);
    }
    let mut stack = Vec::new();
    let mut pc = 0;
    loop {
        assert!(*steps > 0, "probe step cap");
        *steps -= 1;
        match &function.instructions()[pc].operation {
            GenericIrOperation::Constant {
                value: GenericLiteralValue::Int(v),
                ..
            } => stack.push(Scalar::Int(*v)),
            GenericIrOperation::Constant {
                value: GenericLiteralValue::Bool(v),
                ..
            } => stack.push(Scalar::Bool(*v)),
            GenericIrOperation::Unit { .. } => stack.push(Scalar::Unit),
            GenericIrOperation::Load { slot } => {
                stack.push(locals[*slot].expect("initialized scalar slot"))
            }
            GenericIrOperation::Store { slot, .. } => locals[*slot] = Some(stack.pop().unwrap()),
            GenericIrOperation::Drop => {
                stack.pop().unwrap();
            }
            GenericIrOperation::Binary {
                operator: GenericIrBinary::Numeric(operator),
                ..
            } => {
                let Scalar::Int(right) = stack.pop().unwrap() else {
                    panic!()
                };
                let Scalar::Int(left) = stack.pop().unwrap() else {
                    panic!()
                };
                stack.push(match operator {
                    GenericNumericOperator::Add => Scalar::Int(left + right),
                    GenericNumericOperator::Subtract => Scalar::Int(left - right),
                    GenericNumericOperator::Multiply => Scalar::Int(left * right),
                    GenericNumericOperator::Divide => Scalar::Int(
                        left.checked_div(right)
                            .expect("division must not run on skipped RHS"),
                    ),
                    GenericNumericOperator::Less => Scalar::Bool(left < right),
                    GenericNumericOperator::LessEqual => Scalar::Bool(left <= right),
                    GenericNumericOperator::Greater => Scalar::Bool(left > right),
                    GenericNumericOperator::GreaterEqual => Scalar::Bool(left >= right),
                });
            }
            GenericIrOperation::Binary {
                operator: GenericIrBinary::Equality { negated },
                ..
            } => {
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();
                stack.push(Scalar::Bool((left == right) != *negated));
            }
            GenericIrOperation::Jump { target } => {
                pc = *target;
                continue;
            }
            GenericIrOperation::Branch { yes, no } => {
                let Scalar::Bool(condition) = stack.pop().unwrap() else {
                    panic!()
                };
                pc = if condition { *yes } else { *no };
                continue;
            }
            GenericIrOperation::Call { target, arguments } => {
                let args = stack.split_off(stack.len() - arguments);
                stack.push(probe(ir, *target, args, steps));
            }
            GenericIrOperation::Return => {
                let value = stack.pop().unwrap();
                assert!(stack.is_empty());
                return value;
            }
            other => panic!("unsupported scalar probe instruction {other:?}"),
        }
        pc += 1;
    }
}
#[test]
fn loops_branches_and_mutable_slots_execute_in_the_emitted_order() {
    let ir=lower("fn root()->Int{let mut x=0;let mut total=0;while x<5 {if x<3 {total=total+x;}else{total=total+10;}x=x+1;}return total;}",&[]);
    assert_eq!(probe(&ir, 0, vec![], &mut 1000), Scalar::Int(23));
    assert!(ir.functions()[0]
        .instructions()
        .iter()
        .enumerate()
        .any(|(pc, i)| matches!(i.operation,GenericIrOperation::Jump{target}if target<pc)));
    assert!(ir.functions()[0].max_stack() >= 2);
}
#[test]
fn short_circuit_rhs_calls_are_not_evaluated_and_typed_joins_remain_valid() {
    let ir=lower("fn root()->Int{let a=false&&explode::<Int>();let b=true||explode::<Bool>();if (a||b){return 7;}else{return 0;}}",&["fn explode<T>()->Bool{return 1/0==0;}"]);
    assert_eq!(ir.functions().len(), 3);
    assert_eq!(probe(&ir, 0, vec![], &mut 1000), Scalar::Int(7));
    assert_eq!(
        ir.functions()[0]
            .instructions()
            .iter()
            .filter(|i| matches!(i.operation, GenericIrOperation::Call { .. }))
            .count(),
        2
    );
}
#[test]
fn recursive_calls_link_concrete_targets_and_return_exactly_one_result() {
    let ir = lower(
        "fn root()->Int{return fact::<String>(6);}",
        &["fn fact<T>(n:Int)->Int{if n<=1{return 1;}return n*fact::<T>(n-1);}"],
    );
    assert_eq!(ir.functions().len(), 2);
    assert_eq!(probe(&ir, 0, vec![], &mut 1000), Scalar::Int(720));
    assert!(ir.functions()[1].instructions().iter().any(|i| matches!(
        i.operation,
        GenericIrOperation::Call {
            target: 1,
            arguments: 1
        }
    )));
}
#[test]
fn arrays_nominal_values_indexes_strings_and_explicit_conversions_lower_together() {
    let ir = lower(
        r#"fn root(){let mut rows:[Float]=[1,2.0];rows[0]=3;let boxed=Box::<[Float]>{value:rows};let copy=boxed.value[1];let words="a"+"b";let eq=copy==1;}"#,
        &[],
    );
    let code = ir.functions()[0].instructions();
    assert!(code
        .iter()
        .any(|i| matches!(i.operation, GenericIrOperation::Array { count: 2, .. })));
    assert!(code
        .iter()
        .any(|i| matches!(i.operation, GenericIrOperation::IndexStore { .. })));
    assert!(code
        .iter()
        .any(|i| matches!(i.operation, GenericIrOperation::Field { .. })));
    assert!(code
        .iter()
        .any(|i| matches!(i.operation, GenericIrOperation::Index { .. })));
    assert_eq!(
        code.iter()
            .filter(|i| matches!(i.operation, GenericIrOperation::IntToFloat { .. }))
            .count(),
        3
    );
    assert!(code.iter().any(|i| matches!(
        i.operation,
        GenericIrOperation::Binary {
            operator: GenericIrBinary::String(GenericStringOperator::Concatenate),
            ..
        }
    )));
    let ordered = lower("fn root(){Pair::<Int,Float>{right:2,left:1};}", &[]);
    assert!(ordered.functions()[0]
        .instructions()
        .iter()
        .any(|i| matches!(&i.operation,GenericIrOperation::Record{slots,..}if slots==&[1,0])));
}
#[test]
fn enum_dispatch_addresses_start_with_matching_tag_and_payload_binding() {
    let ir=lower("fn root(value:Choice<Float>)->Float{match value {Choice::Unit(u)=>{return 1;},Choice::End()=>{return 2;},Choice::One(x)=>{return x;},Choice::Many(xs)=>{return xs[0];}}}",&[]);
    let code = ir.functions()[0].instructions();
    let GenericIrOperation::Switch { layout, targets } = &code
        .iter()
        .find(|i| matches!(i.operation, GenericIrOperation::Switch { .. }))
        .unwrap()
        .operation
    else {
        panic!()
    };
    assert_eq!(targets.len(), 4);
    assert!(targets[3] < targets[0]);
    for (tag, target) in targets.iter().enumerate() {
        assert!(
            matches!(code[*target].operation,GenericIrOperation::MatchBind{layout:actual,tag:actual_tag,binding}if actual==*layout && tag==actual_tag && binding.is_some()==(tag!=0))
        );
    }
    assert_eq!(
        code.iter()
            .filter(|i| matches!(i.operation, GenericIrOperation::Load { slot: 0 }))
            .count(),
        1
    );
}
#[test]
fn cumulative_emission_validation_and_pending_frame_budgets_are_exact() {
    let plans=plan("fn root(value:Choice<Int>)->Int{let mut total=0;if true&&false{total=99;}while total<2{total=total+1;}let boxed=Box::<Int>{value:id::<Int>(total)};match value{Choice::End()=>{return total;},Choice::One(x)=>{return x;},Choice::Many(xs)=>{return xs[0];},Choice::Unit(u)=>{return boxed.value;}}}",&["fn id<T>(x:T)->T{return x;}"]);
    let ir = lower_generic_ir(&plans, GenericIrLimits::DEFAULT).unwrap();
    let usage = ir.usage();
    let depth = ir.functions().iter().map(|f| f.max_stack()).max().unwrap();
    let exact = GenericIrLimits {
        max_functions: usage.functions,
        max_instructions: usage.instructions,
        max_slots: usage.slots,
        max_operands: usage.operands,
        max_labels: usage.labels,
        max_targets: usage.targets,
        max_pending: usage.peak_pending,
        max_stack_nodes: usage.stack_nodes,
        max_stack_depth: depth,
        max_work: usage.work,
    };
    assert_eq!(lower_generic_ir(&plans, exact).unwrap().usage(), usage);
    for limits in [
        GenericIrLimits {
            max_functions: exact.max_functions - 1,
            ..exact
        },
        GenericIrLimits {
            max_instructions: exact.max_instructions - 1,
            ..exact
        },
        GenericIrLimits {
            max_slots: exact.max_slots - 1,
            ..exact
        },
        GenericIrLimits {
            max_operands: exact.max_operands - 1,
            ..exact
        },
        GenericIrLimits {
            max_labels: exact.max_labels - 1,
            ..exact
        },
        GenericIrLimits {
            max_targets: exact.max_targets - 1,
            ..exact
        },
        GenericIrLimits {
            max_pending: exact.max_pending - 1,
            ..exact
        },
        GenericIrLimits {
            max_stack_nodes: exact.max_stack_nodes - 1,
            ..exact
        },
        GenericIrLimits {
            max_stack_depth: exact.max_stack_depth - 1,
            ..exact
        },
        GenericIrLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
    ] {
        assert!(lower_generic_ir(&plans, limits).is_err(), "{limits:?}");
    }
}
#[test]
fn unit_fallthrough_and_unreachable_source_keep_owned_decoded_values() {
    let ir = lower(
        r#"// λ😀
fn root(){return;let text="unreachable";}"#,
        &[],
    );
    assert_eq!(probe(&ir, 0, vec![], &mut 100), Scalar::Unit);
    assert!(ir.functions()[0].instructions().iter().any(|i|matches!(&i.operation,GenericIrOperation::Constant{value:GenericLiteralValue::String(s),..}if &**s=="unreachable")));
    let clone = ir.clone();
    drop(ir);
    assert!(!clone.functions()[0].instructions().is_empty());
    let empty = lower("fn root(){}", &[]);
    assert_eq!(probe(&empty, 0, vec![], &mut 100), Scalar::Unit);
    assert_eq!(empty.functions()[0].max_stack(), 1);
}
#[test]
fn nested_match_and_loop_control_preserve_payload_scope_and_instruction_addresses() {
    let ir=lower("fn root(input:Choice<Int>){let mut value=input;while false{match value{Choice::End()=>{},Choice::One(x)=>{value=Choice::<Int>::One(x);match value{Choice::End()=>{},Choice::One(y)=>{},Choice::Many(xs)=>{},Choice::Unit(u)=>{}}},Choice::Many(xs)=>{},Choice::Unit(u)=>{}}}}",&[]);
    assert_eq!(
        ir.functions()[0]
            .instructions()
            .iter()
            .filter(|i| matches!(i.operation, GenericIrOperation::Switch { .. }))
            .count(),
        2
    );
    assert!(check_program_source(
        "strict",
        "fn id<T>(x:T)->T{return x;}fn main(){id::<Int>(1);}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}
