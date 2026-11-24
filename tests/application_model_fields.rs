//! Primitive model fields: syntax, source ownership, semantic parity and admission limits.
use sovra::compiler::{
    project::application::{check_model_primitive_fields as check, syntax, MAX_MODEL_FIELDS},
    semantic::Type,
};

#[test]
fn primitive_aliases_separators_and_source_ownership() {
    let source = "// λ\r\ntype Label = String;\r\ntype Title = Label;\r\nmodel Item {\r\n id\r\n :\r\n Int,\r\n title: Title\r\n active: Bool,\r\n empty: Unit, ratio: Float,\r\n}\r\nmodel Other {id:Int}\r\nmodel Empty{}";
    let checked = check(source).unwrap();
    assert!(checked.diagnostics.is_empty());
    assert_eq!(checked.models.len(), 3);
    assert_eq!(checked.models[2].fields.len(), 0);
    assert_eq!(
        checked
            .resolved
            .iter()
            .map(|f| f.field_type.clone())
            .collect::<Vec<_>>(),
        [
            Type::Int,
            Type::String,
            Type::Bool,
            Type::Unit,
            Type::Float,
            Type::Int
        ]
    );
    assert_eq!(checked.resolved[5].model_index, 1);
    for model in &checked.models {
        assert_eq!(
            model.declaration_span,
            checked.data.declarations[model.declaration_index].span
        );
        for field in &model.fields {
            assert_eq!(
                &source[field.name_span.start..field.name_span.end],
                field.name
            );
            assert_eq!(
                &source[field.annotation_span.start..field.annotation_span.end],
                field.annotation
            );
        }
    }
    assert_eq!(checked.models[0].fields[0].name_span.line, 4);
    assert_eq!(syntax::parse(source).unwrap().records.len(), 0);
    // Existing metadata does not silently acquire field coverage.
    assert!(checked.data.declarations[0].unparsed_body.is_some());
    assert!(syntax::parse_combined(source).is_err());
}

#[test]
fn malformed_or_unsupported_fields_never_return_partial_facts() {
    for body in [
        "id:Int title:String",
        "id:Int,, title:String",
        ",id:Int",
        "id",
        "id:",
        "id:Int;",
        "id:Int=1",
        "id:Id<Item>",
        "id:Int?",
        "id:[Int]",
        "id:module.Type",
        "id:module::Type",
        "id:Int\n, ,",
        "id:Int\nfn hidden(){}",
    ] {
        assert!(
            check(&format!("model Item{{good:Bool, {body}}}")).is_err(),
            "{body}"
        );
    }
    for source in [
        "model Item<T>{}",
        "fn interrupted(\nmodel Item{id:Int}\n)",
        "service Interrupted\nmodel Item{id:Int}",
        "model Item {",
        "import local\nmodel Item{id:Int}",
        "use local::module;\nmodel Item{id:Int}",
        "model Item{}\nmodel Other{id:Item}",
        "enum Role{user}\nmodel Item{role:Role}",
        "struct Record{id:Int}\nmodel Item{record:Record}",
        "model Item{}\ntype Alias=Item;\nmodel Other{id:Alias}",
    ] {
        assert!(check(source).is_err(), "{source}");
    }
}

#[test]
fn diagnostics_point_to_original_name_and_annotation_tokens() {
    let source =
        "// λ\r\nmodel Item{\r\n id:Int\r\n id:String\r\n amount:Money\r\n title:Text\r\n}";
    let checked = check(source).unwrap();
    assert!(checked.resolved.is_empty());
    assert_eq!(
        checked
            .diagnostics
            .items
            .iter()
            .map(|d| (d.code, &source[d.span.start..d.span.end]))
            .collect::<Vec<_>>(),
        [("E3008", "id"), ("E3017", "Money"), ("E3017", "Text")]
    );
    assert!(checked.diagnostics.items[0]
        .message
        .contains("model `Item`"));
    assert_eq!(checked.diagnostics.items[0].span.line, 3);
    assert_eq!(checked.diagnostics.items[0].span.column, 1);
    assert!(check("type Text=String;\nmodel Item{title:Text}")
        .unwrap()
        .diagnostics
        .is_empty());
    let duplicate = check("model Item{}\nmodel Item{}").unwrap();
    assert_eq!(duplicate.diagnostics.items[0].code, "E3008");
}

