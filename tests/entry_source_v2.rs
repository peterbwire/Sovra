//! A complete supported source, without caller-supplied enum facts or sliced functions.
use sovra::compiler::{
    execution::{MaterializationLimits, RuntimePolicy},
    resources::FrontendLimits,
    v2::{
        check_entry_source, EntryDeclarationKind as Kind, EntryExecutionError as ExecError,
        EntryLoweringLimits as Lower, EntrySourceError as Error, EntrySourceLimits as Limits,
        RuntimeValueFuel, RuntimeValueV2 as Value, RuntimeVmPolicy as Policy, RuntimeVmServices,
    },
};
fn check(source: &str) -> sovra::compiler::v2::CheckedEntrySource {
    check_entry_source("main.svr", source, Limits::DEFAULT).unwrap()
}
#[test]
fn complete_enum_record_alias_entry_source_executes_independent_example() {
    let source = include_str!("../examples/enum-entry/main.svr");
    let unit = check(source);
    assert_eq!(unit.source(), source);
    assert_eq!(unit.source_label(), "main.svr");
    assert_eq!(
        unit.declarations()
            .iter()
            .map(|d| d.kind)
            .collect::<Vec<_>>(),
        vec![Kind::Alias, Kind::Record, Kind::Enum, Kind::Function]
    );
    assert_eq!(
        unit.execute(&[], Lower::DEFAULT, Policy::DEFAULT)
            .unwrap()
            .output,
        vec!["7"]
    );
}
#[test]
fn declarations_resolve_independent_of_source_order_and_empty_enum_payload_is_typed() {
    for source in [
        "fn f()->E{return E::A(7);} enum E{A(Int),B}",
        "enum E{A(Int),B} fn f()->E{return E::A(7);}",
    ] {
        let unit = check(source);
        let result = unit.execute(&[], Lower::DEFAULT, Policy::DEFAULT).unwrap();
        let mut fuel = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
        assert_eq!(result.value.render(&mut fuel, None).unwrap(), "E::A(7)");
    }
    let unit = check("enum E{A([Int])} fn f()->E{return E::A([]);}");
    assert!(unit.execute(&[], Lower::DEFAULT, Policy::DEFAULT).is_ok());
}
#[test]
fn source_coordinates_remain_original_with_unicode_crlf_and_noninitial_enums() {
    let source="// 😀\r\nfn run()->Int{let text=\"λ😀{}\";let v=E::V(7);match v{E::V(n)=>{return n;},E::End()=>{return 0;}}} enum E{V(Int),End}";
    let unit = check(source);
    let syntax = unit.entry().typed().callable().function().syntax();
    assert_eq!(syntax.span.start, source.find("fn run").unwrap());
    assert_eq!(syntax.span.line, 1);
    assert_eq!(syntax.span.column, 0);
    let declaration = &unit
        .entry()
        .typed()
        .callable()
        .function()
        .environment()
        .declarations()[0];
    let offset = source.rfind("enum E").unwrap() + 5;
    assert_eq!(declaration.span.start, offset);
    assert_eq!(declaration.span.line, 1);
    assert_eq!(
        declaration.span.column,
        source[source.find("fn run").unwrap()..offset]
            .chars()
            .count()
    );
    assert_eq!(
        unit.execute(&[], Lower::DEFAULT, Policy::DEFAULT)
            .unwrap()
            .value
            .as_int(),
        Some(7)
    );
    let unit = check("fn f(){}\r\nenum E{V(Int)}");
    assert_eq!(
        unit.entry()
            .typed()
            .callable()
            .function()
            .environment()
            .declarations()[0]
            .span
            .start,
        15
    );
}

