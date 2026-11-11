//! Admission and incremental retained-IR budgets, without executing hostile input.

use sovra::compiler::{
    ast::Program,
    ir::{self, Instruction, IrProgram, Literal},
    parser::Parser,
    resources::{check_ast_budget, LoweringLimits, DEFAULT_AST_NODES},
    semantic::SemanticAnalyzer,
};

fn strings(program: &IrProgram) -> usize {
    let mut bytes = 0;
    for function in &program.functions {
        bytes += function.name.len() + function.parameters.iter().map(String::len).sum::<usize>();
        for instruction in &function.instructions {
            bytes += match instruction {
                Instruction::LoadLiteral(
                    Literal::Integer(text) | Literal::Float(text) | Literal::String(text),
                )
                | Instruction::LoadName(text)
                | Instruction::StoreName(text)
                | Instruction::Binary(text)
                | Instruction::LoadField(text) => text.len(),
                Instruction::Call { name, .. } => name.len(),
                Instruction::MakeStruct { type_name, fields } => {
                    type_name.len() + fields.iter().map(String::len).sum::<usize>()
                }
                _ => 0,
            };
        }
    }
    bytes
}

#[test]
fn ast_admission_counts_metadata_statements_and_expressions_exactly() {
    let source = "fn main() { print(1); }";
    let parsed = Parser::new().parse_source(source).unwrap();
    assert_eq!(check_ast_budget(&parsed, 5).unwrap(), 5);
    let errors = SemanticAnalyzer::new()
        .analyze_with_node_limit(&parsed, 4)
        .unwrap_err();
    assert_eq!(errors.items[0].code, "E3020");
    let span = errors.items[0].span;
    assert_eq!(&source[span.start..span.end], "1");
    let metadata = Parser::new().parse_source("type Num = Float; struct Point { x: Num } fn main() {} mod area { type A = Num; struct S { x: A } export fn get(p: Point) -> Float { return p.x; } }").unwrap();
    assert_eq!(check_ast_budget(&metadata, 13).unwrap(), 13);
    assert!(SemanticAnalyzer::new()
        .analyze_with_node_limit(&metadata, 13)
        .is_ok());
    assert!(check_ast_budget(&metadata, 12).is_err());
}

#[test]
fn ir_budgets_include_utf8_function_names_and_zero_instruction_functions() {
    let parsed = Parser::new()
        .parse_source("fn main() { print(\"λ\"); }")
        .unwrap();
    let exact = LoweringLimits {
        max_instructions: 3,
        max_string_bytes: 11,
    };
    let program = ir::lower_program_with_limits(&parsed, 5, exact).unwrap();
    assert_eq!(strings(&program), 11); // main + UTF-8 lambda + bare print alias.
    assert_eq!(program.functions[0].instructions.len(), 3);
    let errors = ir::lower_program_with_limits(
        &parsed,
        5,
        LoweringLimits {
            max_instructions: 2,
            ..exact
        },
    )
    .unwrap_err();
    assert_eq!(errors.items[0].code, "E3021");
    let errors = ir::lower_program_with_limits(
        &parsed,
        5,
        LoweringLimits {
            max_string_bytes: 10,
            ..exact
        },
    )
    .unwrap_err();
    assert_eq!(errors.items[0].code, "E3022");
    let empty = Parser::new().parse_source("").unwrap();
    assert_eq!(check_ast_budget(&empty, 0).unwrap(), 0);
    assert!(ir::lower_program_with_limits(
        &empty,
        0,
        LoweringLimits {
            max_instructions: 0,
            max_string_bytes: 0
        }
    )
    .is_ok());
    let main = Parser::new().parse_source("fn main() {}").unwrap();
    assert!(ir::lower_program_with_limits(
        &main,
        1,
        LoweringLimits {
            max_instructions: 0,
            max_string_bytes: 4
        }
    )
    .is_ok());
    assert!(ir::lower_program_with_limits(
        &main,
        1,
        LoweringLimits {
            max_instructions: 0,
            max_string_bytes: 3
        }
    )
    .is_err());
}

