//! Private deterministic clocks verify checkpoint placement without sleeping.
use super::*;
use crate::compiler::execution::{CancellationToken, MaterializationLimits, ELAPSED_ERROR};
use std::{cell::Cell, rc::Rc, time::Duration};

fn function(instructions: Vec<Instruction>) -> IrFunction {
    IrFunction {
        name: "main".into(),
        parameters: vec![],
        instructions,
    }
}
fn settings() -> ExecutionControl {
    ExecutionControl {
        max_elapsed: Some(Duration::from_secs(1)),
        ..ExecutionControl::UNLIMITED
    }
}
#[test]
fn dispatch_and_fallthrough_stop_before_fuel_admission() {
    for instructions in [
        vec![],
        vec![Instruction::LoadLiteral(Literal::Integer("1".into()))],
    ] {
        let polls = Cell::new(0);
        let mut budget = Budget::with_test_clock(
            RuntimePolicy {
                execution: ExecutionLimits {
                    max_instructions: Some(1),
                    max_output_bytes: Some(0),
                },
                materialization: MaterializationLimits {
                    max_value_nodes: Some(1),
                    max_text_bytes: Some(0),
                },
            },
            settings(),
            move || {
                let poll = polls.get();
                polls.set(poll + 1);
                Duration::from_secs(u64::from(poll > 0))
            },
        );
        let main = function(instructions);
        assert_eq!(
            execute_function(
                &main,
                &[],
                &IrProgram { functions: vec![] },
                &mut MemoryHost,
                &mut 0,
                &mut budget
            )
            .unwrap_err(),
            ELAPSED_ERROR
        );
        // Both dispatch/fallthrough refused before spending the only node/instruction.
        budget.instruction().unwrap();
        budget.materialize(1, 0).unwrap();
    }
}

#[test]
fn cancellation_after_frame_entry_precedes_dispatch_fuel() {
    let token = CancellationToken::new();
    let cancel = token.clone();
    let mut budget = Budget::with_test_clock(
        RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: Some(0),
                max_output_bytes: Some(0),
            },
            materialization: MaterializationLimits {
                max_value_nodes: Some(0),
                max_text_bytes: Some(0),
            },
        },
        ExecutionControl {
            cancellation: Some(token),
            max_elapsed: Some(Duration::MAX),
        },
        move || {
            cancel.cancel();
            Duration::ZERO
        },
    );
    let main = function(vec![Instruction::Jump { target: 0 }]);
    assert_eq!(
        execute_function(
            &main,
            &[],
            &IrProgram { functions: vec![] },
            &mut MemoryHost,
            &mut 0,
            &mut budget
        )
        .unwrap_err(),
        "execution cancelled"
    );
}
#[test]
fn successful_host_return_checks_elapsed_before_unknown_payload_admission() {
    struct ClockHost {
        clock: Rc<Cell<Duration>>,
        fail: bool,
        calls: usize,
    }
    impl RuntimeHost for ClockHost {
        fn arguments(&self) -> &[String] {
            &[]
        }
        fn write_line(&mut self, _: &str) -> Result<(), String> {
            panic!("unexpected output")
        }
        fn read_line(&mut self) -> Result<Option<String>, String> {
            self.calls += 1;
            self.clock.set(Duration::from_secs(1));
            if self.fail {
                Err("read failure".into())
            } else {
                Ok(Some("too large".into()))
            }
        }
    }
    for fail in [false, true] {
        let clock = Rc::new(Cell::new(Duration::ZERO));
        let copy = clock.clone();
        let mut budget = Budget::with_test_clock(
            RuntimePolicy {
                materialization: MaterializationLimits {
                    max_value_nodes: Some(3),
                    max_text_bytes: Some(21),
                },
                ..RuntimePolicy::UNLIMITED
            },
            settings(),
            move || copy.get(),
        );
        let mut host = ClockHost {
            clock,
            fail,
            calls: 0,
        };
        let main = function(vec![Instruction::Call {
            name: "std::read_line".into(),
            arguments: 0,
        }]);
        assert_eq!(
            execute_function(
                &main,
                &[],
                &IrProgram { functions: vec![] },
                &mut host,
                &mut 0,
                &mut budget
            )
            .unwrap_err(),
            if fail { "read failure" } else { ELAPSED_ERROR }
        );
        assert_eq!(host.calls, 1);
        budget.materialize(3, 21).unwrap(); // no partial or full ingress charge
    }
}
#[test]
fn cancellation_during_host_output_preserves_committed_fuel() {
    struct CancelHost(CancellationToken);
    impl RuntimeHost for CancelHost {
        fn arguments(&self) -> &[String] {
            &[]
        }
        fn read_line(&mut self) -> Result<Option<String>, String> {
            unreachable!()
        }
        fn write_line(&mut self, _: &str) -> Result<(), String> {
            self.0.cancel();
            Ok(())
        }
    }
    let token = CancellationToken::new();
    let mut budget = Budget::with_control(
        RuntimePolicy {
            execution: ExecutionLimits {
                max_instructions: None,
                max_output_bytes: Some(2),
            },
            materialization: MaterializationLimits {
                max_value_nodes: Some(2),
                max_text_bytes: Some(2),
            },
        },
        ExecutionControl {
            cancellation: Some(token.clone()),
            max_elapsed: None,
        },
    );
    let main = function(vec![
        Instruction::LoadLiteral(Literal::String("x".into())),
        Instruction::Call {
            name: "print".into(),
            arguments: 1,
        },
    ]);
    assert_eq!(
        execute_function(
            &main,
            &[],
            &IrProgram { functions: vec![] },
            &mut CancelHost(token),
            &mut 0,
            &mut budget
        )
        .unwrap_err(),
        "execution cancelled"
    );
    assert!(budget
        .materialize(1, 0)
        .unwrap_err()
        .contains("node allowance"));
    assert!(budget
        .materialize(0, 1)
        .unwrap_err()
        .contains("text materialization"));
    assert!(budget
        .output(0)
        .unwrap_err()
        .contains("output byte allowance"));
}
#[test]
fn partial_frame_child() {
    let Ok(kind) = std::env::var("SOVRA_CONTROL_FRAME_CHILD") else {
        return;
    };
    let mut deep = Value::Int(1);
    for index in 0..16000 {
        deep = if kind == "records" && index % 2 == 0 {
            Value::Struct(Box::new(StructValue {
                type_name: "Node".into(),
                fields: vec![("next".into(), deep)],
            }))
        } else {
            Value::Array(vec![deep])
        };
    }
    let arguments = OwnedValues(vec![deep, Value::Int(2)]);
    let polls = Cell::new(0);
    let mut budget = Budget::with_test_clock(RuntimePolicy::UNLIMITED, settings(), move || {
        let poll = polls.get();
        polls.set(poll + 1);
        Duration::from_secs(u64::from(poll >= 2))
    });
    let callee = IrFunction {
        name: "pair".into(),
        parameters: vec!["first".into(), "second".into()],
        instructions: vec![],
    };
    // First deep argument is already copied when the second snapshot is refused.
    assert_eq!(
        call_frame(&callee, &arguments, &mut budget).err().unwrap(),
        ELAPSED_ERROR
    );
    println!("partial-frame-control-success");
}
#[test]
fn partially_copied_deep_frames_cleanup_after_control_failure() {
    for kind in ["arrays", "records"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "compiler::interpreter::control_tests::partial_frame_child",
                "--nocapture",
            ])
            .env("SOVRA_CONTROL_FRAME_CHILD", kind)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("partial-frame-control-success"));
    }
}
