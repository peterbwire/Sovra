//! Approved model-v1 logical materialization accounting and real engine parity.
use sovra::compiler::{
    backend,
    execution::{ExecutionLimits, MaterializationLimits, RuntimePolicy},
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
fn policy(nodes: Option<u64>, text: Option<u64>) -> RuntimePolicy {
    RuntimePolicy {
        execution: ExecutionLimits::UNLIMITED,
        materialization: MaterializationLimits {
            max_value_nodes: nodes,
            max_text_bytes: text,
        },
    }
}
fn program(instructions: Vec<Instruction>) -> IrProgram {
    IrProgram {
        functions: vec![IrFunction {
            name: "main".into(),
            parameters: vec![],
            instructions,
        }],
    }
}
fn source(text: &str) -> IrProgram {
    let parsed = sovra::compiler::parser::Parser::new()
        .parse_source(text)
        .unwrap();
    sovra::compiler::compile_program(&parsed).unwrap().0
}
fn compare(ir: &IrProgram, limits: RuntimePolicy, expected: &str, error: Option<&str>) {
    let mut host = Host::default();
    let result = interpreter::run_with_host_status_and_policy(ir, &mut host, limits);
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
    let node = std::process::Command::new("node")
        .args([
            "--eval",
            &backend::render_javascript_with_policy(ir, limits),
        ])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(node.stdout).unwrap(), expected);
    assert_eq!(
        node.status.success(),
        error.is_none(),
        "{}",
        String::from_utf8_lossy(&node.stderr)
    );
    if let Some(error) = error {
        assert!(String::from_utf8_lossy(&node.stderr).contains(error));
    }
}
#[test]
fn approved_copy_print_example_has_exact_cost_and_preserves_prior_effects() {
    let ir = source("fn main(){let original=[\"λ\",\"😀\"];let copy=original;print(copy);}");
    compare(&ir, policy(Some(11), Some(28)), "[λ, 😀]\n", None);
    compare(
        &ir,
        policy(Some(10), Some(28)),
        "[λ, 😀]\n",
        Some("value node allowance"),
    );
    compare(
        &ir,
        policy(Some(11), Some(27)),
        "",
        Some("text materialization allowance"),
    );
    compare(&ir, RuntimePolicy::UNLIMITED, "[λ, 😀]\n", None);
}
#[test]
fn zero_empty_return_and_transfer_costs_are_explicit() {
    compare(
        &program(vec![]),
        policy(Some(0), Some(0)),
        "",
        Some("value node allowance"),
    );
    compare(&program(vec![]), policy(Some(1), Some(0)), "", None);
    let ir = program(vec![
        Instruction::LoadLiteral(Literal::String("λ".into())),
        Instruction::Return,
    ]);
    compare(&ir, policy(Some(1), Some(2)), "", None);
    compare(
        &ir,
        policy(Some(1), Some(1)),
        "",
        Some("text materialization allowance"),
    );
    compare(
        &ir,
        policy(Some(0), Some(0)),
        "",
        Some("value node allowance"),
    );
}

