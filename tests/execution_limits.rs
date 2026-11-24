//! Opt-in deterministic instruction/output counters and Rust/JS parity.
use sovra::compiler::{
    backend,
    execution::ExecutionLimits,
    interpreter::{self, RuntimeHost},
    ir::{Instruction, IrFunction, IrProgram, Literal},
};
#[derive(Default)]
struct Host {
    lines: Vec<String>,
}
impl RuntimeHost for Host {
    fn arguments(&self) -> &[String] {
        &[]
    }
    fn read_line(&mut self) -> Result<Option<String>, String> {
        Err("unexpected input".into())
    }
    fn write_line(&mut self, line: &str) -> Result<(), String> {
        self.lines.push(line.into());
        Ok(())
    }
}
fn program(text: &str) -> IrProgram {
    IrProgram {
        functions: vec![IrFunction {
            name: "main".into(),
            parameters: vec![],
            instructions: vec![
                Instruction::LoadLiteral(Literal::String(text.into())),
                Instruction::Call {
                    name: "print".into(),
                    arguments: 1,
                },
                Instruction::Pop,
                Instruction::Return,
            ],
        }],
    }
}
fn javascript_output(script: String) -> std::process::Output {
    use std::{
        process::Stdio,
        time::{Duration, Instant},
    };
    let mut child = std::process::Command::new("node")
        .args(["--eval", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("JavaScript budget regression exceeded its test deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn compare(ir: &IrProgram, limits: ExecutionLimits, expected: &str, error: Option<&str>) {
    let mut host = Host::default();
    let result = interpreter::run_with_host_status_and_limits(ir, &mut host, limits);
    assert_eq!(
        host.lines
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
        expected
    );
    if let Some(error) = error {
        assert!(result.unwrap_err().contains(error));
    } else {
        assert!(result.is_ok(), "{result:?}");
    }
    let js = javascript_output(backend::render_javascript_with_limits(ir, limits));
    assert_eq!(String::from_utf8(js.stdout).unwrap(), expected);
    assert_eq!(
        js.status.success(),
        error.is_none(),
        "{}",
        String::from_utf8_lossy(&js.stderr)
    );
    if let Some(error) = error {
        assert!(String::from_utf8_lossy(&js.stderr).contains(error));
    }
}
#[test]
fn zero_exact_exceeded_and_unlimited_instruction_counts_precede_effects() {
    let ir = program("ok");
    for (instructions, output, error) in [
        (0, "", Some("instruction allowance")),
        (1, "", Some("instruction allowance")),
        (3, "ok\n", Some("instruction allowance")),
        (4, "ok\n", None),
    ] {
        compare(
            &ir,
            ExecutionLimits {
                max_instructions: Some(instructions),
                ..ExecutionLimits::UNLIMITED
            },
            output,
            error,
        );
    }
    compare(&ir, ExecutionLimits::UNLIMITED, "ok\n", None);
}
#[test]
fn unicode_and_lf_output_are_admitted_before_host_writes() {
    let ir = program("λ😀");
    for bytes in [0, 1, 6] {
        compare(
            &ir,
            ExecutionLimits {
                max_output_bytes: Some(bytes),
                ..ExecutionLimits::UNLIMITED
            },
            "",
            Some("output byte allowance"),
        );
    }
    compare(
        &ir,
        ExecutionLimits {
            max_output_bytes: Some(7),
            ..ExecutionLimits::UNLIMITED
        },
        "λ😀\n",
        None,
    );
    compare(
        &program(""),
        ExecutionLimits {
            max_output_bytes: Some(1),
            ..ExecutionLimits::UNLIMITED
        },
        "\n",
        None,
    );
}
#[test]
fn a_loop_stops_at_the_instruction_allowance() {
    let ir = IrProgram {
        functions: vec![IrFunction {
            name: "main".into(),
            parameters: vec![],
            instructions: vec![Instruction::Jump { target: 0 }],
        }],
    };
    let limits = ExecutionLimits {
        max_instructions: Some(100),
        ..ExecutionLimits::UNLIMITED
    };
    // The harness must also fail promptly if the instruction hook regresses.
    // An unjoined test worker ends with this test process; it cannot outlive it.
    let (sender, receiver) = std::sync::mpsc::channel();
    let cloned = ir.clone();
    std::thread::spawn(move || {
        let _ = sender.send(interpreter::run_with_limits(&cloned, limits));
    });
    let result = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("Rust budget regression exceeded its test deadline");
    assert!(result.unwrap_err().contains("instruction allowance"));
    let js = javascript_output(backend::render_javascript_with_limits(&ir, limits));
    assert!(!js.status.success());
    assert!(js.stdout.is_empty());
    assert!(String::from_utf8_lossy(&js.stderr).contains("instruction allowance"));
}
#[test]
fn u64_allowances_do_not_round_through_javascript_number() {
    let limits = ExecutionLimits {
        max_instructions: Some(u64::MAX),
        max_output_bytes: Some(u64::MAX),
    };
    assert!(
        backend::render_javascript_with_limits(&program("ok"), limits)
            .contains("18446744073709551615n")
    );
    compare(&program("ok"), limits, "ok\n", None);
}

#[test]
fn counters_are_shared_across_callees_and_prior_lines_remain_on_failure() {
    let ir = IrProgram {
        functions: vec![
            IrFunction {
                name: "main".into(),
                parameters: vec![],
                instructions: vec![
                    Instruction::Call {
                        name: "helper".into(),
                        arguments: 0,
                    },
                    Instruction::Pop,
                    Instruction::Call {
                        name: "helper".into(),
                        arguments: 0,
                    },
                    Instruction::Pop,
                    Instruction::Return,
                ],
            },
            IrFunction {
                name: "helper".into(),
                parameters: vec![],
                instructions: vec![
                    Instruction::LoadLiteral(Literal::String("λ".into())),
                    Instruction::Call {
                        name: "std::println".into(),
                        arguments: 1,
                    },
                    Instruction::Return,
                ],
            },
        ],
    };
    compare(
        &ir,
        ExecutionLimits {
            max_instructions: Some(11),
            max_output_bytes: Some(6),
        },
        "λ\nλ\n",
        None,
    );
    compare(
        &ir,
        ExecutionLimits {
            max_instructions: Some(10),
            max_output_bytes: None,
        },
        "λ\nλ\n",
        Some("instruction allowance"),
    );
    compare(
        &ir,
        ExecutionLimits {
            max_instructions: None,
            max_output_bytes: Some(3),
        },
        "λ\n",
        Some("output byte allowance"),
    );
    // A fresh invocation starts a fresh allowance; failed runs do not leak state.
    compare(
        &ir,
        ExecutionLimits {
            max_instructions: Some(11),
            max_output_bytes: Some(6),
        },
        "λ\nλ\n",
        None,
    );
}

#[test]
fn bounded_compound_formatting_preserves_exact_utf8_and_does_not_publish_prefixes() {
    let source = "struct Value{items:[String]}fn main(){print(Value{items:[\"λ\",\"😀\"]});}";
    let parsed = sovra::compiler::parser::Parser::new()
        .parse_source(source)
        .unwrap();
    let (ir, _) = sovra::compiler::compile_program(&parsed).unwrap();
    let output = "Value { items: [λ, 😀] }\n";
    compare(
        &ir,
        ExecutionLimits {
            max_output_bytes: Some(output.len() as u64),
            ..ExecutionLimits::UNLIMITED
        },
        output,
        None,
    );
    for bytes in [1, 7, output.len() as u64 - 1] {
        compare(
            &ir,
            ExecutionLimits {
                max_output_bytes: Some(bytes),
                ..ExecutionLimits::UNLIMITED
            },
            "",
            Some("output byte allowance"),
        );
    }
}

#[test]
fn host_failures_are_not_hidden_and_legacy_entry_points_remain_compatible() {
    struct FailingHost;
    impl RuntimeHost for FailingHost {
        fn arguments(&self) -> &[String] {
            &[]
        }
        fn read_line(&mut self) -> Result<Option<String>, String> {
            Err("unexpected input".into())
        }
        fn write_line(&mut self, _: &str) -> Result<(), String> {
            Err("host failure".into())
        }
    }
    let ir = program("ok");
    let limited = interpreter::run_with_host_status_and_limits(
        &ir,
        &mut FailingHost,
        ExecutionLimits {
            max_instructions: Some(4),
            max_output_bytes: Some(3),
        },
    )
    .unwrap_err();
    assert_eq!(limited, "host failure");
    assert_eq!(interpreter::run(&ir).unwrap(), ["ok"]);
    assert_eq!(
        interpreter::run_with_limits(&ir, ExecutionLimits::UNLIMITED).unwrap(),
        ["ok"]
    );
    let mut selected = program("wrong");
    let mut other = program("chosen").functions.remove(0);
    other.name = "chosen".into();
    selected.functions.push(other);
    let mut host = Host::default();
    assert!(interpreter::run_function_with_host_status_and_limits(
        &selected,
        "chosen",
        &mut host,
        ExecutionLimits {
            max_instructions: Some(4),
            max_output_bytes: Some(7)
        }
    )
    .is_ok());
    assert_eq!(host.lines, ["chosen"]);
}

#[test]
fn invalid_ir_is_rejected_before_optional_execution() {
    let mut ir = program("never");
    ir.functions.push(ir.functions[0].clone());
    compare(
        &ir,
        ExecutionLimits {
            max_instructions: Some(0),
            max_output_bytes: Some(0),
        },
        "",
        Some("duplicate IR function"),
    );
    let empty = IrProgram {
        functions: vec![IrFunction {
            name: "main".into(),
            parameters: vec![],
            instructions: vec![],
        }],
    };
    compare(
        &empty,
        ExecutionLimits {
            max_instructions: Some(0),
            max_output_bytes: Some(0),
        },
        "",
        None,
    );
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
fn independent_registered_package_runs_with_explicit_component_limits() {
    let package = sovra::compiler::project::packages::compile_registered_records(
        "examples/typed-array-packages/app",
    )
    .unwrap();
    compare(
        package.ir(),
        ExecutionLimits {
            max_instructions: Some(1000),
            max_output_bytes: Some(14),
        },
        "9\n1.5\n0\n1\n4.5\n",
        None,
    );
    compare(
        package.ir(),
        ExecutionLimits {
            max_instructions: Some(1000),
            max_output_bytes: Some(13),
        },
        "9\n1.5\n0\n1\n",
        Some("output byte allowance"),
    );
}

#[test]
fn typed_parameter_conversion_instructions_count_and_fallthrough_is_free() {
    let parsed = sovra::compiler::parser::Parser::new()
        .parse_source(
            "fn first(items:[Float])->Float{return items[0];}fn main(){print(first([1]));}",
        )
        .unwrap();
    let (ir, _) = sovra::compiler::compile_program(&parsed).unwrap();
    // Both functions execute once, without jumps; every emitted instruction counts.
    let instructions = ir
        .functions
        .iter()
        .map(|function| function.instructions.len() as u64)
        .sum::<u64>();
    compare(
        &ir,
        ExecutionLimits {
            max_instructions: Some(instructions),
            max_output_bytes: Some(2),
        },
        "1\n",
        None,
    );
    compare(
        &ir,
        ExecutionLimits {
            max_instructions: Some(instructions - 1),
            max_output_bytes: Some(2),
        },
        "1\n",
        Some("instruction allowance"),
    );
}

#[test]
fn empty_lines_consume_the_shared_output_allowance() {
    let parsed = sovra::compiler::parser::Parser::new()
        .parse_source("fn main(){print(\"\");print(\"\");}")
        .unwrap();
    let (ir, _) = sovra::compiler::compile_program(&parsed).unwrap();
    compare(
        &ir,
        ExecutionLimits {
            max_output_bytes: Some(2),
            ..ExecutionLimits::UNLIMITED
        },
        "\n\n",
        None,
    );
    compare(
        &ir,
        ExecutionLimits {
            max_output_bytes: Some(1),
            ..ExecutionLimits::UNLIMITED
        },
        "\n",
        Some("output byte allowance"),
    );
}
