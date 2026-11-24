//! Active lexical scopes and exhaustive generic match bodies.
use sovra::compiler::{parser::Parser, v2::*};
fn table(source: &str) -> CheckedGenericSignatures {
    let enums: Vec<_> = [
        "enum Choice<T>{End,One(T),Many(Rows<T>),Unit(Unit)}",
        "enum Other<T>{One(T)}",
        "enum Empty<T>{End}",
    ]
    .iter()
    .map(|s| parse_generic_enum(s, GenericEnumLimits::DEFAULT).unwrap())
    .collect();
    let alias = parse_generic_alias("type Rows<T>=[T];", GenericAliasLimits::DEFAULT).unwrap();
    let mut declarations: Vec<_> = enums.iter().map(GenericTypeDeclaration::Enum).collect();
    declarations.push(GenericTypeDeclaration::Alias(&alias));
    let registry = register_generic_types(&declarations, GenericRegistryLimits::DEFAULT).unwrap();
    let env = prepare_generic_aliases(&registry, &[&alias], GenericAliasEnvironmentLimits::DEFAULT)
        .unwrap();
    let types =
        check_generic_declarations(&env, &declarations, GenericDeclarationLimits::DEFAULT).unwrap();
    let function = parse_generic_function(source, GenericFunctionLimits::DEFAULT).unwrap();
    check_generic_signatures(&types, &[&function], GenericSignatureLimits::DEFAULT).unwrap()
}

