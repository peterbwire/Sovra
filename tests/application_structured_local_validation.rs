//! Ordinary semantic parity beside opaque application declarations, without contextual fallback.
use sovra::compiler::{
    parser::Parser,
    project::application::{self, check_structured_local_functions as check, syntax},
    semantic::SemanticAnalyzer,
};

const PREFIX: &str = "// λ\r\nmodel Item{value:Money}\r\nenum Role{user admin}\r\nauth session{\r\nprovider: future\r\nallow user to read on Item where false\r\n}\r\npage Home(id:Unknown){view Hidden(){}}\r\nview Card(){fn hidden(){}}\r\napp Demo{\r\ntitle: \"future\"\r\nroute BAD \"relative\" -> missing\r\n}\r\n";

#[test]
fn ordinary_function_semantics_match_core_with_original_unicode_crlf_spans() {
    for source in [
        "fn main(){print(1);}",
        "type Count=Int; struct Record{value:Count} fn main(){let item=Record{value:1};print(item.value);}",
        "fn widen(value:Int)->Float{return value;} fn main(){print(widen(1));}",
        "fn main(){let value:Int=true;}",
        "fn main(){print(missing);}",
        "fn main(){hidden();}",
        "fn f()->Int{} fn main(){}",
        "struct P{x:Int} fn main(){let p=P{x:1};print(p.missing);}",
        "fn f(x:Int){} fn main(){f(true);}",
        "fn main(){let value:Unknown=1;}",
        "fn main(){if 1{print(1);}}",
    ] {
        let program = Parser::new().parse_source(source).unwrap();
        let expected = SemanticAnalyzer::new().analyze(&program);
        let combined = format!("{PREFIX}{source}");
        let checked = check(&combined).unwrap();
        assert_eq!(checked.checked_functions, program.functions.len());
        match expected {
            Ok(_) => assert!(checked.diagnostics.is_empty(), "{source}"),
            Err(mut diagnostics) => {
                for diagnostic in &mut diagnostics.items {
                    diagnostic.span.start += PREFIX.len();
                    diagnostic.span.end += PREFIX.len();
                    diagnostic.span.line += PREFIX.matches('\n').count();
                }
                assert_eq!(checked.diagnostics.items, diagnostics.items, "{source}");
            }
        }
    }
}

#[test]
fn opaque_headers_bodies_and_bindings_never_become_checked_or_executable() {
    let checked = check(&format!(
        "{PREFIX}fn main(){{print(\"Item Home session\");}}"
    ))
    .unwrap();
    assert!(checked.diagnostics.is_empty());
    assert_eq!(checked.checked_functions, 1);
    assert_eq!(checked.syntax.render.declarations.len(), 2);
    assert_eq!(checked.syntax.data.declarations.len(), 2);
    assert_eq!(
        checked.syntax.base.declarations.base.base.callables.len(),
        1
    );
    assert_eq!(
        checked
            .syntax
            .base
            .declarations
            .base
            .base
            .types
            .records
            .len(),
        0
    );
    for span in checked
        .syntax
        .data
        .declarations
        .iter()
        .filter_map(|d| d.unparsed_body)
        .chain(
            checked
                .syntax
                .render
                .declarations
                .iter()
                .filter_map(|d| d.unparsed_body),
        )
    {
        assert!(checked
            .syntax
            .unparsed_ranges
            .iter()
            .any(|gap| gap.start <= span.start && span.end <= gap.end));
    }
    assert!(matches!(
        checked.syntax.auth.blocks[0].policies[0].condition,
        syntax::PolicyConditionSyntax::Unparsed { .. }
    ));
    assert!(application::check_local_functions(&format!("{PREFIX}fn main(){{}}")).is_err());
    assert!(!application::check_model_primitive_fields(PREFIX)
        .unwrap()
        .diagnostics
        .is_empty());
    assert_eq!(
        check("model Empty{}\npage Home(){}\nauth session{}")
            .unwrap()
            .checked_functions,
        0
    );
}

#[test]
fn missing_context_is_unavailable_and_never_an_empty_success() {
    for source in [
        "use app.library\nmodel Item{}\nfn main(){}",
        "model Item{}\nservice Api{fn f();}\nfn main(){}",
        "model Item{}\ntask job(){}\nfn main(){}",
        "model Item{}\nfn main(){let item=Item{id:1};}",
        "enum Role{user}\nfn f(role:Role){}",
        "model Item{}\ntype Alias=Item;\nfn main(){}",
        "model Item{}\nstruct Record{value:Item}\nfn main(){}",
        "page Home(){}\nfn main(){Home();}",
        "auth session{}\nfn main(){print(session);}",
        // Conservative unavailable outcomes, not global namespace/shadowing rules.
        "model Item{}\nfn main(){let Item=1;print(Item);}",
        "page Home(){}\nfn Home(){}",
    ] {
        assert!(
            check(source).unwrap_err().contains("context unavailable"),
            "{source}"
        );
    }
    for source in [
        "model Item{}\nfn main(){let x=;}",
        "fn interrupted(\nmodel Item{}\n)",
        "service Interrupted\nmodel Item{}\n{fn main(){}}",
    ] {
        assert!(check(source).is_err(), "{source}");
    }
}

#[test]
fn original_source_token_and_depth_limits_include_opaque_bodies() {
    assert!(check(&" ".repeat(syntax::MAX_SOURCE_BYTES + 1)).is_err());
    let source = format!(
        "enum Opaque{{{}}}",
        "value ".repeat(syntax::MAX_CALLABLE_TOKENS - 4)
    );
    assert_eq!(
        check(&source).unwrap().syntax.token_count,
        syntax::MAX_CALLABLE_TOKENS
    );
    assert!(check(&(source + "\nextra")).is_err());
    assert!(check(&format!(
        "page Home(){{{}{}}}",
        "{".repeat(128),
        "}".repeat(128)
    ))
    .is_err());
}
