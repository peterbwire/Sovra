//! Ordinary standalone compilation uses bounded retained evidence.
use sovra::compiler::{
    compile_program, ir,
    parser::Parser,
    resources::{LoweringLimits, DEFAULT_AST_NODES},
    semantic::SemanticAnalyzer,
};

#[test]
fn normal_compilation_retains_evidence_and_preserves_ir_and_source_errors() {
    for source in [
        include_str!("../examples/numbers/main.svr"),
        include_str!("../examples/records/main.svr"),
        include_str!("../examples/scoped-types/main.svr"),
    ] {
        let parsed = Parser::new().parse_source(source).unwrap();
        let expected = ir::lower_with_limits(
            &SemanticAnalyzer::new().analyze(&parsed).unwrap(),
            LoweringLimits::DEFAULT,
        )
        .unwrap();
        let (actual, unit) = compile_program(&parsed).unwrap();
        assert_eq!(actual, expected);
        assert!(
            unit.source().is_none(),
            "ordinary lowering retains no source copy"
        );
        assert!(unit.semantic_evidence().unwrap().lowering().is_some());
    }
    for source in [
        "fn main() { missing(); }",
        "fn f(x) {} fn main() {}",
        "fn main() { let x: Int = true; }",
    ] {
        let parsed = Parser::new().parse_source(source).unwrap();
        assert_eq!(
            compile_program(&parsed).unwrap_err(),
            SemanticAnalyzer::new().analyze(&parsed).unwrap_err()
        );
    }
}

#[test]
fn active_admission_preserves_ast_code_and_rejects_excess_types_without_fallback() {
    let source = format!("fn main() {{{}}}", "1;".repeat(DEFAULT_AST_NODES / 2));
    let parsed = Parser::new().parse_source(&source).unwrap();
    assert_eq!(compile_program(&parsed).unwrap_err().items[0].code, "E3020");
    drop(parsed);
    let count = 65_536;
    let source = (0..count)
        .map(|i| format!("struct T{i} {{}}"))
        .collect::<String>()
        + "fn f("
        + &(0..count)
            .map(|i| format!("p{i}: T{i}"))
            .collect::<Vec<_>>()
            .join(",")
        + ") {} fn main() {}";
    let parsed = Parser::new().parse_source(&source).unwrap();
    // Existing syntax/semantic rules accept this source; the independently
    // selected interned-type ceiling deliberately adds admission rejection.
    assert!(SemanticAnalyzer::new().analyze(&parsed).is_ok());
    let rejected = compile_program(&parsed).unwrap_err();
    assert_eq!(rejected.items[0].code, "E3025");
    assert!(rejected.items[0].source_file.is_none());
}

#[test]
fn metadata_exhaustion_in_language_test_json_has_no_fabricated_location() {
    let count = 65_536;
    let source = (0..count)
        .map(|i| format!("struct T{i} {{}}"))
        .collect::<String>()
        + "fn f("
        + &(0..count)
            .map(|i| format!("p{i}: T{i}"))
            .collect::<Vec<_>>()
            .join(",")
        + ") {} fn main() {}";
    let path = std::env::temp_dir().join(format!(
        "svr-evidence-cli-{}-{}.svr",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, source).unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_svr"))
        .args(["test", "--format", "json"])
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(!result.status.success());
    let json = String::from_utf8(result.stdout).unwrap();
    assert!(json.contains("E3025"), "{json}");
    assert!(json.contains("\"source_file\":null"), "{json}");
    assert!(json.contains("\"span\":null"), "{json}");
}
