//! Production host-free generic execution, owned snapshots and shared resource policy.
use sovra::compiler::{
    execution::{
        CancellationToken, ExecutionControl, ExecutionLimits, MaterializationLimits, RuntimePolicy,
    },
    v2::*,
};
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

fn run(root: &str, functions: &[&str]) -> GenericVmOutcome {
    execute_generic_ir(&lower(root, functions), GenericVmPolicy::DEFAULT).unwrap()
}
const FACT: &str = "fn fact<T>(n:Int)->Int{if n<=1{return 1;}return n*fact::<T>(n-1);}";
#[test]
fn iterative_calls_loops_and_short_circuiting_execute_without_a_test_probe() {
    let result=run("fn root()->Int{let mut n=0;let mut total=0;while n<5{total=total+n;n=n+1;}return total+fact::<String>(6);}",&[FACT]);
    assert_eq!(result.value().as_int(), Some(730));
    assert_eq!(result.peak_frames(), 7);
    let short = run(
        "fn root()->Bool{return (false&&explode::<Int>())||(true||explode::<Bool>());}",
        &["fn explode<T>()->Bool{return 1/0==0;}"],
    );
    assert_eq!(short.value().as_bool(), Some(true));
}
#[test]
fn arrays_records_and_repeated_indexed_replacement_preserve_owned_snapshots() {
    let result=run("fn root()->Pair<[[Int]],[[Int]]>{let mut a:[[Int]]=[[1],[2],[3]];let old=a;let mut child=a[0];child[0]=9;a[1]=[7,8];a[0]=[];a[2]=[4];let boxed=Box::<[[Int]]>{value:a};return Pair::<[[Int]],[[Int]]>{right:old,left:boxed.value};}",&[]);
    let root = result.value();
    let updated = root.field(0).unwrap();
    let old = root.field(1).unwrap();
    assert_eq!(updated.array_item(0).unwrap().array_len(), Some(0));
    assert_eq!(
        updated
            .array_item(1)
            .unwrap()
            .array_item(0)
            .unwrap()
            .as_int(),
        Some(7)
    );
    assert_eq!(
        updated
            .array_item(1)
            .unwrap()
            .array_item(1)
            .unwrap()
            .as_int(),
        Some(8)
    );
    assert_eq!(
        updated
            .array_item(2)
            .unwrap()
            .array_item(0)
            .unwrap()
            .as_int(),
        Some(4)
    );
    for index in 0..3 {
        assert_eq!(
            old.array_item(index)
                .unwrap()
                .array_item(0)
                .unwrap()
                .as_int(),
            Some(index as i64 + 1)
        );
    }
    assert_eq!(root.footprint().0, 15);
    let equal = run(
        "fn root()->Bool{return Pair::<Int,Int>{right:2,left:1}==Pair::<Int,Int>{left:1,right:2};}",
        &[],
    );
    assert_eq!(equal.value().as_bool(), Some(true));
}
#[test]
fn enums_and_generic_payload_matches_keep_nullary_and_unit_distinct() {
    let choose="fn choose<T>(value:Choice<T>,fallback:T)->T{match value{Choice::Unit(u)=>{return fallback;},Choice::Many(xs)=>{return xs[0];},Choice::End()=>{return fallback;},Choice::One(x)=>{return x;}}}";
    let result = run(
        r#"fn root()->[String]{return [choose::<String>(Choice::<String>::One("ok"),"fail"),choose::<String>(Choice::<String>::End(),"end"),choose::<String>(Choice::<String>::Many(["many"]),"fail"),choose::<String>(Choice::<String>::Unit(unit::<String>()),"unit")];}"#,
        &[choose, "fn unit<T>(){}"],
    );
    for (index, text) in ["ok", "end", "many", "unit"].iter().enumerate() {
        assert_eq!(
            result.value().array_item(index).unwrap().as_str(),
            Some(*text)
        );
    }
    let unit = run(
        "fn root()->Choice<Int>{return Choice::<Int>::Unit(unit::<Int>());}",
        &["fn unit<T>(){}"],
    );
    assert_eq!(unit.value().enum_tag(), Some(3));
    assert!(unit.value().payload().unwrap().is_unit());
    let nullary = run("fn root()->Choice<Int>{return Choice::<Int>::End();}", &[]);
    assert_eq!(nullary.value().enum_tag(), Some(0));
    assert!(nullary.value().payload().is_none());
}
#[test]
fn arithmetic_widening_strings_and_failure_categories_match_existing_value_contracts() {
    let widened = run(
        "fn root()->Float{let mut values:[Float]=[1];values[0]=3;return values[0]/2.0;}",
        &[],
    );
    assert_eq!(widened.value().as_float(), Some(1.5));
    let huge = format!("fn root()->Float{{return {}.0*2.0;}}", "9".repeat(308));
    assert!(run(&huge, &[]).value().as_float().unwrap().is_infinite());
    let text = run(r#"fn root()->String{return "λ😀"+"!";}"#, &[]);
    assert_eq!(text.value().as_str(), Some("λ😀!"));
    for (source, kind) in [
        (
            "fn root()->Int{return 1/0;}",
            GenericVmErrorKind::Arithmetic("division by zero"),
        ),
        (
            "fn root()->Int{return 9223372036854775807+1;}",
            GenericVmErrorKind::Arithmetic("integer overflow"),
        ),
        (
            "fn root()->Int{let a:[Int]=[];return a[0];}",
            GenericVmErrorKind::Bounds,
        ),
        (
            "fn root(){let mut a=[1];a[0-1]=2;}",
            GenericVmErrorKind::Bounds,
        ),
    ] {
        let source = source.replace("let mut a=[1]", "let mut a:[Int]=[1]");
        let error = execute_generic_ir(&lower(&source, &[]), GenericVmPolicy::DEFAULT).unwrap_err();
        assert_eq!(error.kind, kind);
        assert!(error.span.end <= source.len());
    }
}
#[test]
fn exact_shared_fuel_work_and_active_frame_limits_and_one_under_are_enforced() {
    let ir = lower(
        r#"fn root()->Pair<Int,String>{return Pair::<Int,String>{right:"λ😀"+"!",left:fact::<Int>(5)};}"#,
        &[FACT],
    );
    let baseline = execute_generic_ir(&ir, GenericVmPolicy::DEFAULT).unwrap();
    let nodes = 1_000_000
        - baseline
            .remaining()
            .materialization
            .max_value_nodes
            .unwrap();
    let text = 16 * 1024 * 1024 - baseline.remaining().materialization.max_text_bytes.unwrap();
    let exact = GenericVmPolicy {
        runtime: RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: Some(baseline.instructions()),
                max_output_bytes: Some(0),
            },
            materialization: MaterializationLimits {
                max_value_nodes: Some(nodes),
                max_text_bytes: Some(text),
            },
        },
        max_work: baseline.work(),
        max_call_depth: baseline.peak_frames(),
        max_active_slots: baseline.peak_slots(),
        max_active_operands: baseline.peak_operands(),
        ..GenericVmPolicy::DEFAULT
    };
    let success = execute_generic_ir(&ir, exact.clone()).unwrap();
    assert_eq!(success.value().field(0).unwrap().as_int(), Some(120));
    assert_eq!(success.remaining().materialization.max_value_nodes, Some(0));
    assert_eq!(success.remaining().materialization.max_text_bytes, Some(0));
    for policy in [
        GenericVmPolicy {
            max_work: exact.max_work - 1,
            ..exact.clone()
        },
        GenericVmPolicy {
            max_call_depth: exact.max_call_depth - 1,
            ..exact.clone()
        },
        GenericVmPolicy {
            max_active_slots: exact.max_active_slots - 1,
            ..exact.clone()
        },
        GenericVmPolicy {
            max_active_operands: exact.max_active_operands - 1,
            ..exact.clone()
        },
        GenericVmPolicy {
            runtime: RuntimePolicy {
                execution: ExecutionLimits {
                    max_instructions: Some(baseline.instructions() - 1),
                    ..exact.runtime.execution
                },
                ..exact.runtime
            },
            ..exact.clone()
        },
        GenericVmPolicy {
            runtime: RuntimePolicy {
                materialization: MaterializationLimits {
                    max_value_nodes: Some(nodes - 1),
                    ..exact.runtime.materialization
                },
                ..exact.runtime
            },
            ..exact.clone()
        },
        GenericVmPolicy {
            runtime: RuntimePolicy {
                materialization: MaterializationLimits {
                    max_text_bytes: Some(text - 1),
                    ..exact.runtime.materialization
                },
                ..exact.runtime
            },
            ..exact.clone()
        },
    ] {
        assert!(execute_generic_ir(&ir, policy).is_err());
    }
}
#[test]
fn cancellation_elapsed_instruction_and_recursive_frame_refusals_have_no_fallback() {
    let ir = lower("fn root(){while true{}}", &[]);
    let token = CancellationToken::new();
    token.cancel();
    for control in [
        ExecutionControl {
            cancellation: Some(token),
            max_elapsed: None,
        },
        ExecutionControl {
            cancellation: None,
            max_elapsed: Some(std::time::Duration::ZERO),
        },
    ] {
        assert!(matches!(
            execute_generic_ir(
                &ir,
                GenericVmPolicy {
                    control,
                    ..GenericVmPolicy::DEFAULT
                }
            ),
            Err(GenericVmError {
                kind: GenericVmErrorKind::Execution(_),
                ..
            })
        ));
    }
    assert!(execute_generic_ir(
        &ir,
        GenericVmPolicy {
            runtime: RuntimePolicy {
                execution: ExecutionLimits {
                    max_instructions: Some(20),
                    ..ExecutionLimits::UNLIMITED
                },
                materialization: MaterializationLimits::UNLIMITED
            },
            ..GenericVmPolicy::DEFAULT
        }
    )
    .is_err());
    let recursive = lower(
        "fn root(){forever::<Int>();}",
        &["fn forever<T>(){forever::<T>();}"],
    );
    assert!(matches!(
        execute_generic_ir(
            &recursive,
            GenericVmPolicy {
                max_call_depth: 4,
                ..GenericVmPolicy::DEFAULT
            }
        ),
        Err(GenericVmError {
            kind: GenericVmErrorKind::Limit("call depth"),
            ..
        })
    ));
    for depth in [0, 257] {
        assert!(execute_generic_ir(
            &recursive,
            GenericVmPolicy {
                max_call_depth: depth,
                ..GenericVmPolicy::DEFAULT
            }
        )
        .is_err());
    }
}
#[test]
fn deep_recursive_values_copy_inspect_and_drop_without_recursive_rust_values() {
    let ir=lower("fn root()->Tree<Int>{let mut tree=Tree::<Int>::End();let mut n=0;while n<2000{tree=Tree::<Int>::Next([tree]);n=n+1;}let copy=tree;if(tree==copy){return copy;}return tree;}",&[]);
    let result = execute_generic_ir(
        &ir,
        GenericVmPolicy {
            runtime: RuntimePolicy {
                materialization: MaterializationLimits {
                    max_value_nodes: Some(10_000_000),
                    max_text_bytes: Some(64 * 1024 * 1024),
                },
                ..GenericVmPolicy::DEFAULT.runtime
            },
            max_work: 100_000_000,
            ..GenericVmPolicy::DEFAULT
        },
    )
    .unwrap();
    assert_eq!(result.value().footprint().0, 4001);
    let mut value = result.value();
    let mut depth = 0;
    while value.enum_tag() == Some(2) {
        value = value.payload().unwrap().array_item(0).unwrap();
        depth += 1;
    }
    assert_eq!(depth, 2000);
    assert_eq!(value.enum_tag(), Some(0));
    assert_eq!(result.peak_frames(), 1);
    drop(result);
}
#[test]
fn empty_unit_and_root_argument_boundary_keep_borrowed_results_owned() {
    let result = run("fn root(){}", &[]);
    assert!(result.value().is_unit());
    assert_eq!(result.instructions(), 2);
    assert_eq!(result.value().footprint(), (1, 0));
    let parameterized = lower("fn root(value:Int)->Int{return value;}", &[]);
    assert!(matches!(
        execute_generic_ir(&parameterized, GenericVmPolicy::DEFAULT),
        Err(GenericVmError {
            kind: GenericVmErrorKind::RootArguments,
            ..
        })
    ));
    assert!(result.value().field(0).is_none());
    assert!(result.value().array_item(0).is_none());
    assert!(check_program_source(
        "strict",
        "fn id<T>(x:T)->T{return x;}fn main(){id::<Int>(1);}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
}

#[test]
fn payload_bindings_and_callee_array_locals_keep_their_own_snapshots() {
    let matched=run("fn root()->[Int]{let mut value=Choice::<[Int]>::One([1,2]);match value{Choice::One(xs)=>{value=Choice::<[Int]>::One([9]);return xs;},Choice::Many(rows)=>{return rows[0];},Choice::End()=>{return [];},Choice::Unit(u)=>{return [];}}}",&[]);
    assert_eq!(matched.value().array_len(), Some(2));
    assert_eq!(matched.value().array_item(0).unwrap().as_int(), Some(1));
    assert_eq!(matched.value().array_item(1).unwrap().as_int(), Some(2));
    let called=run("fn root()->Pair<[Int],[Int]>{let original:[Int]=[1];let changed=change::<Bool>(original);return Pair::<[Int],[Int]>{left:original,right:changed};}",&["fn change<T>(xs:[Int])->[Int]{let mut copy=xs;copy[0]=9;return copy;}"]);
    assert_eq!(
        called
            .value()
            .field(0)
            .unwrap()
            .array_item(0)
            .unwrap()
            .as_int(),
        Some(1)
    );
    assert_eq!(
        called
            .value()
            .field(1)
            .unwrap()
            .array_item(0)
            .unwrap()
            .as_int(),
        Some(9)
    );
}