#[test]
fn core_alias_errors_are_retained_without_valid_field_facts() {
    use sovra::compiler::{lexer::Lexer, parser::Parser, semantic::SemanticAnalyzer};
    for aliases in [
        "type A=B; type B=A;",
        "type A=Missing;",
        "type A=A;",
        "type A=Int; type A=String;",
    ] {
        let program = Parser::new()
            .parse_tokens(&Lexer::new().tokenize(aliases).unwrap())
            .unwrap();
        let expected = SemanticAnalyzer::new().analyze(&program).unwrap_err();
        let checked = check(&format!("{aliases}\nmodel Item{{good:Bool}}")).unwrap();
        assert_eq!(checked.diagnostics, expected);
        assert!(checked.resolved.is_empty());
    }
}

#[test]
fn comments_and_lf_crlf_preserve_equivalent_fields() {
    let source = "model Item{\n first // λ\n : // comment\n String // next field\n second: Int, // trailing\n}";
    for source in [source.to_owned(), source.replace('\n', "\r\n")] {
        let checked = check(&source).unwrap();
        assert!(checked.diagnostics.is_empty());
        assert_eq!(checked.models[0].fields.len(), 2);
        assert_eq!(checked.resolved[0].field_type, Type::String);
        assert_eq!(checked.models[0].fields[1].name_span.line, 4);
    }
}

#[test]
fn field_and_diagnostic_limits_are_exact() {
    let fields = (0..MAX_MODEL_FIELDS)
        .map(|n| format!("f{n}:Int\n"))
        .collect::<String>();
    assert_eq!(
        check(&format!("model Item{{\n{fields}}}"))
            .unwrap()
            .resolved
            .len(),
        MAX_MODEL_FIELDS
    );
    assert!(check(&format!("model Item{{\n{fields}extra:Int}}"))
        .unwrap_err()
        .contains("field count"));
    let fields = (0..1024)
        .map(|n| format!("f{n}:Missing\n"))
        .collect::<String>();
    assert_eq!(
        check(&format!("model Item{{\n{fields}}}"))
            .unwrap()
            .diagnostics
            .items
            .len(),
        1024
    );
    assert!(check(&format!("model Item{{\n{fields}extra:Missing}}"))
        .unwrap_err()
        .contains("diagnostic limit"));
    let exact = "model Item{}\n".repeat(4096);
    // Repeated names would exceed diagnostic quota, so use distinct declarations.
    let exact = exact
        .lines()
        .enumerate()
        .map(|(n, _)| format!("model M{n}{{}}\n"))
        .collect::<String>();
    assert_eq!(check(&exact).unwrap().models.len(), 4096);
    assert!(check(&(exact + "model Extra{}\n")).is_err());
    assert!(check(&" ".repeat(1024 * 1024 + 1)).is_err());
}

#[test]
fn byte_source_token_and_depth_limits_do_not_truncate_projection() {
    let name = "M".repeat(5000);
    let exact = format!("model {name}{{\n{}}}", "id:Int\n".repeat(257));
    let checked = check(&exact).unwrap();
    assert_eq!(checked.diagnostics.items.len(), 256);
    assert_eq!(
        checked
            .diagnostics
            .items
            .iter()
            .map(|d| d.message.len())
            .sum::<usize>(),
        1024 * 1024
    );
    let excess = format!("model {name}{{\n{}}}", "id:Int\n".repeat(258));
    assert!(check(&excess).unwrap_err().contains("diagnostic limit"));
    let prefix = "model Empty{}\n";
    let exact = format!("{prefix}{}", " ".repeat(1024 * 1024 - prefix.len()));
    assert_eq!(check(&exact).unwrap().models.len(), 1);
    // Empty model contributes four tokens; all remaining original tokens count.
    let exact = format!("{prefix}{}", "x ".repeat(65536 - 4));
    assert_eq!(check(&exact).unwrap().models.len(), 1);
    assert!(check(&(exact + "x")).unwrap_err().contains("token limit"));
    assert!(check(&format!("{}{}", "{".repeat(129), "}".repeat(129))).is_err());
}
