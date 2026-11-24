//! Existing route-binding parity beside opaque application declarations.
use sovra::compiler::project::application::{
    self, check_structured_route_placeholders as check, syntax,
};
const PREFIX: &str = "// λ\r\nmodel Item{value:Money}\r\nenum Role{user}\r\nauth session{\r\nallow user to read on Item where false\r\n}\r\npage Home(id:Unknown){fn hidden(){}}\r\nview Card(){}\r\n";
#[test]
fn existing_binding_rules_keep_codes_messages_and_original_spans() {
    for source in [
        "app Demo{\nroute GET \"/:id\" -> handler\n}\nfn handler(id:Int){}",
        "app Demo{\nroute GET \"/:id/:detail\" -> handler\n}\nfn handler(id:Unknown){}",
        "app Demo{\nroute GET \"/\" -> missing\n}\nfn handler(){}",
        "app Demo{\nroute BAD \"relative path\" -> missing\n}\nfn handler(){}",
        "app Demo{\nroute GET \"/\" -> handler\nroute GET \"/\" -> handler\n}\nfn handler(){}",
    ] {
        let old = application::check_route_placeholders(source).unwrap();
        let combined = format!("{PREFIX}{source}");
        let checked = check(&combined).unwrap();
        assert_eq!(
            checked.routes.checked_placeholders,
            old.checked_placeholders
        );
        assert_eq!(
            checked.routes.targets.checked_routes,
            old.targets.checked_routes
        );
        let mut expected = old.diagnostics.items;
        for diagnostic in &mut expected {
            diagnostic.span.start += PREFIX.len();
            diagnostic.span.end += PREFIX.len();
            diagnostic.span.line += PREFIX.matches('\n').count();
        }
        assert_eq!(checked.routes.diagnostics.items, expected, "{source}");
        for target in checked.routes.targets.resolved {
            assert_eq!(
                &combined[target.target_span.start..target.target_span.end],
                "handler"
            );
            let declaration = &checked
                .routes
                .targets
                .syntax
                .declarations
                .base
                .base
                .callables[target.callable_index];
            assert_eq!(target.declaration_span, declaration.span);
        }
        assert!(application::check_route_placeholders(&combined).is_err());
    }
}
#[test]
fn opaque_declarations_never_validate_or_supply_handlers() {
    let source=format!("{PREFIX}app Demo{{\nroute GET \"/:id\" -> handler\n}}\nfn handler(id:Unknown){{print(missing);}}");
    let checked = check(&source).unwrap();
    assert!(checked.routes.diagnostics.is_empty()); // Name binding only, not handler types/body.
    assert_eq!(checked.routes.targets.resolved.len(), 1);
    assert_eq!(
        checked
            .routes
            .targets
            .syntax
            .declarations
            .base
            .base
            .callables
            .len(),
        1
    );
    assert!(matches!(
        checked.auth.blocks[0].policies[0].condition,
        syntax::PolicyConditionSyntax::Unparsed { .. }
    ));
    for span in checked
        .data
        .declarations
        .iter()
        .filter_map(|d| d.unparsed_body)
        .chain(
            checked
                .render
                .declarations
                .iter()
                .filter_map(|d| d.unparsed_body),
        )
    {
        assert!(checked
            .unparsed_ranges
            .iter()
            .any(|gap| gap.start <= span.start && span.end <= gap.end));
    }
    let checked = check(&format!(
        "{PREFIX}app Demo{{\nroute GET \"/\" -> hidden\n}}"
    ))
    .unwrap();
    assert_eq!(checked.routes.diagnostics.items[0].code, "E4033");
    assert!(checked.routes.targets.resolved.is_empty());
    assert_eq!(
        check("model Empty{}\npage Home(){}")
            .unwrap()
            .routes
            .targets
            .checked_routes,
        0
    );
}
#[test]
fn unavailable_contexts_never_select_an_interpretation() {
    for source in [
        "use app.handlers\nmodel Item{}\nfn handler(){}",
        "service Api{}\nmodel Item{}\nfn handler(){}",
        "model Item{}\ntask job(){}",
        "model Item{}\nfn handler(){} fn handler(){}",
        "model Item{}\napp Demo{\nroute GET \"/\" -> remote.handler\n}",
        "model Item{}\napp Demo{\nroute GET \"/\" -> Item\n}",
        "page Home(){}\napp Demo{\nroute GET \"/\" -> Home\n}\nfn Home(){}",
        "auth session{}\napp Demo{\nroute GET \"/\" -> session\n}",
        "model Item{}\napp Demo{\nroute GET \"/:id/:id\" -> handler\n}\nfn handler(id:Int){}",
    ] {
        assert!(
            check(source).unwrap_err().contains("context unavailable"),
            "{source}"
        );
    }
    for source in [
        "fn interrupted(\nmodel Item{}\n)",
        "service Interrupted\nmodel Item{}\n{fn handler(){}}",
    ] {
        assert!(check(source).is_err());
    }
}
#[test]
fn diagnostics_and_original_source_tokens_remain_bounded() {
    let source = format!(
        "model Item{{}}\napp Demo{{\n{}}}",
        "route GET \"/:id\" -> handler\n".repeat(1025)
    );
    let source = format!("{source}\nfn handler(id:Int){{}}");
    assert_eq!(check(&source).unwrap().routes.diagnostics.items.len(), 1024);
    assert!(check(&source.replace(
        "fn handler",
        "app More{\nroute GET \"/:id\" -> handler\n}\nfn handler"
    ))
    .is_err());
    let source = format!(
        "enum Opaque{{{}}}",
        "value ".repeat(syntax::MAX_CALLABLE_TOKENS - 4)
    );
    assert_eq!(
        check(&source).unwrap().token_count,
        syntax::MAX_CALLABLE_TOKENS
    );
    assert!(check(&(source + "\nextra")).is_err());
    assert!(check(&" ".repeat(syntax::MAX_SOURCE_BYTES + 1)).is_err());
}