fn check(source: &str) -> Result<CheckedGenericCopyBodies, GenericCopyBodyError> {
    check_generic_copy_bodies(&table(source), GenericCopyBodyLimits::DEFAULT)
}
const ARMS: &str = "Choice::End()=>{return fallback;},Choice::One(x)=>{return x;},Choice::Many(xs)=>{return xs[0];},Choice::Unit(unit)=>{return fallback;}";
fn choose(arms: &str) -> String {
    format!("fn choose<U>(value:Choice<U>,fallback:U)->U{{match value {{{arms}}}}}")
}
#[test]
fn exhaustive_arms_retain_single_read_payload_origins_ranges_and_return_coverage() {
    let source = choose(ARMS);
    let checked = check(&source).unwrap();
    let body = &checked.bodies()[0];
    let GenericCopyStatement::Match {
        scrutinee,
        arms,
        span,
        ..
    } = &body.statements()[0]
    else {
        panic!()
    };
    assert_eq!(&source[span.start..span.start + 5], "match");
    assert_eq!(arms.len(), 4);
    assert!(!body.implicit_unit_return());
    assert!(
        matches!(body.expression(*scrutinee).unwrap().kind, GenericCopyExpressionKind::Read(id) if id == body.bindings()[0].id)
    );
    assert_eq!(body.expressions().iter().filter(|e| matches!(e.kind, GenericCopyExpressionKind::Read(id) if id == body.bindings()[0].id)).count(),1);
    for arm in arms {
        assert!(!arm.statements.is_empty());
        assert!(arm.statements.end <= body.statements().len());
        if let Some(id) = arm.payload_binding {
            let binding = body.binding(id).unwrap();
            assert_eq!(
                binding.kind,
                GenericCopyBindingKind::MatchPayload {
                    scrutinee: *scrutinee,
                    variant_index: arm.variant_index
                }
            );
            assert_eq!(
                &source[binding.name_span.start..binding.name_span.end],
                binding.name.as_ref()
            );
        }
    }
    for usage in body.name_uses() {
        let binding = body.binding(usage.binding).unwrap();
        assert_eq!(
            &source[usage.span.start..usage.span.end],
            binding.name.as_ref()
        );
    }
    assert!(body.name_uses().iter().any(|usage| matches!(
        body.binding(usage.binding).unwrap().kind,
        GenericCopyBindingKind::MatchPayload { .. }
    )));
    let clone = body.clone();
    assert!(clone.expression(*scrutinee).is_some());
    let other = check(&source).unwrap();
    assert!(other.bodies()[0].expression(*scrutinee).is_none());
}
#[test]
fn payloads_are_immutable_isolated_and_do_not_leak() {
    for arms in [
        ARMS.replace("return x;", "x = fallback; return x;"),
        ARMS.replace("return fallback;", "return x;"),
        ARMS.replace("return xs[0];", "xs[0] = fallback; return fallback;"),
    ] {
        assert!(check(&choose(&arms)).is_err(), "{arms}");
    }
    assert!(check(&format!(
        "fn f<U>(value:Choice<U>){{match value {{{}}} x;}}",
        ARMS.replace("return fallback;", "return;")
            .replace("return x;", "return;")
            .replace("return xs[0];", "return;")
    ))
    .is_err());
    // Ordinary arm-local shadowing is permitted and restores the parent afterward.
    assert!(check("fn f<U>(value:Choice<U>,x:U)->U {match value {Choice::End()=>{let x=1;},Choice::One(x)=>{let x=true;},Choice::Many(xs)=>{},Choice::Unit(u)=>{}} return x;}").is_ok());
}
#[test]
fn active_scope_selects_the_scrutinee_and_restores_parent_after_branches() {
    assert!(
        check("fn f<U>(value:Choice<U>){let value=1; match value {Choice::End()=>{}}}").is_err()
    );
    assert!(check("fn f<U>(value:Choice<U>){if true {let value=1;} match value {Choice::End()=>{},Choice::One(x)=>{},Choice::Many(xs)=>{},Choice::Unit(u)=>{}}}").is_ok());
    assert!(check("fn f<U>(value:Choice<U>){match missing {Choice::End()=>{}}}").is_err());
}
#[test]
fn nested_matches_and_generic_calls_share_original_tokens_and_contexts() {
    let base = table("fn identity<T>(x:T)->T{return x;}");
    let function=parse_generic_function("fn f<U>(value:Choice<U>,fallback:U)->U {match value {Choice::End()=>{return identity::<U>(fallback);},Choice::One(x)=>{match value {Choice::End()=>{return fallback;},Choice::One(x)=>{return identity::<U>(x);},Choice::Many(xs)=>{return xs[0];},Choice::Unit(u)=>{return fallback;}}},Choice::Many(xs)=>{return xs[0];},Choice::Unit(u)=>{return fallback;}}}",GenericFunctionLimits::DEFAULT).unwrap();
    let signatures = check_generic_signatures(
        base.types(),
        &[base.signatures()[0].syntax(), &function],
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap();
    let checked = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    assert_eq!(
        checked.bodies()[1]
            .statements()
            .iter()
            .filter(|s| matches!(s, GenericCopyStatement::Match { .. }))
            .count(),
        2
    );
    assert_eq!(checked.call_usage().calls, 2);
}
#[test]
fn return_coverage_checks_every_arm_and_keeps_loops_conservative() {
    let incomplete = ARMS.replace("return x;", "let copy=x;");
    assert!(check(&choose(&incomplete)).is_err());
    assert!(check(&format!(
        "fn f<U>(value:Choice<U>,fallback:U)->U{{match value {{{incomplete}}}return fallback;}}"
    ))
    .is_ok());
    let looping = ARMS.replace("return x;", "while true {return x;}");
    assert!(check(&choose(&looping)).is_err());
    let branching = ARMS.replace("return x;", "if true {return x;} else {return fallback;}");
    assert!(check(&choose(&branching)).is_ok());
    assert!(check(&choose(&ARMS.replace("return x;", "return false;"))).is_err());
    assert!(check(&format!(
        "fn unused<U>(value:Choice<U>,fallback:U)->U{{return fallback;match value {{{}}}}}",
        ARMS.replace("return x;", "unknown;return x;")
    ))
    .is_err());
}
#[test]
fn pattern_coverage_and_unsupported_shapes_refuse_without_partial_proofs() {
    for arms in [
        "Choice::End()=>{}",
        "Choice::End()=>{},Choice::End()=>{}",
        "Other::One(x)=>{}",
        "Choice::Missing()=>{}",
        "Choice::One()=>{}",
        "Choice::End(x)=>{}",
        "Choice::Unit()=>{}",
        "Choice::One(_)=>{}",
        "Choice::One(x,y)=>{}",
        "Choice::One(x) if true =>{}",
        "_=>{}",
    ] {
        assert!(check(&choose(arms)).is_err(), "{arms}");
    }
    assert!(check("fn f<U>(value:Choice<U>){match value.One {Choice::End()=>{}}}").is_err());
}
#[test]
fn shared_body_payload_and_ast_allowances_are_exact_and_fail_closed() {
    let signatures = table(&choose(ARMS));
    let checked = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    let exact = GenericCopyBodyLimits {
        max_source_bytes: checked.source_bytes(),
        max_tokens: checked.tokens(),
        max_ast_nodes: checked.ast_nodes(),
        max_statements: checked.statement_count(),
        max_expressions: checked.expression_count(),
        max_bindings: checked.binding_count(),
        max_binding_text_bytes: checked.binding_text_bytes(),
        max_field_nodes: checked.field_nodes(),
        max_field_edges: checked.field_edges(),
        max_work: checked.work(),
        ..GenericCopyBodyLimits::DEFAULT
    };
    assert!(check_generic_copy_bodies(&signatures, exact).is_ok());
    for limit in [
        GenericCopyBodyLimits {
            max_ast_nodes: exact.max_ast_nodes - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_statements: exact.max_statements - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_bindings: exact.max_bindings - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_expressions: exact.max_expressions - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_field_nodes: exact.max_field_nodes - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_field_edges: exact.max_field_edges - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            max_work: exact.max_work - 1,
            ..exact
        },
        GenericCopyBodyLimits {
            field_expansion: GenericFieldSubstitutionLimits {
                max_work: 0,
                ..exact.field_expansion
            },
            ..exact
        },
    ] {
        assert!(
            check_generic_copy_bodies(&signatures, limit).is_err(),
            "{limit:?}"
        );
    }
}
#[test]
fn unicode_ranges_and_contextual_match_identifier_preserve_legacy_grammar() {
    let source = choose(ARMS).replace("match value", "// λ\r\n match value");
    let checked = check(&source).unwrap();
    for binding in checked.bodies()[0].bindings() {
        assert_eq!(
            &source[binding.name_span.start..binding.name_span.end],
            binding.name.as_ref()
        );
    }
    assert!(check("fn f<U>(match:U)->U{return match;}").is_ok());
    assert!(Parser::new()
        .parse_source("fn f(value:Int){match value {Choice::End()=>{}}}")
        .is_err());
    assert!(Parser::new()
        .parse_source("fn f(match:Int)->Int{return match;}")
        .is_ok());
}

#[test]
fn payload_budgets_are_cumulative_across_functions_and_source_profiles_are_bounded() {
    let base = table(&choose(ARMS));
    let second = parse_generic_function(
        &choose(ARMS).replace("fn choose", "fn second"),
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let signatures = check_generic_signatures(
        base.types(),
        &[base.signatures()[0].syntax(), &second],
        GenericSignatureLimits::DEFAULT,
    )
    .unwrap();
    let checked = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    let one = check_generic_copy_bodies(&base, GenericCopyBodyLimits::DEFAULT).unwrap();
    assert_eq!(checked.field_nodes(), 2 * one.field_nodes());
    assert_eq!(checked.field_edges(), 2 * one.field_edges());
    assert!(check_generic_copy_bodies(
        &signatures,
        GenericCopyBodyLimits {
            max_field_nodes: one.field_nodes(),
            ..GenericCopyBodyLimits::DEFAULT
        }
    )
    .is_err());
    // The header name storage cap applies even before nominal lookup succeeds.
    let oversized = format!("{}::End()=>{{}}", "X".repeat(32769));
    assert!(check(&choose(&oversized)).is_err());
    let arms = (0..257)
        .map(|_| "Choice::End()=>{}")
        .collect::<Vec<_>>()
        .join(",");
    assert!(check(&choose(&arms)).is_err());
}
#[test]
fn mutable_scrutinee_replacement_does_not_change_payload_type_or_binding_identity() {
    let source="fn f<U>(input:Choice<U>,fallback:U)->U {let mut value=input; match value {Choice::End()=>{return fallback;},Choice::One(value)=>{return value;},Choice::Many(xs)=>{value=Choice::<U>::End(); return xs[0];},Choice::Unit(u)=>{return fallback;}}}";
    assert!(check(source).is_ok());
    let bad = source.replace("return value;", "value=fallback; return value;");
    assert!(check(&bad).is_err());
}
