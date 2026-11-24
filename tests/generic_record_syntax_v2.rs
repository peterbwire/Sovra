//! Generic record attachment, shared parameter identities and cumulative field admission.
use sovra::compiler::{lexer::Lexer, v2::*};
fn parse(source: &str) -> GenericRecordSyntax {
    parse_generic_record(source, GenericRecordLimits::DEFAULT).unwrap()
}
#[test]
fn fields_share_the_record_parameter_owner_and_original_full_source_spans() {
    let source = "// λ😀\nexport struct Entry<K:std::Key,V>{key:K,values:[V],pair:Pair<K,V>,}";
    let record = parse(source);
    assert!(record.is_exported());
    assert_eq!(record.name(), "Entry");
    assert_eq!(record.source(), source);
    assert_eq!(record.fields().len(), 3);
    assert_eq!(
        &source[record.span().start..record.span().end],
        source.trim_start_matches("// λ😀\n")
    );
    for (index, field) in record.fields().iter().enumerate() {
        assert_eq!(
            &source[field.name_span.start..field.name_span.end],
            field.name
        );
        for usage in field.annotation.uses() {
            let parameter = record.parameters().parameter(usage.parameter).unwrap();
            assert!(field
                .annotation
                .parameters()
                .parameter(usage.parameter)
                .is_some());
            let span = record.field_type_node_span(index, usage.node).unwrap();
            assert_eq!(&source[span.start..span.end], parameter.name);
            assert_eq!(record.parameter_span(usage.parameter).unwrap().line, 1);
        }
    }
    assert_eq!(
        record.fields()[0].annotation.uses()[0].parameter,
        record.fields()[2].annotation.uses()[0].parameter
    );
    let other = parse("struct Other<K>{value:K}");
    assert!(other
        .parameter_span(record.parameters().find("K").unwrap())
        .is_none());
    assert!(record.field_type_node_span(usize::MAX, 0).is_none());
}
#[test]
fn commas_inside_generic_types_are_not_field_delimiters_and_empty_records_remain_valid() {
    let record = parse("struct R<T>{x:Map<T,Pair<[T],Int>>,y:T}");
    assert_eq!(record.fields().len(), 2);
    assert_eq!(record.parameter_uses(), 3);
    assert!(parse("struct Empty<T>{}").fields().is_empty());
    assert!(parse("struct Empty<T>{}").parameters().find("T").is_some());
}
#[test]
fn field_limits_are_aggregate_not_reset_for_each_annotation() {
    let source = "struct Pair<T>{first:[T],second:[T]}";
    let record = parse(source);
    let exact = GenericRecordLimits {
        max_source_bytes: source.len(),
        max_tokens: Lexer::new().tokenize(source).unwrap().len(),
        max_retained_source_bytes: record.retained_source_bytes(),
        max_name_bytes: record.name_bytes(),
        max_fields: 2,
        max_type_nodes: record.type_nodes(),
        max_type_edges: record.type_edges(),
        max_parameter_uses: record.parameter_uses(),
        ..GenericRecordLimits::DEFAULT
    };
    assert!(parse_generic_record(source, exact).is_ok());
    for limits in [
        GenericRecordLimits {
            max_source_bytes: source.len() - 1,
            ..exact
        },
        GenericRecordLimits {
            max_tokens: exact.max_tokens - 1,
            ..exact
        },
        GenericRecordLimits {
            max_retained_source_bytes: exact.max_retained_source_bytes - 1,
            ..exact
        },
        GenericRecordLimits {
            max_name_bytes: exact.max_name_bytes - 1,
            ..exact
        },
        GenericRecordLimits {
            max_fields: 1,
            ..exact
        },
        GenericRecordLimits {
            max_type_nodes: exact.max_type_nodes - 1,
            ..exact
        },
        GenericRecordLimits {
            max_type_edges: exact.max_type_edges - 1,
            ..exact
        },
        GenericRecordLimits {
            max_parameter_uses: exact.max_parameter_uses - 1,
            ..exact
        },
        GenericRecordLimits {
            annotations: GenericTypeLimits {
                max_nodes: 1,
                ..exact.annotations
            },
            ..exact
        },
    ] {
        assert!(parse_generic_record(source, limits).is_err(), "{limits:?}");
    }
}
#[test]
fn duplicates_malformed_fields_and_extra_declarations_refuse() {
    for source in [
        "",
        "struct",
        "struct R{}",
        "struct R<>{}",
        "struct R<T>",
        "struct R<T>{",
        "struct R<T>{x}",
        "struct R<T>{x:}",
        "struct R<T>{x:T,x:Int}",
        "struct R<T>{x:T<Int>}",
        "struct R<T>{x:[T}",
        "struct R<T>{x:T]}",
        "struct R<T>{x:Map<T,T}",
        "struct R<T>{x:T y:T}",
        "struct R<T>{x:T;} ",
        "struct R<T>{x:T}struct S<U>{}",
        "mod m{struct R<T>{x:T}}",
    ] {
        assert!(
            parse_generic_record(source, GenericRecordLimits::DEFAULT).is_err(),
            "{source}"
        );
    }
}
#[test]
fn field_fragment_errors_use_original_multiline_coordinates() {
    let source = "// λ😀\nstruct R<T>{\n first:T,\n second:T<Int>\n}";
    let GenericTypeError::Syntax { span, .. } =
        parse_generic_record(source, GenericRecordLimits::DEFAULT).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(span.line, 3);
    assert_eq!(&source[span.start..span.end], "T<Int>");
}
#[test]
fn truncations_do_not_index_past_eof_and_execution_stays_strict() {
    for source in [
        "struct R<T>{}",
        "export struct R<T>{first:T,second:Pair<T,[T]>}",
    ] {
        for (end, _) in source.char_indices() {
            assert!(parse_generic_record(&source[..end], GenericRecordLimits::DEFAULT).is_err());
        }
        assert!(parse_generic_record(source, GenericRecordLimits::DEFAULT).is_ok());
    }
    assert!(check_program_source(
        "strict",
        "struct R<T>{x:T}fn main(){}",
        ProgramSourceLimits::DEFAULT
    )
    .is_err());
    assert_eq!(parse("struct R<T>{x:Unknown<T>}").fields().len(), 1);
}
