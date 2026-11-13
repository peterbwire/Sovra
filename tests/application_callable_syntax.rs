//! Public callable syntax is retained syntax, never an application-check result.
use sovra::compiler::{
    ast::{ExpressionKind, Statement},
    project::application::{inspect_functions, syntax},
};

#[test]
fn public_callable_api_retains_real_body_nodes_and_original_ranges() {
    let source = "// λ\r\nuse app.library;\r\nexport fn f(value: Unknown)->Int{return missing;}\r\ntask job(){print(1);}";
    let parsed = syntax::parse_callables(source).unwrap();
    assert_eq!(parsed.callables.len(), 2);
    assert_eq!(parsed.callables[1].kind, syntax::CallableKind::Task);
    let function = &parsed.callables[0].function;
    assert!(function.is_exported);
    let Statement::Return {
        value: Some(expression),
        ..
    } = &function.body[0]
    else {
        panic!("expected retained return syntax");
    };
    assert!(matches!(&expression.kind, ExpressionKind::Identifier(name) if name == "missing"));
    assert_eq!(
        &source[expression.span.start..expression.span.end],
        "missing"
    );
    let annotation = function.parameters[0].type_span.unwrap();
    assert_eq!(&source[annotation.start..annotation.end], "Unknown");
    assert_eq!(parsed.unparsed_ranges.len(), 1);
    let gap = parsed.unparsed_ranges[0];
    assert_eq!(&source[gap.start..gap.end], "use app.library;");
}

#[test]
fn callable_projection_does_not_change_the_existing_inspection_contract() {
    let source = "fn f(){let value=missing;}";
    assert!(syntax::parse_callables(source).is_ok()); // Syntax, not name checking.
    assert_eq!(inspect_functions(source, &[]).unwrap().len(), 1);
    assert!(syntax::parse_callables("fn good(){} view unsupported {}").is_err());
    let types = syntax::parse("type T = Int; view unsupported {}").unwrap();
    assert_eq!(types.aliases.len(), 1);
    assert!(!types.unparsed_ranges.is_empty()); // Type-only API remains separate.
}

#[test]
fn public_service_api_retains_prototypes_without_executable_bodies() {
    let source = "service Api{fn declared(x:Unknown)->Missing; fn implemented()->Int{return 1;}}";
    let parsed = syntax::parse_services(source).unwrap();
    assert_eq!(parsed.services.len(), 1);
    assert_eq!(parsed.services[0].operations.len(), 2);
    assert!(parsed.services[0].operations[0].body.is_none());
    assert!(parsed.services[0].operations[1].body.is_some());
    assert!(parsed.unparsed_ranges.is_empty()); // Coverage only, unresolved types remain.
    let signature = &parsed.services[0].operations[0].signature;
    assert_eq!(
        &source[signature.span.start..signature.span.end],
        "fn declared(x:Unknown)->Missing"
    );
    assert_eq!(signature.return_type.as_deref(), Some("Missing"));
}