fn parse_fixture() -> (&'static str, Program) {
    let source = r#"
        type Scalar = Float;
        struct Point { value: Scalar }
        mod local { export fn make(value: Scalar) -> Point { return Point { value: value }; } }
        fn main() {
            let mut rows = [[], [2.0]]; rows[0] = [1];
            let p = local::make(3);
            if ((p.value > 1.0 && true) || false) { print(p.value); }
            else { let p = "λ"; print(p); }
            while (false) { print(rows[0][0]); }
        }
    "#;
    (source, Parser::new().parse_source(source).unwrap())
}

#[test]
fn every_instruction_boundary_rejects_without_broken_branch_patching() {
    let (source, parsed) = parse_fixture();
    let typed = SemanticAnalyzer::new().analyze(&parsed).unwrap();
    let expected = ir::lower(&typed);
    assert_eq!(ir::lower_program(&parsed).unwrap(), expected);
    let count = expected
        .functions
        .iter()
        .map(|function| function.instructions.len())
        .sum::<usize>();
    let bytes = strings(&expected);
    for max_instructions in 0..count {
        let errors = ir::lower_with_limits(
            &typed,
            LoweringLimits {
                max_instructions,
                max_string_bytes: bytes,
            },
        )
        .unwrap_err();
        assert_eq!(errors.items.len(), 1);
        assert_eq!(errors.items[0].code, "E3021");
        assert!(errors.items[0].span.end <= source.len());
    }
    for max_string_bytes in [0, 1, bytes / 2, bytes - 1] {
        let errors = ir::lower_with_limits(
            &typed,
            LoweringLimits {
                max_instructions: count,
                max_string_bytes,
            },
        )
        .unwrap_err();
        assert_eq!(errors.items[0].code, "E3022");
    }
    assert_eq!(
        ir::lower_with_limits(
            &typed,
            LoweringLimits {
                max_instructions: count,
                max_string_bytes: bytes
            }
        )
        .unwrap(),
        expected
    );
    assert_eq!(sovra::compiler::interpreter::run(&expected).unwrap(), ["3"]);
}

#[test]
fn checked_budget_path_preserves_semantic_rejection_and_wide_admission() {
    let source = format!("fn main() {{ {} }}", "1;".repeat(10000));
    let parsed = Parser::new().parse_source(&source).unwrap();
    assert_eq!(check_ast_budget(&parsed, 20001).unwrap(), 20001);
    assert!(check_ast_budget(&parsed, 1).is_err());
    let invalid = Parser::new()
        .parse_source("fn main() { print(missing); }")
        .unwrap();
    let errors = ir::lower_program_with_limits(
        &invalid,
        100,
        LoweringLimits {
            max_instructions: 100,
            max_string_bytes: 100,
        },
    )
    .unwrap_err();
    assert_eq!(errors.items[0].code, "E3001");
}

#[test]
fn approved_defaults_admit_exact_ast_boundary_and_reject_overflow() {
    assert_eq!(DEFAULT_AST_NODES, 250_000);
    assert_eq!(LoweringLimits::DEFAULT.max_instructions, 1_000_000);
    assert_eq!(LoweringLimits::DEFAULT.max_string_bytes, 64 * 1024 * 1024);
    let source = format!("fn main() {{ {} return; }}", "1;".repeat(124_999));
    let mut parsed = Parser::new().parse_source(&source).unwrap();
    assert_eq!(
        check_ast_budget(&parsed, DEFAULT_AST_NODES).unwrap(),
        DEFAULT_AST_NODES
    );
    assert!(SemanticAnalyzer::new().analyze(&parsed).is_ok());
    assert!(ir::lower_program(&parsed).is_ok());
    let extra = parsed.functions[0].body[0].clone();
    parsed.functions[0].body.push(extra);
    for result in [
        SemanticAnalyzer::new().analyze(&parsed).map(|_| ()),
        ir::lower_program(&parsed).map(|_| ()),
    ] {
        assert_eq!(result.unwrap_err().items[0].code, "E3020");
    }
    // Explicit callers retain control over admission instead of inheriting a
    // second, hidden default check.
    assert!(SemanticAnalyzer::new()
        .analyze_with_node_limit(&parsed, DEFAULT_AST_NODES + 2)
        .is_ok());
}
