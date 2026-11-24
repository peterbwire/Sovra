//! Public G11 partial-syntax contract, independent of scanner success.
use sovra::compiler::project::application::{inspect_functions, syntax};

#[test]
fn public_syntax_retains_source_nodes_without_asserting_application_validity() {
    let source =
        "type Count = Int;\nstruct Item { count: Count }\nservice Unknown { future syntax }\n";
    let parsed = syntax::parse(source).unwrap();
    assert_eq!(parsed.aliases[0].target, "Int");
    let field = &parsed.records[0].fields[0];
    assert_eq!(field.name, "count");
    let annotation = field.type_span.unwrap();
    assert_eq!(&source[annotation.start..annotation.end], "Count");
    assert_eq!(parsed.unparsed_ranges.len(), 1);
    assert!(
        source[parsed.unparsed_ranges[0].start..parsed.unparsed_ranges[0].end]
            .starts_with("service Unknown")
    );
    assert!(inspect_functions(source, &[]).is_err());
}

#[test]
fn optional_syntax_exhaustion_does_not_disable_ordinary_inspection() {
    let mut source = (0..=syntax::MAX_DECLARATIONS)
        .map(|n| format!("type T{n} = Int;\n"))
        .collect::<String>();
    source.push_str("fn main(){let value:Int=1;}");
    assert!(syntax::parse(&source)
        .unwrap_err()
        .contains("metadata limit"));
    let inspected = inspect_functions(&source, &[]).unwrap();
    assert_eq!(inspected.len(), 1);
    assert_eq!(inspected[0].local_bindings.len(), 1);
}
