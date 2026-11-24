//! Semantic parity between self-contained application functions and executable functions.
use sovra::compiler::{parser::Parser, project::application, semantic::SemanticAnalyzer};

#[test]
fn application_local_diagnostics_match_core_codes_messages_and_original_ranges() {
    let prefix = "// λ context\r\napp Demo{}\r\n";
    for source in [
        "fn main(){print(1);}",
        "type Count=Int; struct Item{value:Count} fn main(){let item=Item{value:1};print(item.value);}",
        "fn widen(value:Int)->Float{return value;} fn main(){print(widen(1));}",
        "fn main(){let value:Int=true;}",
        "fn main(){print(missing);}",
        "fn f()->Int{} fn main(){}",
        "struct P{x:Int} fn main(){let p=P{x:1};print(p.missing);}",
        "fn f(x:Int){} fn main(){f(true);}",
        "fn main(){let value:Unknown=1;}",
        "fn main(){if 1{print(1);}}",
    ] {
        let program = Parser::new().parse_source(source).unwrap();
        let expected = SemanticAnalyzer::new().analyze(&program);
        let combined = format!("{prefix}{source}");
        let checked = application::check_local_functions(&combined).unwrap();
        match expected {
            Ok(_) => assert!(checked.diagnostics.is_empty(), "{source}: {:?}", checked.diagnostics),
            Err(mut diagnostics) => {
                for diagnostic in &mut diagnostics.items {
                    diagnostic.span.start += prefix.len();
                    diagnostic.span.end += prefix.len();
                    diagnostic.span.line += 2;
                }
                assert_eq!(checked.diagnostics.items, diagnostics.items, "{source}");
            }
        }
    }
}

#[test]
fn local_validation_is_additive_and_never_authorizes_entry_fields_or_other_contexts() {
    let source = "app Demo{\nroute BAD \"relative\" -> missing\n}\nfn main(){}";
    let checked = application::check_local_functions(source).unwrap();
    assert_eq!(checked.checked_functions, 1);
    assert!(checked.diagnostics.is_empty()); // Local functions only; route is not validated.
    assert!(application::inspect_functions(source, &[]).is_err());
    for source in [
        "use app.remote; fn main(){}",
        "service Api{} fn main(){}",
        "task job(){} fn main(){}",
    ] {
        assert!(application::check_local_functions(source)
            .unwrap_err()
            .contains("context unavailable"));
    }
}
