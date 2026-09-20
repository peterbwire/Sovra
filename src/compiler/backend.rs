//! Backends for inspecting and compiling M10 IR.

use std::fmt::Write;

use crate::compiler::ir::{Instruction, IrFunction, IrProgram, Literal};

/// Render IR as a stable human-readable build artifact.
pub fn render(program: &IrProgram) -> String {
    let mut output = String::new();
    for function in &program.functions {
        let _ = writeln!(output, "function {}:", function.name);
        for instruction in &function.instructions {
            let _ = writeln!(output, "  {}", instruction_text(instruction));
        }
    }
    output
}

fn instruction_text(instruction: &Instruction) -> String {
    match instruction {
        Instruction::LoadLiteral(value) => format!("load {}", literal_text(value)),
        Instruction::LoadName(name) => format!("load-name {name}"),
        Instruction::StoreName(name) => format!("store-name {name}"),
        Instruction::WidenFloat => "widen-float".to_owned(),
        Instruction::Binary(operator) => format!("binary {operator}"),
        Instruction::Call { name, arguments } => format!("call {name} {arguments}"),
        Instruction::Return => "return".to_owned(),
        Instruction::Pop => "pop".to_owned(),
    }
}

/// Render IR as portable JavaScript.
pub fn render_javascript(program: &IrProgram) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "\"use strict\";");
    let _ = writeln!(output);
    let _ = writeln!(output, "const svrOutput = [];");
    let _ = writeln!(output, "const svrFunctions = Object.create(null);");
    output.push_str(include_str!("numeric_runtime.js"));
    let _ = writeln!(output);
    for (index, function) in program.functions.iter().enumerate() {
        render_js_function(&mut output, index, function);
    }
    for (index, function) in program.functions.iter().enumerate() {
        let _ = writeln!(
            output,
            "svrFunctions[{}] = {};",
            js_string(&function.name),
            js_function_name(index)
        );
    }
    let _ = writeln!(output);
    let _ = writeln!(
        output,
        "if (!svrFunctions.main) throw new Error(\"entry function `main` was not found\");"
    );
    let _ = writeln!(output, "svrFunctions.main();");
    let _ = writeln!(
        output,
        "for (const line of svrOutput) console.log(String(line));"
    );
    output
}

fn render_js_function(output: &mut String, index: usize, function: &IrFunction) {
    let _ = write!(output, "function {}(", js_function_name(index));
    for (parameter_index, parameter) in function.parameters.iter().enumerate() {
        if parameter_index > 0 {
            let _ = write!(output, ", ");
        }
        let _ = write!(output, "{}", js_identifier(parameter));
    }
    let _ = writeln!(output, ") {{");
    let _ = writeln!(output, "  const stack = [];");
    let _ = writeln!(output, "  const names = Object.create(null);");
    for parameter in &function.parameters {
        let _ = writeln!(
            output,
            "  names[{}] = {};",
            js_string(parameter),
            js_identifier(parameter)
        );
    }
    for instruction in &function.instructions {
        render_js_instruction(output, instruction);
    }
    let _ = writeln!(output, "  return stack.length ? stack.pop() : undefined;");
    let _ = writeln!(output, "}}");
    let _ = writeln!(output);
}

fn render_js_instruction(output: &mut String, instruction: &Instruction) {
    match instruction {
        Instruction::LoadLiteral(value) => {
            let _ = writeln!(output, "  stack.push({});", js_literal(value));
        }
        Instruction::LoadName(name) => {
            let _ = writeln!(output, "  stack.push(names[{}]);", js_string(name));
        }
        Instruction::StoreName(name) => {
            let _ = writeln!(output, "  names[{}] = stack.pop();", js_string(name));
        }
        Instruction::WidenFloat => {
            let _ = writeln!(output, "  stack.push(svrWidenFloat(stack.pop()));");
        }
        Instruction::Binary(operator) => {
            let _ = writeln!(output, "  {{");
            let _ = writeln!(output, "    const right = stack.pop();");
            let _ = writeln!(output, "    const left = stack.pop();");
            let _ = writeln!(
                output,
                "    stack.push(svrBinary({}, left, right));",
                js_string(operator)
            );
            let _ = writeln!(output, "  }}");
        }
        Instruction::Call { name, arguments } => render_js_call(output, name, *arguments),
        Instruction::Return => {
            let _ = writeln!(output, "  return stack.length ? stack.pop() : undefined;");
        }
        Instruction::Pop => {
            let _ = writeln!(output, "  stack.pop();");
        }
    }
}

