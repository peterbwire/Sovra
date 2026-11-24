//! Isolated regressions: an old stack overflow must fail a child, not the test runner.
use sovra::compiler::{
    backend, interpreter,
    ir::{Instruction, IrFunction, IrProgram, Literal},
};
fn program(case: &str, depth: usize) -> IrProgram {
    let mut instructions = vec![Instruction::LoadLiteral(
        if case.ends_with("materialization-text-error") {
            Literal::String("λ".into())
        } else {
            Literal::Integer("7".into())
        },
    )];
    let records = case.starts_with("record-");
    let case = case.strip_prefix("record-").unwrap_or(case);
    for index in 0..depth {
        if records && index % 2 == 0 {
            instructions.push(Instruction::MakeStruct {
                type_name: "Node".into(),
                fields: vec!["next".into()],
            });
        } else {
            instructions.push(Instruction::MakeArray { length: 1 });
        }
    }
    match case {
        "append" | "append-error" | "concat" | "concat-error" => {
            instructions.push(Instruction::LoadLiteral(Literal::Integer("0".into())));
            if case.starts_with("concat") {
                instructions.push(Instruction::MakeArray { length: 1 });
            }
            instructions.push(Instruction::Call {
                name: if case.starts_with("concat") {
                    "std::array_concat"
                } else {
                    "std::array_append"
                }
                .into(),
                arguments: 2,
            });
        }
        "slice" | "slice-error" => {
            instructions.push(Instruction::LoadLiteral(Literal::Integer("0".into())));
            instructions.push(Instruction::LoadLiteral(Literal::Integer("1".into())));
            instructions.push(Instruction::Call {
                name: "std::array_slice".into(),
                arguments: 3,
            });
        }
        "materialization-copy-error" | "materialization-text-error" => {
            instructions.push(Instruction::StoreName("tree".into()));
            instructions.push(Instruction::LoadName("tree".into()));
        }
        "materialization-parameter-error" => instructions.push(Instruction::Call {
            name: "identity".into(),
            arguments: 1,
        }),
        "materialization-partial-frame-error" => {
            instructions.push(Instruction::LoadLiteral(Literal::Integer("0".into())));
            instructions.push(Instruction::Call {
                name: "pair".into(),
                arguments: 2,
            });
        }
        "clone" => {
            instructions.push(Instruction::StoreName("tree".into()));
            instructions.push(Instruction::LoadName("tree".into()));
            instructions.push(Instruction::StoreName("copy".into()));
        }
        "display" | "javascript" => {
            instructions.push(Instruction::Call {
                name: "std::to_string".into(),
                arguments: 1,
            });
            instructions.push(Instruction::Call {
                name: "std::len".into(),
                arguments: 1,
            });
            instructions.push(Instruction::Call {
                name: "print".into(),
                arguments: 1,
            });
        }
        "pop" | "budget-instruction-error" => instructions.push(Instruction::Pop),
        "budget-output-error" => instructions.push(Instruction::Call {
            name: "print".into(),
            arguments: 1,
        }),
        "return" => instructions.push(Instruction::Return),
        "overwrite" => {
            instructions.push(Instruction::StoreName("tree".into()));
            instructions.push(Instruction::LoadLiteral(Literal::Integer("0".into())));
            instructions.push(Instruction::StoreName("tree".into()));
        }
        "call" => instructions.push(Instruction::Call {
            name: "identity".into(),
            arguments: 1,
        }),
        "host-error" | "control-host-error" => instructions.push(Instruction::Call {
            name: "print".into(),
            arguments: 1,
        }),
        "duplicate-field-error" => {
            instructions.push(Instruction::LoadLiteral(Literal::Integer("0".into())));
            instructions.push(Instruction::MakeStruct {
                type_name: "Invalid".into(),
                fields: vec!["same".into(), "same".into()],
            });
        }
        "frame-error" => {
            instructions.push(Instruction::StoreName("tree".into()));
            instructions.push(Instruction::Jump { target: usize::MAX });
        }
        "builtin-error" => instructions.push(Instruction::Call {
            name: "std::len".into(),
            arguments: 1,
        }),
        "binary-underflow" => instructions.push(Instruction::Binary("+".into())),
        "field-error" => instructions.push(Instruction::LoadField("missing".into())),
        "widen-error" => instructions.push(Instruction::WidenFloatArray { depth: 0 }),
        "replace" => {
            instructions.push(Instruction::LoadLiteral(Literal::Integer("0".into())));
            instructions.push(Instruction::LoadLiteral(Literal::Integer("9".into())));
            instructions.push(Instruction::StoreIndex);
        }
        _ => unreachable!(),
    }
    IrProgram {
        functions: vec![
            IrFunction {
                name: "main".into(),
                parameters: vec![],
                instructions,
            },
            IrFunction {
                name: "pair".into(),
                parameters: vec!["tree".into(), "other".into()],
                instructions: vec![],
            },
            IrFunction {
                name: "identity".into(),
                parameters: vec!["value".into()],
                instructions: vec![Instruction::LoadName("value".into()), Instruction::Return],
            },
        ],
    }
}
#[test]
fn deep_child() {
    let Ok(case) = std::env::var("SOVRA_DEEP_RUNTIME_CASE") else {
        return;
    };
    let depth = 16000;
    let ir = program(&case, depth);
    let records = case.starts_with("record-");
    let case = case.strip_prefix("record-").unwrap_or(&case);
    let expected_length = if records {
        1 + (depth / 2) * ("Node { next: ".len() + " }".len() + 2)
    } else {
        2 * depth + 1
    };
    if case == "javascript" {
        use std::io::Write;
        use std::process::Stdio;
        let mut child = std::process::Command::new("node")
            .arg("--stack-size=256")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut javascript = backend::render_javascript(&program("pop", 0));
        javascript.push_str(&format!(r#"
let deep = 7n;
for (let index = 0; index < {depth}; index++) {{
  if ({records} && index % 2 === 0) deep = {{__svrStruct: {{typeName: "Node", fields: {{next: deep}}, fieldOrder: ["next"]}}}};
  else deep = [deep];
}}
const copied = svrCopy(deep);
console.log(svrDisplay(copied).length);
"#));
        child
            .stdin
            .take()
            .unwrap()
            .write_all(javascript.as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{:?}: {}",
            result.status,
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            String::from_utf8(result.stdout).unwrap().trim(),
            expected_length.to_string()
        );
    } else {
        struct FailingHost(Option<sovra::compiler::execution::CancellationToken>);
        impl interpreter::RuntimeHost for FailingHost {
            fn arguments(&self) -> &[String] {
                &[]
            }
            fn read_line(&mut self) -> Result<Option<String>, String> {
                Err("unexpected input".into())
            }
            fn write_line(&mut self, _: &str) -> Result<(), String> {
                if let Some(token) = &self.0 {
                    token.cancel();
                    Ok(())
                } else {
                    Err("host output blocked".into())
                }
            }
        }
        let result = if case.starts_with("append") || case.starts_with("concat") {
            use sovra::compiler::execution::{MaterializationLimits, RuntimePolicy};
            let concat = u64::from(case.starts_with("concat"));
            interpreter::run_with_policy(
                &ir,
                RuntimePolicy {
                    materialization: MaterializationLimits {
                        max_value_nodes: Some(if case.ends_with("error") {
                            depth as u64 + 2 + concat
                        } else {
                            2 * depth as u64 + 4 + concat
                        }),
                        max_text_bytes: None,
                    },
                    ..RuntimePolicy::UNLIMITED
                },
            )
        } else if case == "slice" || case == "slice-error" {
            use sovra::compiler::execution::{MaterializationLimits, RuntimePolicy};
            interpreter::run_with_policy(
                &ir,
                RuntimePolicy {
                    materialization: MaterializationLimits {
                        max_value_nodes: Some(if case == "slice" {
                            2 * depth as u64 + 4
                        } else {
                            depth as u64 + 3
                        }),
                        max_text_bytes: None,
                    },
                    ..RuntimePolicy::UNLIMITED
                },
            )
        } else if case == "control-host-error" {
            use sovra::compiler::execution::{CancellationToken, ExecutionControl, RuntimePolicy};
            let token = CancellationToken::new();
            let outcome = interpreter::run_with_host_status_and_policy_and_control(
                &ir,
                &mut FailingHost(Some(token.clone())),
                RuntimePolicy::UNLIMITED,
                ExecutionControl {
                    cancellation: Some(token),
                    max_elapsed: None,
                },
            );
            assert_eq!(outcome.unwrap_err(), "execution cancelled");
            Err("execution cancelled".into())
        } else if case.starts_with("materialization-") {
            use sovra::compiler::execution::{MaterializationLimits, RuntimePolicy};
            let partial = case == "materialization-partial-frame-error";
            let text = case == "materialization-text-error";
            interpreter::run_with_policy(
                &ir,
                RuntimePolicy {
                    materialization: MaterializationLimits {
                        max_value_nodes: (!text).then_some(if partial {
                            (2 * (depth + 1) + 1) as u64
                        } else {
                            (depth + 1) as u64
                        }),
                        max_text_bytes: text.then_some(if records {
                            (depth / 2 * 8 + 2) as u64
                        } else {
                            2
                        }),
                    },
                    ..RuntimePolicy::UNLIMITED
                },
            )
        } else if case.starts_with("budget-") {
            use sovra::compiler::execution::ExecutionLimits;
            interpreter::run_with_limits(
                &ir,
                ExecutionLimits {
                    max_instructions: (case == "budget-instruction-error").then_some(depth as u64),
                    max_output_bytes: (case == "budget-output-error").then_some(1),
                },
            )
        } else if case == "host-error" {
            interpreter::run_with_host(&ir, &mut FailingHost(None))
        } else {
            interpreter::run(&ir)
        };
        if case.ends_with("error") || case == "binary-underflow" {
            assert!(result.is_err());
        } else if case == "display" {
            assert_eq!(result.unwrap(), [expected_length.to_string()]);
        } else {
            assert!(result.is_ok());
        }
    }
    println!("deep-case-success");
}
#[test]
fn deep_values_copy_display_replace_and_cleanup_without_host_stack_recursion() {
    for case in [
        "clone",
        "display",
        "pop",
        "frame-error",
        "builtin-error",
        "binary-underflow",
        "field-error",
        "widen-error",
        "replace",
        "javascript",
        "record-clone",
        "record-display",
        "record-pop",
        "record-builtin-error",
        "record-replace",
        "record-javascript",
        "return",
        "overwrite",
        "call",
        "host-error",
        "duplicate-field-error",
        "record-return",
        "record-overwrite",
        "record-call",
        "record-host-error",
        "record-duplicate-field-error",
        "budget-instruction-error",
        "budget-output-error",
        "record-budget-instruction-error",
        "record-budget-output-error",
        "materialization-copy-error",
        "materialization-parameter-error",
        "materialization-partial-frame-error",
        "materialization-text-error",
        "record-materialization-copy-error",
        "record-materialization-parameter-error",
        "record-materialization-partial-frame-error",
        "record-materialization-text-error",
        "control-host-error",
        "record-control-host-error",
        "slice",
        "slice-error",
        "record-slice",
        "record-slice-error",
        "append",
        "append-error",
        "concat",
        "concat-error",
        "record-append",
        "record-append-error",
        "record-concat",
        "record-concat-error",
    ] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "deep_child", "--nocapture"])
            .env("SOVRA_DEEP_RUNTIME_CASE", case)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{case}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("deep-case-success"));
    }
}