fn literal(value: &str) -> Instruction {
    Instruction::LoadLiteral(Literal::String(value.into()))
}
fn integer(value: &str) -> Instruction {
    Instruction::LoadLiteral(Literal::Integer(value.into()))
}
fn call(name: &str, arguments: usize) -> Instruction {
    Instruction::Call {
        name: name.into(),
        arguments,
    }
}
fn exact(instructions: Vec<Instruction>, nodes: u64, text: u64) {
    let mut instructions = instructions;
    instructions.push(Instruction::Return);
    let ir = program(instructions);
    compare(&ir, policy(Some(nodes), Some(text)), "", None);
    compare(
        &ir,
        policy(Some(nodes - 1), Some(text)),
        "",
        Some("value node allowance"),
    );
    if text > 0 {
        compare(
            &ir,
            policy(Some(nodes), Some(text - 1)),
            "",
            Some("text materialization allowance"),
        );
    }
}
#[test]
fn standard_pure_producers_follow_the_complete_table() {
    exact(vec![integer("7"), call("std::to_string", 1)], 2, 1);
    exact(vec![literal("λ😀"), call("std::to_string", 1)], 2, 12);
    exact(vec![literal("λ😀"), call("std::len", 1)], 2, 6);
    exact(
        vec![literal("a|b"), literal("|"), call("std::split_once", 2)],
        6,
        36,
    );
    exact(
        vec![literal("a|b"), literal(""), call("std::split_once", 2)],
        6,
        36,
    );
    exact(vec![literal("42"), call("std::parse_int", 1)], 4, 23);
    exact(vec![literal("bad"), call("std::parse_int", 1)], 4, 24);
    exact(vec![literal("a\na"), call("std::lines_unique", 1)], 2, 3);
    exact(vec![literal("\nλ"), call("std::escape_terminal", 1)], 2, 13);
    exact(
        vec![literal("\u{85}"), call("std::escape_terminal", 1)],
        2,
        10,
    );
    exact(
        vec![
            Instruction::MakeArray { length: 0 },
            call("std::array_len", 1),
        ],
        2,
        0,
    );
    exact(vec![call("std::arg_count", 0)], 1, 0);
    exact(vec![integer("0"), call("std::set_exit_code", 1)], 2, 0);
}
#[test]
fn snapshots_container_labels_and_transfers_are_charged_once() {
    exact(
        vec![literal("λ"), Instruction::MakeArray { length: 1 }],
        2,
        2,
    );
    exact(
        vec![
            literal("λ"),
            Instruction::MakeStruct {
                type_name: "R".into(),
                fields: vec!["x".into()],
            },
        ],
        2,
        4,
    );
    exact(
        vec![
            literal("λ"),
            Instruction::MakeStruct {
                type_name: "R".into(),
                fields: vec!["x".into()],
            },
            Instruction::LoadField("x".into()),
        ],
        3,
        6,
    );
    exact(
        vec![
            literal("λ"),
            Instruction::MakeArray { length: 1 },
            integer("0"),
            Instruction::Index,
        ],
        4,
        4,
    );
    exact(
        vec![
            literal("λ"),
            Instruction::StoreName("x".into()),
            Instruction::LoadName("x".into()),
        ],
        2,
        4,
    );
    exact(
        vec![literal("λ"), literal("😀"), Instruction::Binary("+".into())],
        3,
        12,
    );
    exact(
        vec![integer("1"), integer("2"), Instruction::Binary("+".into())],
        3,
        0,
    );
    exact(
        vec![integer("1"), integer("2"), Instruction::Binary("==".into())],
        3,
        0,
    );
    exact(vec![integer("1"), Instruction::WidenFloat], 1, 0);
    exact(
        vec![
            integer("1"),
            Instruction::MakeArray { length: 1 },
            Instruction::WidenFloatArray { depth: 1 },
        ],
        2,
        0,
    );
    exact(
        vec![
            integer("1"),
            Instruction::MakeArray { length: 1 },
            integer("0"),
            integer("9"),
            Instruction::StoreIndex,
        ],
        4,
        0,
    );
}
#[test]
fn overwrite_does_not_refund_and_parameter_snapshots_share_fuel() {
    exact(
        vec![
            literal("λ"),
            Instruction::StoreName("x".into()),
            literal("😀"),
            Instruction::StoreName("x".into()),
            Instruction::LoadName("x".into()),
        ],
        3,
        10,
    );
    let mut ir = program(vec![literal("λ"), call("identity", 1), Instruction::Return]);
    ir.functions.push(IrFunction {
        name: "identity".into(),
        parameters: vec!["value".into()],
        instructions: vec![Instruction::LoadName("value".into()), Instruction::Return],
    });
    compare(&ir, policy(Some(3), Some(6)), "", None);
    compare(
        &ir,
        policy(Some(2), Some(6)),
        "",
        Some("value node allowance"),
    );
    compare(
        &ir,
        policy(Some(3), Some(5)),
        "",
        Some("text materialization allowance"),
    );
}
#[test]
fn old_limits_composition_output_priority_and_large_exact_u64_are_preserved() {
    let ir = program(vec![literal("λ"), call("print", 1), Instruction::Return]);
    let mut limit = policy(Some(1), Some(2));
    limit.execution.max_output_bytes = Some(0);
    compare(&ir, limit, "", Some("output byte allowance"));
    limit.execution.max_output_bytes = Some(3);
    compare(&ir, limit, "", Some("value node allowance"));
    compare(&ir, policy(Some(2), Some(4)), "λ\n", None);
    compare(&ir, policy(Some(u64::MAX), Some(u64::MAX)), "λ\n", None);
    assert!(
        backend::render_javascript_with_policy(&ir, policy(Some(u64::MAX), Some(u64::MAX)))
            .contains("18446744073709551615n")
    );
}

