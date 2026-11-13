//! Cooperative Rust embedding controls: effects, priority and supervisor cancellation.
use sovra::compiler::{
    execution::{
        CancellationToken, ExecutionControl, ExecutionLimits, MaterializationLimits, RuntimePolicy,
    },
    interpreter::{self, RuntimeHost},
    ir::{Instruction, IrFunction, IrProgram, Literal},
    text_files::{TextRead, TextWrite},
};
use std::time::Duration;

fn program(instructions: Vec<Instruction>) -> IrProgram {
    IrProgram {
        functions: vec![IrFunction {
            name: "main".into(),
            parameters: vec![],
            instructions,
        }],
    }
}
fn control(token: &CancellationToken) -> ExecutionControl {
    ExecutionControl {
        cancellation: Some(token.clone()),
        max_elapsed: None,
    }
}
fn call(name: &str, arguments: usize) -> Instruction {
    Instruction::Call {
        name: name.into(),
        arguments,
    }
}
fn text(value: &str) -> Instruction {
    Instruction::LoadLiteral(Literal::String(value.into()))
}

#[test]
fn empty_and_precancelled_runs_preserve_validation_and_control_priority() {
    let token = CancellationToken::new();
    token.cancel();
    let empty = program(vec![]);
    let zero = RuntimePolicy {
        execution: ExecutionLimits {
            max_instructions: Some(0),
            max_output_bytes: Some(0),
        },
        materialization: MaterializationLimits {
            max_value_nodes: Some(0),
            max_text_bytes: Some(0),
        },
    };
    assert_eq!(
        interpreter::run_with_policy_and_control(
            &empty,
            zero,
            ExecutionControl {
                cancellation: Some(token.clone()),
                max_elapsed: Some(Duration::ZERO),
            }
        )
        .unwrap_err(),
        "execution cancelled"
    );
    assert_eq!(
        interpreter::run_with_policy_and_control(
            &empty,
            zero,
            ExecutionControl {
                cancellation: None,
                max_elapsed: Some(Duration::ZERO),
            }
        )
        .unwrap_err(),
        "execution elapsed time allowance exceeded"
    );
    let mut invalid = empty.clone();
    invalid.functions.push(invalid.functions[0].clone());
    let expected = interpreter::run(&invalid).unwrap_err();
    assert_eq!(
        interpreter::run_with_policy_and_control(&invalid, zero, control(&token)).unwrap_err(),
        expected
    );
    let mut host = EffectHost::new(token.clone());
    assert_eq!(
        interpreter::run_function_with_host_status_and_policy_and_control(
            &empty,
            "missing",
            &mut host,
            zero,
            control(&token),
        )
        .unwrap_err(),
        "entry function `missing` was not found"
    );
    let mut with_parameter = empty.clone();
    with_parameter.functions[0].parameters.push("x".into());
    assert!(
        interpreter::run_with_policy_and_control(&with_parameter, zero, control(&token))
            .unwrap_err()
            .contains("expects 1 argument")
    );
    // Dispatch priority is also checked for a nonempty run.
    assert_eq!(
        interpreter::run_with_policy_and_control(&program(vec![text("x")]), zero, control(&token))
            .unwrap_err(),
        "execution cancelled"
    );
    assert!(host.effects.is_empty());
}

#[test]
fn disabled_control_and_large_duration_preserve_policy_and_fresh_invocations() {
    let ir = program(vec![text("λ"), call("print", 1), Instruction::Pop]);
    let policy = RuntimePolicy {
        execution: ExecutionLimits {
            max_instructions: Some(3),
            max_output_bytes: Some(3),
        },
        materialization: MaterializationLimits {
            max_value_nodes: Some(3),
            max_text_bytes: Some(4),
        },
    };
    for settings in [
        ExecutionControl::UNLIMITED,
        ExecutionControl {
            cancellation: None,
            max_elapsed: Some(Duration::MAX),
        },
    ] {
        for _ in 0..2 {
            assert_eq!(
                interpreter::run_with_policy_and_control(&ir, policy, settings.clone()).unwrap(),
                ["λ"]
            );
        }
    }
    let token = CancellationToken::new();
    let settings = control(&token);
    assert!(interpreter::run_with_policy_and_control(&ir, policy, settings.clone()).is_ok());
    token.cancel();
    for _ in 0..2 {
        assert_eq!(
            interpreter::run_with_policy_and_control(&ir, policy, settings.clone()).unwrap_err(),
            "execution cancelled"
        );
    }
}

struct EffectHost {
    token: CancellationToken,
    effects: Vec<&'static str>,
    fail: bool,
    arguments: Vec<String>,
}
impl EffectHost {
    fn new(token: CancellationToken) -> Self {
        Self {
            token,
            effects: vec![],
            fail: false,
            arguments: vec!["argument".into()],
        }
    }
    fn effect(&mut self, name: &'static str) {
        self.effects.push(name);
        self.token.cancel();
    }
}
impl RuntimeHost for EffectHost {
    fn arguments(&self) -> &[String] {
        self.token.cancel();
        &self.arguments
    }
    fn read_line(&mut self) -> Result<Option<String>, String> {
        self.effect("read_line");
        if self.fail {
            Err("read failure".into())
        } else {
            Ok(Some("oversized λ".into()))
        }
    }
    fn write_line(&mut self, _: &str) -> Result<(), String> {
        self.effect("write_line");
        if self.fail {
            Err("write failure".into())
        } else {
            Ok(())
        }
    }
    fn read_text(&mut self, _: &str) -> TextRead {
        self.effect("read_text");
        TextRead {
            ok: true,
            text: "oversized λ".into(),
            error: String::new(),
        }
    }
    fn write_text(&mut self, _: &str, _: &str) -> TextWrite {
        self.effect("write_text");
        TextWrite {
            ok: false,
            error: "failure payload".into(),
        }
    }
    fn write_text_if(&mut self, _: &str, _: bool, _: &str, _: &str) -> TextWrite {
        self.effect("write_text_if");
        TextWrite {
            ok: false,
            error: "failure payload".into(),
        }
    }
}

