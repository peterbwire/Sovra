//! Versioned nominal enum and exhaustive arm-shape facts; no execution claim.
use sovra::compiler::{
    ast::Program,
    diagnostics::Span,
    parser::Parser,
    v2::{
        check_enum_types, EnumMatchArm, EnumPayloadType as Ty, EnumSource, EnumTypeError as Error,
        EnumTypeLimit as Dim, EnumTypeLimits as Limits,
    },
};
fn empty() -> Program {
    Parser::new().parse_source("").unwrap()
}
fn source<'a>(label: &'a str, text: &'a str) -> EnumSource<'a> {
    EnumSource {
        label,
        source: text,
    }
}
fn span() -> Span {
    Span {
        start: 5,
        end: 8,
        line: 1,
        column: 2,
    }
}
#[test]
fn nominal_enum_graph_preserves_aliases_arrays_records_and_recursive_identities() {
    let context=Parser::new().parse_source("type Count=Int;type Counts=[Count];struct Item{value:Float}type Items=[Item];type Link=Node;").unwrap();
    let inputs = [
        source(
            "node.svr",
            "enum Node{End,Next(Node),Children([Node]),Count(Counts),Items(Items)}",
        ),
        source("other.svr", "enum Other{Node(Node),Empty}"),
    ];
    let facts =
        check_enum_types("owner", "context.svr", &context, &inputs, Limits::DEFAULT).unwrap();
    assert_eq!(facts.owner(), "owner");
    let node = facts.find("Node").unwrap();
    let other = facts.find("Other").unwrap();
    assert_ne!(node, other);
    let variants = &facts.declarations()[node.ordinal()].variants;
    assert_eq!(
        facts.payload_type(variants[1].payload.unwrap()),
        Some(&Ty::Enum(node))
    );
    let Ty::Array(element) = facts.payload_type(variants[2].payload.unwrap()).unwrap() else {
        panic!("expected array")
    };
    assert_eq!(facts.payload_type(*element), Some(&Ty::Enum(node)));
    let Ty::Array(element) = facts.payload_type(variants[3].payload.unwrap()).unwrap() else {
        panic!("expected array")
    };
    assert_eq!(facts.payload_type(*element), Some(&Ty::Int));
    let Ty::Array(element) = facts.payload_type(variants[4].payload.unwrap()).unwrap() else {
        panic!("expected array")
    };
    assert_eq!(
        facts.payload_type(*element),
        Some(&Ty::Record("Item".into()))
    );
    assert_eq!(facts.declarations()[0].source_label.as_ref(), "node.svr");
}
#[test]
fn reject_duplicate_empty_unknown_and_alias_cycle_declarations_with_owned_origins() {
    for (context, inputs, code, label) in [
        (
            "",
            vec![source("a", "enum E{A}"), source("b", "enum E{B}")],
            "E3100",
            "b",
        ),
        ("", vec![source("a", "enum E{}")], "E3101", "a"),
        ("", vec![source("a", "enum E{A,A(Int)}")], "E3102", "a"),
        ("", vec![source("a", "enum E{A(Missing)}")], "E3017", "a"),
        (
            "type A=B;type B=A;",
            vec![source("a", "enum E{V(A)}")],
            "E3017",
            "context",
        ),
    ] {
        let context = Parser::new().parse_source(context).unwrap();
        let error =
            check_enum_types("owner", "context", &context, &inputs, Limits::DEFAULT).unwrap_err();
        let Error::Diagnostics(errors) = error else {
            panic!("{error:?}")
        };
        assert!(
            errors
                .iter()
                .any(|e| e.diagnostic.code == code && e.source_label.as_ref() == label),
            "{errors:?}"
        );
        for error in errors {
            let text = if error.source_label.as_ref() == "context" {
                "type A=B;type B=A;"
            } else {
                inputs
                    .iter()
                    .find(|i| i.label == error.source_label.as_ref())
                    .unwrap()
                    .source
            };
            assert!(
                error.diagnostic.span.start <= error.diagnostic.span.end
                    && error.diagnostic.span.end <= text.len()
            );
            assert!(error.diagnostic.source_file.is_none());
        }
    }
}
#[test]
fn coverage_and_payload_binding_arity_are_exhaustive_and_nominal() {
    let inputs = [source("a", "enum E{None,Unit(Unit),Text(String)}")];
    let facts = check_enum_types("owner", "context", &empty(), &inputs, Limits::DEFAULT).unwrap();
    let id = facts.find("E").unwrap();
    let arm = |name, binding| EnumMatchArm {
        enum_name: "E",
        variant_name: name,
        has_payload_binding: binding,
        span: span(),
    };
    let plan = facts
        .check_match(
            id,
            "use.svr",
            span(),
            &[arm("Text", true), arm("None", false), arm("Unit", true)],
        )
        .unwrap();
    assert_eq!(plan.variant_order(), [2, 0, 1]);
    assert_eq!(plan.enum_id(), id);
    assert_eq!(plan.environment().owner(), "owner");
    for arms in [
        vec![arm("None", false)],
        vec![arm("Text", true), arm("None", false), arm("None", false)],
        vec![arm("None", true), arm("Unit", true), arm("Text", true)],
        vec![arm("None", false), arm("Unit", false), arm("Text", true)],
        vec![arm("Missing", false)],
        vec![EnumMatchArm {
            enum_name: "Other",
            variant_name: "None",
            has_payload_binding: false,
            span: span(),
        }],
    ] {
        let Error::Diagnostics(errors) =
            facts.check_match(id, "use.svr", span(), &arms).unwrap_err()
        else {
            panic!("expected coverage error")
        };
        assert!(errors
            .iter()
            .all(|e| e.diagnostic.code == "E3103" && e.source_label.as_ref() == "use.svr"));
    }
}
#[test]
fn checked_identities_cannot_be_replayed_in_another_environment() {
    let inputs = [source("a", "enum E{Some(Int)}")];
    let a = check_enum_types("same", "context", &empty(), &inputs, Limits::DEFAULT).unwrap();
    let b = check_enum_types("same", "context", &empty(), &inputs, Limits::DEFAULT).unwrap();
    let id = a.find("E").unwrap();
    let payload = a.declarations()[0].variants[0].payload.unwrap();
    assert_ne!(id, b.find("E").unwrap());
    assert!(b.payload_type(payload).is_none());
    assert_eq!(
        b.check_match(id, "use", span(), &[]).unwrap_err(),
        Error::UnavailableIdentity
    );
    let clone = a.clone();
    assert_eq!(clone.payload_type(payload), Some(&Ty::Int));
}
#[test]
fn source_syntax_and_context_failures_do_not_become_enum_facts() {
    let input = [source("bad", "enum E{V(Int,String)}")];
    assert!(
        matches!(check_enum_types("owner","ctx",&empty(),&input,Limits::DEFAULT),Err(Error::Syntax{source_label,..})if source_label.as_ref()=="bad")
    );
    let inputs = [source("same", "enum A{X}"), source("same", "enum B{X}")];
    assert!(matches!(
        check_enum_types("owner", "ctx", &empty(), &inputs, Limits::DEFAULT),
        Err(Error::DuplicateSourceLabel(_))
    ));
    let context = Parser::new().parse_source("struct E{x:Int}").unwrap();
    assert!(matches!(
        check_enum_types(
            "owner",
            "ctx",
            &context,
            &[source("enum", "enum E{X}")],
            Limits::DEFAULT
        ),
        Err(Error::Diagnostics(_))
    ));
    // This API intentionally checks type headers, not arbitrary context function bodies.
    let context = Parser::new()
        .parse_source("fn main(){print(missing);}")
        .unwrap();
    assert!(check_enum_types(
        "owner",
        "ctx",
        &context,
        &[source("a", "enum E{V(Int)}")],
        Limits::DEFAULT
    )
    .is_ok());
    assert!(sovra::compiler::compile_program(&context).is_err());
}
#[test]
fn bounded_metadata_refusal_is_explicit_and_fresh_after_each_failure() {
    let inputs = [source("a", "enum E{V([Int])}")];
    let context = empty();
    for (limits, dimension, cap) in [
        (
            Limits {
                max_enums: 0,
                ..Limits::DEFAULT
            },
            Dim::Enums,
            0,
        ),
        (
            Limits {
                max_source_bytes: 0,
                ..Limits::DEFAULT
            },
            Dim::SourceBytes,
            0,
        ),
        (
            Limits {
                max_context_ast_nodes: 0,
                ..Limits::DEFAULT
            },
            Dim::ContextAstNodes,
            0,
        ),
        (
            Limits {
                max_metadata_nodes: 0,
                ..Limits::DEFAULT
            },
            Dim::MetadataNodes,
            0,
        ),
        (
            Limits {
                max_text_bytes: 0,
                ..Limits::DEFAULT
            },
            Dim::TextBytes,
            0,
        ),
    ] {
        assert_eq!(
            check_enum_types("owner", "ctx", &context, &inputs, limits).unwrap_err(),
            Error::Limit {
                dimension,
                limit: cap
            }
        );
        assert!(check_enum_types("owner", "ctx", &context, &inputs, Limits::DEFAULT).is_ok());
    }
}