#[test]
fn additional_functions_modules_imports_and_trailing_unknown_syntax_never_succeed() {
    for source in [
        "fn a(){} fn b(){}",
        "module m{fn f(){}} fn a(){}",
        "use x::m;fn f(){}",
        "fn f(){} garbage",
        "task f(){}",
        "export",
    ] {
        assert!(
            matches!(
                check_entry_source("s", source, Limits::DEFAULT),
                Err(Error::Unsupported { .. })
            ),
            "{source}"
        );
    }
    for source in ["", "enum E{V(Int)}", "struct R{x:Int}"] {
        assert!(matches!(
            check_entry_source("s", source, Limits::DEFAULT),
            Err(Error::MissingEntry)
        ));
    }
}
#[test]
fn malformed_and_semantically_invalid_source_never_returns_partial_unit() {
    for source in [
        "enum E{V(} fn f(){}",
        "struct R{x:} fn f(){}",
        "type A=; fn f(){}",
        "enum E{V(Missing)} fn f(){}",
        "type A=B;type B=A;fn f(){}",
        "enum E{A,A} fn f(){}",
        "enum E{} fn f(){}",
        "enum E{A} enum E{B} fn f(){}",
        "enum E{A(Int),B} fn f(){let v=E::A(true);}",
        "enum E{A,B} fn f(){let v=E::A();match v{E::A()=>{}}}",
        "enum E{A} fn f(){let v=E::A();if true{match v{E::A()=>{}}}}",
        "fn f(){",
        "enum E{A}",
        "type A=Int fn f(){}",
    ] {
        assert!(
            check_entry_source("s", source, Limits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn whole_source_and_component_limits_refuse_at_exact_boundaries() {
    let source = "enum E{A} fn f(){}";
    let mut limits = Limits::DEFAULT;
    limits.frontend.max_source_bytes = source.len();
    limits.max_source_label_bytes = 1;
    limits.max_declarations = 2;
    assert!(check_entry_source("s", source, limits).is_ok());
    limits.frontend.max_source_bytes -= 1;
    assert!(matches!(
        check_entry_source("s", source, limits),
        Err(Error::Syntax(_))
    ));
    for limits in [
        Limits {
            max_declarations: 1,
            ..Limits::DEFAULT
        },
        Limits {
            max_source_label_bytes: 0,
            ..Limits::DEFAULT
        },
        Limits {
            frontend: FrontendLimits {
                max_tokens: 0,
                ..FrontendLimits::DEFAULT
            },
            ..Limits::DEFAULT
        },
    ] {
        assert!(check_entry_source("s", source, limits).is_err());
    }
    let mut limits = Limits::DEFAULT;
    limits.types.max_enums = 0;
    assert!(matches!(
        check_entry_source("s", source, limits),
        Err(Error::Limit("enums"))
    ));
    let mut limits = Limits::DEFAULT;
    limits.types.syntax.max_variants = 0;
    assert!(matches!(
        check_entry_source("s", source, limits),
        Err(Error::EnumSyntax(_))
    ));
    let mut limits = Limits::DEFAULT;
    limits.callable.typed.callable.function.frontend.max_tokens = 0;
    assert!(matches!(
        check_entry_source("s", source, limits),
        Err(Error::Limit("function tokens"))
    ));
    let mut limits = Limits::DEFAULT;
    limits.callable.bindings.max_declarations = 0;
    assert!(check_entry_source("s", "fn f(x:Int){}", limits).is_err());
}

#[test]
fn checked_source_lowering_and_runtime_refusals_never_fall_back() {
    let unit = check("fn f(){print(7);}");
    let mut lower = Lower::DEFAULT;
    lower.mapping.max_nodes = 0;
    assert!(matches!(
        unit.execute(&[], lower, Policy::DEFAULT),
        Err(ExecError::Mapping(_))
    ));
    let mut lower = Lower::DEFAULT;
    lower.expressions.max_instructions = 0;
    assert!(matches!(
        unit.execute(&[], lower, Policy::DEFAULT),
        Err(ExecError::Expression(_))
    ));
    let policy = Policy {
        runtime: RuntimePolicy {
            materialization: MaterializationLimits {
                max_value_nodes: Some(0),
                max_text_bytes: Some(0),
            },
            ..RuntimePolicy::UNLIMITED
        },
        ..Policy::DEFAULT
    };
    assert!(matches!(
        unit.execute(&[], Lower::DEFAULT, policy),
        Err(ExecError::Runtime(_))
    ));
    assert_eq!(
        unit.execute(&[], Lower::DEFAULT, Policy::DEFAULT)
            .unwrap()
            .output,
        vec!["7"]
    );
}
#[test]
fn complete_entry_source_can_opt_into_command_context_without_ambient_services() {
    let unit=check("enum E{V(String)} fn f(){let v=E::V(std::arg(0));match v{E::V(text)=>{print(text);}}std::set_exit_code(7);}");
    assert!(unit.execute(&[], Lower::DEFAULT, Policy::DEFAULT).is_err());
    let result = unit
        .execute_with_services(
            &[],
            &["λ😀".to_owned()],
            RuntimeVmServices::default(),
            Lower::DEFAULT,
            Policy::DEFAULT,
        )
        .unwrap();
    assert_eq!(result.execution.output, vec!["λ😀"]);
    assert_eq!(result.exit_code, 7);
    // Entry parameters remain separate from program arguments.
    let unit = check("enum E{V(Int)} fn f(n:Int)->E{return E::V(n);}");
    let mut fuel = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
    assert!(unit
        .execute(
            &[Value::int(7, &mut fuel).unwrap()],
            Lower::DEFAULT,
            Policy::DEFAULT
        )
        .is_ok());
}
#[test]
fn all_truncated_declaration_prefixes_refuse_without_panics() {
    let source = "enum E{V(Int),End} struct R{x:Float} type N=Int; fn f(n:N)->E{return E::V(n);}";
    for end in source.char_indices().map(|(i, _)| i) {
        assert!(
            check_entry_source("s", &source[..end], Limits::DEFAULT).is_err(),
            "{end}"
        );
    }
}

#[test]
fn function_diagnostics_retain_original_file_byte_ranges() {
    let source = "enum E{V(Int)}\r\nstruct R{x:Int}\r\nfn f(){print(missing);}";
    let Err(Error::Function(sovra::compiler::v2::EnumFunctionError::Diagnostics(diagnostics))) =
        check_entry_source("s", source, Limits::DEFAULT)
    else {
        panic!("expected function diagnostics")
    };
    assert!(diagnostics
        .items
        .iter()
        .any(|d| source.get(d.span.start..d.span.end) == Some("missing") && d.span.line == 2));
}

#[test]
fn ordinary_entry_matches_legacy_checked_runtime_and_old_parser_stays_strict() {
    let source = "type N=Float;struct R{x:N} fn main(){let a=[R{x:1}];print(a[0].x/2);}";
    let program = sovra::compiler::parser::Parser::new()
        .parse_source(source)
        .unwrap();
    let (legacy, _) = sovra::compiler::compile_program(&program).unwrap();
    assert_eq!(
        check(source)
            .execute(&[], Lower::DEFAULT, Policy::DEFAULT)
            .unwrap()
            .output,
        sovra::compiler::interpreter::run(&legacy).unwrap()
    );
    assert!(sovra::compiler::parser::Parser::new()
        .parse_source(include_str!("../examples/enum-entry/main.svr"))
        .is_err());
}
#[test]
fn root_export_intent_is_retained_without_claiming_module_visibility() {
    let unit = check("export enum E{A} export struct R{x:Int} export fn f()->E{return E::A();}");
    assert!(
        unit.entry()
            .typed()
            .callable()
            .function()
            .syntax()
            .is_exported
    );
    assert!(
        unit.entry()
            .typed()
            .callable()
            .function()
            .environment()
            .declarations()[0]
            .is_exported
    );
    assert!(unit.execute(&[], Lower::DEFAULT, Policy::DEFAULT).is_ok());
}