#[test]
fn standard_host_results_are_preflighted_and_unknown_payload_failure_keeps_effects() {
    use sovra::compiler::text_files::{TextRead, TextWrite};
    #[derive(Default)]
    struct DataHost {
        calls: usize,
    }
    impl RuntimeHost for DataHost {
        fn arguments(&self) -> &[String] {
            &[]
        }
        fn read_line(&mut self) -> Result<Option<String>, String> {
            self.calls += 1;
            Ok(Some("λ".into()))
        }
        fn write_line(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn read_text(&mut self, _: &str) -> TextRead {
            self.calls += 1;
            TextRead {
                ok: true,
                text: "λ".into(),
                error: String::new(),
            }
        }
        fn write_text(&mut self, _: &str, _: &str) -> TextWrite {
            self.calls += 1;
            TextWrite {
                ok: false,
                error: "io".into(),
            }
        }
        fn write_text_if(&mut self, _: &str, _: bool, _: &str, _: &str) -> TextWrite {
            self.calls += 1;
            TextWrite {
                ok: false,
                error: "io".into(),
            }
        }
    }
    // Ingress is admitted after the host-produced payload size is known.
    for (ir, nodes, text, minimum) in [
        (
            program(vec![call("std::read_line", 0), Instruction::Return]),
            3,
            23,
            21,
        ),
        (
            program(vec![
                literal("p"),
                call("std::read_text", 1),
                Instruction::Return,
            ]),
            5,
            27,
            25,
        ),
        (
            program(vec![
                literal("p"),
                literal("x"),
                call("std::write_text", 2),
                Instruction::Return,
            ]),
            5,
            25,
            23,
        ),
        (
            program(vec![
                literal("p"),
                Instruction::LoadLiteral(Literal::Boolean(false)),
                literal(""),
                literal("x"),
                call("std::write_text_if", 4),
                Instruction::Return,
            ]),
            7,
            25,
            23,
        ),
    ] {
        let mut host = DataHost::default();
        assert!(interpreter::run_with_host_status_and_policy(
            &ir,
            &mut host,
            policy(Some(nodes), Some(text))
        )
        .is_ok());
        assert_eq!(host.calls, 1);
        let mut host = DataHost::default();
        assert!(interpreter::run_with_host_status_and_policy(
            &ir,
            &mut host,
            policy(Some(nodes), Some(minimum - 1))
        )
        .unwrap_err()
        .contains("text materialization allowance"));
        assert_eq!(host.calls, 0);
        let mut host = DataHost::default();
        assert!(interpreter::run_with_host_status_and_policy(
            &ir,
            &mut host,
            policy(Some(nodes), Some(text - 1))
        )
        .unwrap_err()
        .contains("text materialization allowance"));
        assert_eq!(host.calls, 1);
    }
}

#[test]
fn node_host_ingress_and_argument_snapshots_match_rust_exact_costs() {
    use sovra::compiler::text_files::{TextRead, TextWrite};
    struct DataHost {
        args: Vec<String>,
    }
    impl RuntimeHost for DataHost {
        fn arguments(&self) -> &[String] {
            &self.args
        }
        fn read_line(&mut self) -> Result<Option<String>, String> {
            Ok(Some("λ".into()))
        }
        fn write_line(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn read_text(&mut self, _: &str) -> TextRead {
            TextRead {
                ok: true,
                text: "λ".into(),
                error: String::new(),
            }
        }
        fn write_text(&mut self, _: &str, _: &str) -> TextWrite {
            TextWrite {
                ok: false,
                error: "io".into(),
            }
        }
        fn write_text_if(&mut self, _: &str, _: bool, _: &str, _: &str) -> TextWrite {
            TextWrite {
                ok: false,
                error: "io".into(),
            }
        }
    }
    for (ir, nodes, text) in [
        (
            program(vec![integer("0"), call("std::arg", 1), Instruction::Return]),
            2,
            6,
        ),
        (
            program(vec![call("std::read_line", 0), Instruction::Return]),
            3,
            23,
        ),
        (
            program(vec![
                literal("p"),
                call("std::read_text", 1),
                Instruction::Return,
            ]),
            5,
            27,
        ),
        (
            program(vec![
                literal("p"),
                literal("x"),
                call("std::write_text", 2),
                Instruction::Return,
            ]),
            5,
            25,
        ),
        (
            program(vec![
                literal("p"),
                Instruction::LoadLiteral(Literal::Boolean(false)),
                literal(""),
                literal("x"),
                call("std::write_text_if", 4),
                Instruction::Return,
            ]),
            7,
            25,
        ),
    ] {
        for (limit, success) in [
            (policy(Some(nodes), Some(text)), true),
            (policy(Some(nodes - 1), Some(text)), false),
            (policy(Some(nodes), Some(text - 1)), false),
        ] {
            let rust = interpreter::run_with_host_status_and_policy(
                &ir,
                &mut DataHost {
                    args: vec!["λ😀".into()],
                },
                limit,
            );
            let js = backend::render_javascript_with_policy(&ir, limit)
                .replace(
                    "const svrArgv = process.argv.slice(2);",
                    "const svrArgv = [\"λ😀\"];",
                )
                .replace(
                    "svrFunctions.main();",
                    r#"
svrReadLine = () => svrResult("std::InputLine", {eof:false,text:"λ"});
svrReadText = () => svrResult("std::TextRead", {ok:true,text:"λ",error:""});
svrWriteText = () => svrResult("std::TextWrite", {ok:false,error:"io"});
svrWriteTextIf = () => svrResult("std::TextWrite", {ok:false,error:"io"});
svrFunctions.main();
"#,
                );
            let node = std::process::Command::new("node")
                .args(["--eval", &js])
                .output()
                .unwrap();
            assert_eq!(rust.is_ok(), success, "{rust:?}");
            assert_eq!(
                node.status.success(),
                success,
                "{}",
                String::from_utf8_lossy(&node.stderr)
            );
        }
    }
}

#[test]
fn rejected_creation_leaves_both_node_counters_unchanged() {
    let ir = program(vec![]);
    let js = backend::render_javascript_with_policy(&ir, policy(Some(2), Some(1)));
    let js=js+"\ntry { svrMaterialize(1n,2n); } catch {} svrMaterialize(1n,1n); console.log(String(svrNodeRemaining)+','+String(svrTextRemaining));";
    let node = std::process::Command::new("node")
        .args(["--eval", &js])
        .output()
        .unwrap();
    assert!(
        node.status.success(),
        "{}",
        String::from_utf8_lossy(&node.stderr)
    );
    assert_eq!(String::from_utf8(node.stdout).unwrap().trim(), "0,0");
}

#[test]
fn operation_validation_precedes_new_admission_and_invalid_literals_stay_runtime_errors() {
    for (instructions, error) in [
        (
            vec![Instruction::LoadLiteral(Literal::Integer("bad".into()))],
            "invalid or out-of-range integer literal",
        ),
        (
            vec![integer("0"), integer("0"), Instruction::Binary("/".into())],
            "division by zero",
        ),
        (
            vec![
                integer("0"),
                Instruction::MakeArray { length: 1 },
                integer("9"),
                Instruction::Index,
            ],
            "array index out of bounds",
        ),
        (
            vec![
                integer("0"),
                integer("0"),
                Instruction::MakeStruct {
                    type_name: "R".into(),
                    fields: vec!["x".into(), "x".into()],
                },
            ],
            "duplicate field",
        ),
    ] {
        compare(
            &program(instructions),
            RuntimePolicy::UNLIMITED,
            "",
            Some(error),
        );
    }
    // No fuel remains after the valid operands, but division validation still wins.
    compare(
        &program(vec![
            integer("1"),
            integer("0"),
            Instruction::Binary("/".into()),
        ]),
        policy(Some(2), Some(0)),
        "",
        Some("division by zero"),
    );
    compare(
        &program(vec![Instruction::LoadLiteral(Literal::Integer(
            "bad".into(),
        ))]),
        policy(Some(0), Some(0)),
        "",
        Some("invalid or out-of-range integer literal"),
    );
}

#[test]
fn registered_library_has_bounded_execution_and_no_unlimited_fallback() {
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
    ))]
    {
        let package = sovra::compiler::project::packages::compile_registered_records(
            "examples/typed-array-packages/app",
        )
        .unwrap();
        compare(
            package.ir(),
            policy(Some(1000), Some(10000)),
            "9\n1.5\n0\n1\n4.5\n",
            None,
        );
        compare(
            package.ir(),
            policy(Some(0), Some(0)),
            "",
            Some("value node allowance"),
        );
    }
}