#[test]
fn exact_metadata_budgets_refuse_one_under_before_returning_facts() {
    let inputs = [source("s", "enum E{V([Int])}")];
    let limits = Limits {
        max_enums: 1,
        max_source_bytes: 16,
        max_context_ast_nodes: 3,
        max_metadata_nodes: 9,
        max_text_bytes: 23,
        ..Limits::DEFAULT
    };
    assert!(check_enum_types("o", "c", &empty(), &inputs, limits).is_ok());
    for (limited, dimension, limit) in [
        (
            Limits {
                max_source_bytes: 15,
                ..limits
            },
            Dim::SourceBytes,
            15,
        ),
        (
            Limits {
                max_context_ast_nodes: 2,
                ..limits
            },
            Dim::ContextAstNodes,
            2,
        ),
        (
            Limits {
                max_metadata_nodes: 8,
                ..limits
            },
            Dim::MetadataNodes,
            8,
        ),
        (
            Limits {
                max_text_bytes: 22,
                ..limits
            },
            Dim::TextBytes,
            22,
        ),
    ] {
        assert_eq!(
            check_enum_types("o", "c", &empty(), &inputs, limited).unwrap_err(),
            Error::Limit { dimension, limit }
        );
    }
}
#[test]
fn source_origins_survive_context_alias_resolution_and_long_declarations() {
    let text = "// λ😀\r\nenum Outcome{Values([Missing])}";
    let Error::Diagnostics(errors) = check_enum_types(
        "owner",
        "context",
        &empty(),
        &[source("enum.svr", text)],
        Limits::DEFAULT,
    )
    .unwrap_err() else {
        panic!("expected source error")
    };
    let error = errors
        .iter()
        .find(|e| e.source_label.as_ref() == "enum.svr")
        .unwrap();
    let s = error.diagnostic.span;
    assert_eq!(&text[s.start..s.end], "[Missing]");
    assert_eq!(s.line, 1);
    // Many qualified components remain bounded spelling, not recursive type unfolding.
    let text = format!(
        "enum E{{V({})}}",
        "[".repeat(127) + "Int" + &"]".repeat(127)
    );
    assert!(check_enum_types(
        "owner",
        "context",
        &empty(),
        &[source("deep", &text)],
        Limits::DEFAULT
    )
    .is_ok());
}