#[test]
fn public_values_keep_movable_fields_and_offer_explicit_iterative_cleanup() {
    let mut value = interpreter::Value::Int(7);
    for _ in 0..16000 {
        value = interpreter::Value::Array(vec![value]);
    }
    let copy = value.clone();
    if let interpreter::Value::Array(items) = copy {
        for item in items {
            item.drop_iterative();
        }
    } else {
        panic!("expected array");
    }
    value.drop_iterative();
}

#[test]
fn recursive_source_records_preserve_copy_equality_and_display_in_both_engines() {
    let source = "struct Node{children:[Node]} fn main(){let mut root=Node{children:[]};let mut index=0;while index<120 {root=Node{children:[root]};index=index+1;}let copy=root;print(copy==root);print(std::to_string(copy));}";
    let program = sovra::compiler::parser::Parser::new()
        .parse_source(source)
        .unwrap();
    let (ir, _) = sovra::compiler::compile_program(&program).unwrap();
    let mut text = "Node { children: [] }".to_owned();
    for _ in 0..120 {
        text = format!("Node {{ children: [{text}] }}");
    }
    assert_eq!(
        interpreter::run(&ir).unwrap(),
        ["true".to_owned(), text.clone()]
    );
    let js = std::process::Command::new("node")
        .args(["--eval", &backend::render_javascript(&ir)])
        .output()
        .unwrap();
    assert!(
        js.status.success(),
        "{}",
        String::from_utf8_lossy(&js.stderr)
    );
    assert_eq!(
        String::from_utf8(js.stdout).unwrap().trim(),
        format!("true\n{text}")
    );
}