#[test]
fn generated_host_side_effects_obey_minimum_and_post_result_admission() {
    let ir = program(vec![
        literal("p"),
        literal("x"),
        call("std::write_text", 2),
        Instruction::Return,
    ]);
    for (text, expected_calls, error) in [(22, 0, true), (24, 1, true), (25, 1, false)] {
        let script=backend::render_javascript_with_policy(&ir,policy(Some(5),Some(text))).replace("svrFunctions.main();",r#"
let hostCalls = 0;
svrWriteText = () => {hostCalls++; return svrResult("std::TextWrite", {ok:false,error:"io"});};
let failure = false;
try {svrFunctions.main();} catch (error) {failure = error.message.includes("text materialization allowance");}
console.log(JSON.stringify([hostCalls,failure]));
"#);
        let node = std::process::Command::new("node")
            .args(["--eval", &script])
            .output()
            .unwrap();
        assert!(
            node.status.success(),
            "{}",
            String::from_utf8_lossy(&node.stderr)
        );
        assert_eq!(
            String::from_utf8(node.stdout).unwrap().trim(),
            format!("[{expected_calls},{error}]")
        );
    }
}

#[test]
fn model_version_and_actual_node_snapshot_footprints_are_explicit() {
    assert_eq!(sovra::compiler::execution::MATERIALIZATION_MODEL_VERSION, 1);
    let script = backend::render_javascript_with_policy(&program(vec![]), RuntimePolicy::UNLIMITED)
        + r#"
const packet = svrResult("Packet", {items:["λ","😀"],ok:true});
const size = svrFootprint(packet);
console.log([size.nodes,size.text,size.display].map(String).join(","));
"#;
    assert!(script.contains("const svrMaterializationModelVersion = 1;"));
    let node = std::process::Command::new("node")
        .args(["--eval", &script])
        .output()
        .unwrap();
    assert!(node.status.success());
    assert_eq!(String::from_utf8(node.stdout).unwrap().trim(), "5,19,38");
}

#[test]
fn input_eof_empty_line_and_host_errors_obey_admission_order_in_both_engines() {
    struct InputHost {
        result: Result<Option<String>, String>,
        calls: usize,
    }
    impl RuntimeHost for InputHost {
        fn arguments(&self) -> &[String] {
            &[]
        }
        fn read_line(&mut self) -> Result<Option<String>, String> {
            self.calls += 1;
            self.result.clone()
        }
        fn write_line(&mut self, _: &str) -> Result<(), String> {
            panic!("unexpected output")
        }
    }
    let ir = program(vec![call("std::read_line", 0), Instruction::Return]);
    for result in [
        Ok(None),
        Ok(Some(String::new())),
        Err("input unavailable".into()),
    ] {
        for (nodes, text, expected_calls, expected_error) in [
            (2, 21, 0, Some("execution value node allowance exceeded")),
            (
                3,
                20,
                0,
                Some("execution text materialization allowance exceeded"),
            ),
            (3, 21, 1, result.as_ref().err().map(String::as_str)),
        ] {
            let limits = policy(Some(nodes), Some(text));
            let mut host = InputHost {
                result: result.clone(),
                calls: 0,
            };
            let outcome = interpreter::run_with_host_status_and_policy(&ir, &mut host, limits);
            assert_eq!(host.calls, expected_calls);
            assert_eq!(outcome.is_ok(), expected_error.is_none(), "{outcome:?}");
            if let Some(error) = expected_error {
                assert!(outcome.unwrap_err().contains(error));
            }
            let input = match &result {
                Ok(None) => "return svrResult('std::InputLine', {eof:true,text:''});",
                Ok(Some(_)) => "return svrResult('std::InputLine', {eof:false,text:''});",
                Err(_) => "throw new Error('input unavailable');",
            };
            let script = backend::render_javascript_with_policy(&ir, limits).replace(
                "svrFunctions.main();",
                &format!(
                    r#"
let calls = 0, error = null, answer;
svrReadLine = () => {{ calls++; {input} }};
try {{ answer = svrFunctions.main(); }} catch (failure) {{ error = failure.message; }}
console.log(JSON.stringify({{calls,error}}));
if (answer !== undefined) console.log(svrDisplay(answer));
"#
                ),
            );
            let node = std::process::Command::new("node")
                .args(["--eval", &script])
                .output()
                .unwrap();
            assert!(
                node.status.success(),
                "{}",
                String::from_utf8_lossy(&node.stderr)
            );
            let expected = format!(
                "{{\"calls\":{expected_calls},\"error\":{}}}\n",
                expected_error.map_or("null".into(), |e| format!("\"{e}\""))
            );
            let mut expected = expected;
            if expected_error.is_none() {
                expected.push_str(if result == Ok(None) {
                    "std::InputLine { eof: true, text:  }\n"
                } else {
                    "std::InputLine { eof: false, text:  }\n"
                });
            }
            assert_eq!(String::from_utf8(node.stdout).unwrap(), expected);
        }
    }
}

#[test]
fn javascript_deep_snapshot_admission_is_iterative_and_atomic() {
    // Construct a host graph directly to exercise helper depth without generating
    // a huge JavaScript function. Both plain arrays and alternating records matter.
    let mut script = backend::render_javascript(&program(vec![]));
    script.push_str(r#"
for (const records of [false, true]) {
  let value = "λ";
  for (let index = 0; index < 16000; index++) {
    value = records && index % 2 === 0
      ? svrResult("Node", {next:value}) : [value];
  }
  const size = svrFootprint(value);
  if (size.nodes !== 16001n || size.text !== (records ? 64002n : 2n)) throw new Error("bad footprint");
  for (const dimension of ["nodes", "text"]) {
    svrNodeRemaining = size.nodes - (dimension === "nodes" ? 1n : 0n);
    svrTextRemaining = size.text - (dimension === "text" ? 1n : 0n);
    const nodesBefore = svrNodeRemaining, textBefore = svrTextRemaining;
    let copy, error;
    try { copy = svrCopy(value); } catch (failure) { error = failure.message; }
    const expected = dimension === "nodes" ? "execution value node allowance exceeded" : "execution text materialization allowance exceeded";
    if (error !== expected || copy !== undefined) throw new Error("snapshot was published");
    if (svrNodeRemaining !== nodesBefore || svrTextRemaining !== textBefore) throw new Error("partial charge");
  }
  svrNodeRemaining = size.nodes;
  svrTextRemaining = size.text;
  const copy = svrCopy(value);
  if (copy === value || svrNodeRemaining !== 0n || svrTextRemaining !== 0n) throw new Error("copy cost");
  if (svrDisplay(copy) !== svrDisplay(value)) throw new Error("copy content");
}
console.log("deep-admission-success");
"#);
    let node = std::process::Command::new("node")
        .args(["--stack-size=256", "--eval", &script])
        .output()
        .unwrap();
    assert!(
        node.status.success(),
        "{}",
        String::from_utf8_lossy(&node.stderr)
    );
    assert_eq!(
        String::from_utf8(node.stdout).unwrap(),
        "deep-admission-success\n"
    );
}