#[test]
fn constructor_plans_preserve_nullary_unit_nominal_types_and_numeric_widening() {
    use sovra::compiler::v2::{EnumConstructorArgument as Arg, EnumPayloadCoercion as Coerce};
    let context = Parser::new()
        .parse_source("type Count=Int;type Counts=[[Int]];struct Item{x:Int}")
        .unwrap();
    let inputs = [source(
        "types",
        "enum E{None,Unit(Unit),Scalar(Float),Rows([[Float]]),Record(Item),Child(E)}",
    )];
    let facts = check_enum_types("owner", "ctx", &context, &inputs, Limits::DEFAULT).unwrap();
    let id = facts.find("E").unwrap();
    let arg = |name| Arg {
        type_name: name,
        span: span(),
    };
    let plan = facts
        .check_constructor(id, "use", span(), "None", &[])
        .unwrap();
    assert_eq!(plan.expected_payload(), None);
    assert_eq!(plan.payload_coercion(), None);
    assert_eq!(plan.variant_index(), 0);
    assert_eq!(plan.environment().owner(), "owner");
    assert_eq!(plan.enum_id(), id);
    for (variant, argument, coercion) in [
        ("Unit", "Unit", Coerce::Identity),
        ("Scalar", "Count", Coerce::IntToFloat { array_depth: 0 }),
        ("Rows", "Counts", Coerce::IntToFloat { array_depth: 2 }),
        ("Record", "Item", Coerce::Identity),
        ("Child", "E", Coerce::Identity),
    ] {
        let plan = facts
            .check_constructor(id, "use", span(), variant, &[arg(argument)])
            .unwrap();
        assert_eq!(plan.payload_coercion(), Some(coercion));
        assert!(plan.expected_payload().is_some());
    }
}
#[test]
fn constructor_plans_refuse_arity_unresolved_narrowing_and_foreign_nominals() {
    use sovra::compiler::v2::EnumConstructorArgument as Arg;
    let context = Parser::new()
        .parse_source("struct Item{x:Int}struct Other{x:Int}")
        .unwrap();
    let inputs = [
        source("types", "enum E{None,Number(Int),Record(Item),Child(E)}"),
        source("other", "enum OtherEnum{None}"),
    ];
    let facts = check_enum_types("owner", "ctx", &context, &inputs, Limits::DEFAULT).unwrap();
    let id = facts.find("E").unwrap();
    let arg = |name| Arg {
        type_name: name,
        span: span(),
    };
    for (name, args, code) in [
        ("Missing", vec![], "E3105"),
        ("None", vec![arg("Unit")], "E3105"),
        ("Number", vec![], "E3105"),
        ("Number", vec![arg("Float")], "E3106"),
        ("Record", vec![arg("Other")], "E3106"),
        ("Child", vec![arg("OtherEnum")], "E3106"),
        ("Number", vec![arg("Missing")], "E3106"),
    ] {
        let Error::Diagnostics(errors) = facts
            .check_constructor(id, "caller.svr", span(), name, &args)
            .unwrap_err()
        else {
            panic!("expected constructor error")
        };
        assert_eq!(errors[0].source_label.as_ref(), "caller.svr");
        assert_eq!(errors[0].diagnostic.code, code);
        assert_eq!(errors[0].diagnostic.span, span());
    }
    let other = check_enum_types("owner", "ctx", &context, &inputs, Limits::DEFAULT).unwrap();
    assert_eq!(
        other
            .check_constructor(id, "caller", span(), "None", &[])
            .unwrap_err(),
        Error::UnavailableIdentity
    );
}