#[test]
fn successful_host_callbacks_preserve_effects_but_refuse_publication_before_payload_admission() {
    for (instructions, effect, nodes, bytes) in [
        (vec![call("std::read_line", 0)], "read_line", 3, 21),
        (
            vec![text("p"), call("std::read_text", 1)],
            "read_text",
            5,
            25,
        ),
        (
            vec![text("p"), text("x"), call("std::write_text", 2)],
            "write_text",
            5,
            23,
        ),
        (
            vec![
                text("p"),
                Instruction::LoadLiteral(Literal::Boolean(false)),
                text(""),
                text("x"),
                call("std::write_text_if", 4),
            ],
            "write_text_if",
            7,
            23,
        ),
        (vec![text("λ"), call("print", 1)], "write_line", 2, 4),
    ] {
        let token = CancellationToken::new();
        let mut host = EffectHost::new(token.clone());
        let policy = RuntimePolicy {
            materialization: MaterializationLimits {
                max_value_nodes: Some(nodes),
                max_text_bytes: Some(bytes),
            },
            ..RuntimePolicy::UNLIMITED
        };
        let result = interpreter::run_with_host_status_and_policy_and_control(
            &program(instructions),
            &mut host,
            policy,
            control(&token),
        );
        assert_eq!(result.unwrap_err(), "execution cancelled", "{effect}");
        assert_eq!(host.effects, [effect]);
    }
    // arguments() is a trusted callback too, despite borrowing instead of allocating.
    for instructions in [
        vec![call("std::arg_count", 0)],
        vec![
            Instruction::LoadLiteral(Literal::Integer("0".into())),
            call("std::arg", 1),
        ],
    ] {
        let token = CancellationToken::new();
        let mut host = EffectHost::new(token.clone());
        assert_eq!(
            interpreter::run_with_host_status_and_policy_and_control(
                &program(instructions),
                &mut host,
                RuntimePolicy::UNLIMITED,
                control(&token)
            )
            .unwrap_err(),
            "execution cancelled"
        );
    }
}

#[test]
fn failed_host_callbacks_keep_their_error_even_when_they_cancel() {
    for (instructions, expected) in [
        (vec![call("std::read_line", 0)], "read failure"),
        (vec![text("line"), call("print", 1)], "write failure"),
    ] {
        let token = CancellationToken::new();
        let mut host = EffectHost::new(token.clone());
        host.fail = true;
        assert_eq!(
            interpreter::run_with_host_status_and_policy_and_control(
                &program(instructions),
                &mut host,
                RuntimePolicy::UNLIMITED,
                control(&token)
            )
            .unwrap_err(),
            expected
        );
        assert_eq!(host.effects.len(), 1);
        assert!(token.is_cancelled());
    }
}

#[test]
fn child_callee_loop() {
    let Ok(mode) = std::env::var("SOVRA_CONTROL_LOOP_CHILD") else {
        return;
    };
    if mode == "elapsed" {
        let mut ir = program(vec![call("worker", 0)]);
        ir.functions.push(IrFunction {
            name: "worker".into(),
            parameters: vec![],
            instructions: vec![Instruction::Jump { target: 0 }],
        });
        assert_eq!(
            interpreter::run_with_policy_and_control(
                &ir,
                RuntimePolicy::UNLIMITED,
                ExecutionControl {
                    cancellation: None,
                    max_elapsed: Some(Duration::from_millis(10))
                },
            )
            .unwrap_err(),
            "execution elapsed time allowance exceeded"
        );
        println!("elapsed-callee-success");
        return;
    }
    let token = CancellationToken::new();
    let (started_tx, started_rx) = std::sync::mpsc::sync_channel(0);
    let worker_token = token.clone();
    let worker = std::thread::spawn(move || {
        struct SignallingHost(std::sync::mpsc::SyncSender<()>);
        impl RuntimeHost for SignallingHost {
            fn arguments(&self) -> &[String] {
                &[]
            }
            fn read_line(&mut self) -> Result<Option<String>, String> {
                unreachable!()
            }
            fn write_line(&mut self, _: &str) -> Result<(), String> {
                self.0.send(()).unwrap();
                Ok(())
            }
        }
        let mut ir = program(vec![call("worker", 0)]);
        ir.functions.push(IrFunction {
            name: "worker".into(),
            parameters: vec![],
            instructions: vec![
                text("started"),
                call("print", 1),
                Instruction::Jump { target: 2 },
            ],
        });
        interpreter::run_with_host_status_and_policy_and_control(
            &ir,
            &mut SignallingHost(started_tx),
            RuntimePolicy::UNLIMITED,
            control(&worker_token),
        )
    });
    started_rx.recv().unwrap();
    token.cancel();
    assert_eq!(worker.join().unwrap().unwrap_err(), "execution cancelled");
    assert!(token.clone().is_cancelled());
    println!("cancelled-callee-success");
}

#[test]
fn supervisor_cancels_callee_loop_with_a_test_harness_kill_deadline() {
    use std::process::{Command, Stdio};
    for mode in ["cancelled", "elapsed"] {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "child_callee_loop", "--nocapture"])
            .env("SOVRA_CONTROL_LOOP_CHILD", mode)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let start = std::time::Instant::now();
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if start.elapsed() > Duration::from_secs(20) {
                child.kill().unwrap();
                let output = child.wait_with_output().unwrap();
                panic!(
                    "cancellation child exceeded test deadline: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains(&format!("{mode}-callee-success")));
    }
}
