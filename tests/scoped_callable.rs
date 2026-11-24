//! Scope-owned value declarations and read/write/match-scrutinee facts.
use sovra::compiler::{
    parser::Parser,
    semantic::Type,
    v2::{
        check_enum_types, BindingEvidenceLimits as Bindings, CheckedEnumTypes,
        EnumFunctionError as Error, EnumSource, EnumTypeLimits, ScopedCallableLimits as Limits,
        ScopedDeclarationKind as Decl, ScopedUseKind as Use,
    },
};
fn env() -> CheckedEnumTypes {
    check_enum_types(
        "owner",
        "types",
        &Parser::new().parse_source("").unwrap(),
        &[EnumSource {
            label: "enum",
            source: "enum E{V(Int),Empty}",
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
#[test]
fn initializer_reads_precede_shadow_declarations_and_branch_scopes_restore_outer_ids() {
    let source = "fn f(n:Int)->Int{let mut total=n;if true{let n=total;total=n;}return n;}";
    let checked = env()
        .check_scoped_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let facts = checked.bindings();
    let decls = facts.declarations();
    assert_eq!(decls.len(), 3);
    assert_eq!(decls[0].kind, Decl::Parameter);
    assert_eq!(decls[1].kind, Decl::Local { mutable: true });
    assert_eq!(decls[2].name.as_ref(), "n");
    let read_targets: Vec<_> = facts
        .uses()
        .iter()
        .filter(|u| u.kind == Use::Read)
        .map(|u| u.declaration)
        .collect();
    assert_eq!(
        read_targets,
        vec![decls[0].id, decls[1].id, decls[2].id, decls[0].id]
    );
    let write = facts.uses().iter().find(|u| u.kind == Use::Write).unwrap();
    assert_eq!(write.declaration, decls[1].id);
    assert!(write.value.is_none());
    for use_fact in facts.uses().iter().filter(|u| u.kind == Use::Read) {
        let expression = checked
            .typed()
            .types()
            .expression(use_fact.value.unwrap())
            .unwrap();
        assert_eq!(expression.span, use_fact.span);
        assert_eq!(
            expression.ty,
            facts.declaration(use_fact.declaration).unwrap().ty
        );
    }
}
#[test]
fn same_block_shadowing_resolves_initializers_to_the_previous_declaration() {
    let source = "fn f()->Int{let x=1;let x=x+1;return x;}";
    let checked = env()
        .check_scoped_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let facts = checked.bindings();
    assert_eq!(facts.declarations().len(), 2);
    assert_eq!(facts.uses()[0].declaration, facts.declarations()[0].id);
    assert_eq!(facts.uses()[1].declaration, facts.declarations()[1].id);
}
#[test]
fn match_scrutinees_payloads_and_arm_local_shadowing_have_explicit_identities() {
    let source="fn f(input:E,n:Int)->Int{match input{E::V(n)=>{let n=n+1;return n;},E::Empty()=>{return n;}}}";
    let checked = env()
        .check_scoped_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let facts = checked.bindings();
    let d = facts.declarations();
    assert_eq!(d.len(), 4);
    assert_eq!(
        d[2].kind,
        Decl::Payload {
            match_index: 0,
            variant: 0
        }
    );
    assert_eq!(d[2].ty, Type::Int);
    let scrutinee = &facts.uses()[0];
    assert_eq!(scrutinee.kind, Use::MatchScrutinee { match_index: 0 });
    assert_eq!(scrutinee.declaration, d[0].id);
    assert!(scrutinee.value.is_none());
    let reads: Vec<_> = facts
        .uses()
        .iter()
        .filter(|u| u.kind == Use::Read)
        .map(|u| u.declaration)
        .collect();
    assert_eq!(reads, vec![d[2].id, d[3].id, d[1].id]);
    assert_eq!(&source[d[2].span.start..d[2].span.end], "n");
}
#[test]
fn indexed_writes_resolve_the_binding_and_keep_separate_evaluated_reads() {
    let source = "fn f()->Int{let mut rows=[1];rows[0]=2;return rows[0];}";
    let checked = env()
        .check_scoped_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let facts = checked.bindings();
    let write = facts
        .uses()
        .iter()
        .find(|u| u.kind == Use::IndexedWrite)
        .unwrap();
    assert_eq!(&source[write.span.start..write.span.end], "rows[0]");
    assert_eq!(write.declaration, facts.declarations()[0].id);
    assert!(write.value.is_none());
    assert_eq!(
        facts.uses().iter().filter(|u| u.kind == Use::Read).count(),
        2
    );
}
#[test]
fn then_else_and_loop_declarations_do_not_replace_continuation_bindings() {
    let source="fn f(n:Int)->Int{if true{let n=1;print(n);}else{let n=2;print(n);}while false{let n=3;print(n);}return n;}";
    let checked = env()
        .check_scoped_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let facts = checked.bindings();
    let declarations = facts.declarations();
    assert_eq!(declarations.len(), 4);
    let targets: Vec<_> = facts
        .uses()
        .iter()
        .filter(|u| u.kind == Use::Read)
        .map(|u| u.declaration)
        .collect();
    assert_eq!(
        targets,
        vec![
            declarations[1].id,
            declarations[2].id,
            declarations[3].id,
            declarations[0].id
        ]
    );
}
#[test]
fn declaration_and_use_ids_refuse_other_units_and_survive_clones() {
    let env = env();
    let source = "fn f(n:Int)->Int{return n;}";
    let a = env
        .check_scoped_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let b = env
        .check_scoped_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let declaration = a.bindings().declarations()[0].id;
    let usage = a.bindings().uses()[0].id;
    assert!(b.bindings().declaration(declaration).is_none());
    assert!(b.bindings().usage(usage).is_none());
    let clone = a.clone();
    assert!(clone.bindings().declaration(declaration).is_some());
    assert!(clone.bindings().usage(usage).is_some());
    assert!(clone
        .typed()
        .types()
        .expression(a.bindings().uses()[0].value.unwrap())
        .is_some());
}
#[test]
fn binding_limits_are_exact_zero_is_real_and_fresh_after_failure() {
    let env = env();
    let source = "fn f(n:Int)->Int{let mut x=n;x=2;return x;}";
    let exact = Bindings {
        max_declarations: 2,
        max_uses: 3,
        max_name_bytes: 2,
        max_type_nodes: 2,
        max_type_text_bytes: 0,
        max_scope_frames: 1,
    };
    let limits = Limits {
        bindings: exact,
        ..Limits::DEFAULT
    };
    assert!(env
        .check_scoped_callable_function_source("f", source, limits)
        .is_ok());
    for bindings in [
        Bindings {
            max_declarations: 1,
            ..exact
        },
        Bindings {
            max_uses: 2,
            ..exact
        },
        Bindings {
            max_name_bytes: 1,
            ..exact
        },
        Bindings {
            max_type_nodes: 1,
            ..exact
        },
        Bindings {
            max_scope_frames: 0,
            ..exact
        },
    ] {
        assert!(matches!(
            env.check_scoped_callable_function_source("f", source, Limits { bindings, ..limits }),
            Err(Error::Limit(_))
        ));
        assert!(env
            .check_scoped_callable_function_source("f", source, limits)
            .is_ok());
    }
    assert!(env
        .check_scoped_callable_function_source(
            "f",
            "fn f(){}",
            Limits {
                bindings: Bindings {
                    max_declarations: 0,
                    max_uses: 0,
                    max_name_bytes: 0,
                    max_type_nodes: 0,
                    max_type_text_bytes: 0,
                    max_scope_frames: 1
                },
                ..Limits::DEFAULT
            }
        )
        .is_ok());
    assert!(matches!(
        env.check_scoped_callable_function_source(
            "f",
            "fn f(input:E)->E{return input;}",
            Limits {
                bindings: Bindings {
                    max_type_text_bytes: 0,
                    ..Bindings::DEFAULT
                },
                ..Limits::DEFAULT
            }
        ),
        Err(Error::Limit(_))
    ));
    assert!(env
        .check_scoped_callable_function_source(
            "f",
            "fn f(input:E)->E{return input;}",
            Limits {
                bindings: Bindings {
                    max_type_text_bytes: 1,
                    ..Bindings::DEFAULT
                },
                ..Limits::DEFAULT
            }
        )
        .is_ok());
}
#[test]
fn scope_frame_caps_include_arms_and_nested_core_blocks() {
    let env = env();
    let source = "fn f(input:E){match input{E::V(n)=>{if true{print(n);}},E::Empty()=>{}}}";
    assert!(env
        .check_scoped_callable_function_source(
            "f",
            source,
            Limits {
                bindings: Bindings {
                    max_scope_frames: 3,
                    ..Bindings::DEFAULT
                },
                ..Limits::DEFAULT
            }
        )
        .is_ok());
    assert!(matches!(
        env.check_scoped_callable_function_source(
            "f",
            source,
            Limits {
                bindings: Bindings {
                    max_scope_frames: 2,
                    ..Bindings::DEFAULT
                },
                ..Limits::DEFAULT
            }
        ),
        Err(Error::Limit(_))
    ));
}
#[test]
fn semantic_errors_keep_precedence_and_old_typed_api_stays_compatible() {
    let env = env();
    let source = "fn f(n:Int){n=1;}";
    let Error::Diagnostics(errors) = env
        .check_scoped_callable_function_source(
            "f",
            source,
            Limits {
                bindings: Bindings {
                    max_declarations: 0,
                    ..Bindings::DEFAULT
                },
                ..Limits::DEFAULT
            },
        )
        .unwrap_err()
    else {
        panic!("expected immutability error")
    };
    assert!(errors.items.iter().any(|e| e.code == "E3011"));
    let source = "fn f(n:Int)->Int{let x=n;return x;}";
    let old = env
        .check_typed_callable_function_source("f", source, Limits::DEFAULT.typed)
        .unwrap();
    let new = env
        .check_scoped_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    assert_eq!(
        old.callable().evidence().contexts(),
        new.typed().callable().evidence().contexts()
    );
    let old_types: Vec<_> = old
        .types()
        .expressions()
        .iter()
        .map(|e| (&e.span, &e.ty))
        .collect();
    let new_types: Vec<_> = new
        .typed()
        .types()
        .expressions()
        .iter()
        .map(|e| (&e.span, &e.ty))
        .collect();
    assert_eq!(old_types, new_types);
}
#[test]
fn declaration_and_use_spans_preserve_unicode_crlf_locations() {
    let source = "// λ😀\r\nfn f(n:Int)->Int{\r\nlet x=n;\r\nreturn x;\r\n}";
    let checked = env()
        .check_scoped_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let facts = checked.bindings();
    assert_eq!(facts.declarations()[1].span.line, 2);
    assert!(
        source[facts.declarations()[1].span.start..facts.declarations()[1].span.end]
            .starts_with("let x")
    );
    assert_eq!(
        &source[facts.uses()[1].span.start..facts.uses()[1].span.end],
        "x"
    );
    assert_eq!(facts.uses()[1].span.line, 3);
}

#[test]
fn reordered_arms_and_multiple_matches_keep_variant_and_match_identities() {
    let source="fn f(input:E){match input{E::Empty()=>{},E::V(n)=>{print(n);}}match input{E::V(n)=>{print(n);},E::Empty()=>{}}}";
    let checked = env()
        .check_scoped_callable_function_source("f", source, Limits::DEFAULT)
        .unwrap();
    let facts = checked.bindings();
    let declarations = facts.declarations();
    assert_eq!(
        declarations[1].kind,
        Decl::Payload {
            match_index: 0,
            variant: 0
        }
    );
    assert_eq!(
        declarations[2].kind,
        Decl::Payload {
            match_index: 1,
            variant: 0
        }
    );
    assert_ne!(declarations[1].id, declarations[2].id);
    let reads: Vec<_> = facts
        .uses()
        .iter()
        .filter(|u| u.kind == Use::Read)
        .map(|u| u.declaration)
        .collect();
    assert_eq!(reads, vec![declarations[1].id, declarations[2].id]);
    assert!(facts
        .uses()
        .iter()
        .any(|u| u.kind == Use::MatchScrutinee { match_index: 1 }
            && u.declaration == declarations[0].id));
}