#[test]
fn scoped_enums_resolve_local_aliases_records_and_nominal_references() {
    use sovra::compiler::v2::{check_scoped_enum_types, ScopedEnumSource};
    let context = Parser::new()
        .parse_source("mod a{type Counts=[Int];export struct Item{x:Int}} mod b{}")
        .unwrap();
    let inputs = [
        ScopedEnumSource {
            module: Some("a"),
            declaration: source("a.svr", "enum E{End,Rows(Counts),Items([Item]),Next(E)}"),
        },
        ScopedEnumSource {
            module: Some("b"),
            declaration: source("b.svr", "enum E{Link(a::E)}"),
        },
        ScopedEnumSource {
            module: None,
            declaration: source("root.svr", "enum E{End}"),
        },
    ];
    let facts =
        check_scoped_enum_types("owner", "context", &context, &inputs, Limits::DEFAULT).unwrap();
    let a = facts.find("a::E").unwrap();
    let b = facts.find("b::E").unwrap();
    assert_ne!(a, b);
    assert_ne!(a, facts.find("E").unwrap());
    let variants = &facts.declarations()[a.ordinal()].variants;
    assert_eq!(
        facts.payload_type(variants[3].payload.unwrap()),
        Some(&Ty::Enum(a))
    );
    let array = variants[2].payload.unwrap();
    let Ty::Array(item) = facts.payload_type(array).unwrap() else {
        panic!("expected array")
    };
    assert_eq!(
        facts.payload_type(*item),
        Some(&Ty::Record("a::Item".into()))
    );
    let absent = [ScopedEnumSource {
        module: Some("missing"),
        declaration: source("missing.svr", "enum E{End}"),
    }];
    assert!(matches!(
        check_scoped_enum_types("owner", "context", &context, &absent, Limits::DEFAULT),
        Err(Error::UnavailableModule(_))
    ));
}

#[test]
fn retained_real_record_fields_consume_metadata_and_text_allowances() {
    let context = Parser::new().parse_source("struct R{x:Int}").unwrap();
    let inputs = [source("s", "enum E{V(R)}")];
    let limits = Limits {
        max_metadata_nodes: 15,
        max_text_bytes: 25,
        ..Limits::DEFAULT
    };
    assert!(check_enum_types("o", "c", &context, &inputs, limits).is_ok());
    assert!(matches!(
        check_enum_types(
            "o",
            "c",
            &context,
            &inputs,
            Limits {
                max_metadata_nodes: 14,
                ..limits
            }
        ),
        Err(Error::Limit {
            dimension: Dim::MetadataNodes,
            limit: 14
        })
    ));
    assert!(matches!(
        check_enum_types(
            "o",
            "c",
            &context,
            &inputs,
            Limits {
                max_text_bytes: 24,
                ..limits
            }
        ),
        Err(Error::Limit {
            dimension: Dim::TextBytes,
            limit: 24
        })
    ));
}