fn render_js_call(output: &mut String, name: &str, arguments: usize) {
    let _ = writeln!(output, "  {{");
    let _ = writeln!(
        output,
        "    const args = stack.splice(stack.length - {arguments});"
    );
    match name {
        "print" | "std::print" | "std::println" => {
            let _ = writeln!(output, "    svrOutput.push(args[0] ?? \"\");");
            let _ = writeln!(output, "    stack.push(undefined);");
        }
        "std::len" => {
            let _ = writeln!(
                output,
                "    stack.push(BigInt(new TextEncoder().encode(args[0]).length));"
            );
        }
        "std::to_string" => {
            let _ = writeln!(
                output,
                "    stack.push(args[0] === undefined ? \"\" : String(args[0]));"
            );
        }
        _ => {
            let _ = writeln!(
                output,
                "    const callee = svrFunctions[{}];",
                js_string(name)
            );
            let _ = writeln!(
                output,
                "    if (!callee) throw new Error(\"runtime function `{name}` was not found\");"
            );
            let _ = writeln!(output, "    stack.push(callee(...args));");
        }
    }
    let _ = writeln!(output, "  }}");
}

fn literal_text(value: &Literal) -> String {
    match value {
        Literal::Integer(value) | Literal::Float(value) => value.clone(),
        Literal::Boolean(value) => value.to_string(),
        Literal::String(value) => format!("{value:?}"),
    }
}

fn js_literal(value: &Literal) -> String {
    match value {
        Literal::Integer(value) => format!("svrCheckedInt(BigInt({}))", js_string(value)),
        Literal::Float(value) => format!("Number({})", js_string(value)),
        Literal::Boolean(value) => value.to_string(),
        Literal::String(value) => js_string(value),
    }
}

fn js_function_name(index: usize) -> String {
    format!("svr_fn_{index}")
}

fn js_identifier(name: &str) -> String {
    let mut identifier = String::from("svr_arg_");
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            identifier.push(character);
        } else {
            identifier.push('_');
        }
    }
    identifier
}

fn js_string(value: &str) -> String {
    format!("{value:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn execute_javascript(source: &str) -> std::process::Output {
        use std::io::Write as _;
        use std::process::{Command, Stdio};
        let mut child = Command::new("node")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Node.js is required for backend execution tests");
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(source.as_bytes())
            .expect("write JavaScript");
        child.wait_with_output().expect("wait for Node.js")
    }

    #[test]
    fn string_concatenation_matches_interpreter() {
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(include_str!("../../examples/strings/main.svr"))
            .expect("valid syntax");
        let ir = crate::compiler::ir::lower_program(&parsed).expect("inferred String types");
        let expected = crate::compiler::interpreter::run(&ir).expect("string execution");
        assert_eq!(expected, vec!["abc!", "2", "true"]);
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual = String::from_utf8(output.stdout).unwrap();
        assert_eq!(actual.lines().collect::<Vec<_>>(), expected);
    }

    #[test]
    fn numeric_javascript_matches_expected_output() {
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(include_str!("../../examples/numbers/main.svr"))
            .expect("valid syntax");
        let ir = crate::compiler::ir::lower_program(&parsed).expect("valid types");
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual = String::from_utf8(output.stdout).expect("UTF-8 output");
        assert_eq!(
            actual.lines().collect::<Vec<_>>(),
            include_str!("../../tests/fixtures/numbers.stdout")
                .lines()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn numeric_javascript_errors_match_interpreter() {
        let mut javascript = String::new();
        let mut expected = Vec::new();
        for expression in [
            "9223372036854775807 + 1",
            "(0 - 9223372036854775807 - 1) - 1",
            "9223372036854775807 * 2",
            "(0 - 9223372036854775807 - 1) / (0 - 1)",
            "1 / 0",
            "1.0 / 0",
            "1 / 0.0",
        ] {
            let parsed = crate::compiler::parser::Parser::new()
                .parse_source(&format!("fn main() {{ print({expression}) }}"))
                .expect("valid syntax");
            let ir = crate::compiler::ir::lower_program(&parsed).expect("valid types");
            expected.push(crate::compiler::interpreter::run(&ir).expect_err("runtime failure"));
            javascript.push_str("try { (function() {\n");
            javascript.push_str(&render_javascript(&ir));
            javascript.push_str("})(); console.log('unexpected success'); } catch (error) { console.log(error.message); }\n");
        }
        let output = execute_javascript(&javascript);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual = String::from_utf8(output.stdout).expect("UTF-8 output");
        assert_eq!(actual.lines().collect::<Vec<_>>(), expected);
    }

    #[test]
    fn renders_function_header() {
        let output = render(&IrProgram {
            functions: vec![crate::compiler::ir::IrFunction {
                name: "main".into(),
                parameters: Vec::new(),
                instructions: vec![Instruction::Return],
            }],
        });
        assert!(output.contains("function main:"));
        assert!(output.contains("return"));
    }

    #[test]
    fn renders_javascript_backend() {
        let output = render_javascript(&IrProgram {
            functions: vec![crate::compiler::ir::IrFunction {
                name: "main".into(),
                parameters: Vec::new(),
                instructions: vec![
                    Instruction::LoadLiteral(Literal::String("Hello".into())),
                    Instruction::Call {
                        name: "std::println".into(),
                        arguments: 1,
                    },
                ],
            }],
        });
        assert!(output.contains("function svr_fn_0()"));
        assert!(output.contains("svrFunctions[\"main\"] = svr_fn_0;"));
        assert!(output.contains("console.log"));
    }
}
