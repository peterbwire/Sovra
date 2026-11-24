//! Actual generated Node workers; supervisors stay responsive during busy source loops.
use sovra::compiler::{
    backend::{self, JavaScriptWorkerControl},
    execution::{ExecutionLimits, MaterializationLimits, RuntimePolicy},
    interpreter,
    ir::{Instruction, IrFunction, IrProgram, Literal},
};
use std::{
    io::Write,
    process::{Command, Stdio},
    time::Duration,
};
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
fn cancellation() -> JavaScriptWorkerControl {
    JavaScriptWorkerControl {
        enable_cancellation: true,
        max_elapsed: None,
    }
}
const FLAG: &str = "{sovraControl:{version:1,cancellation:new SharedArrayBuffer(4)}}";
const CANCELLED: &str = "{sovraControl:{version:1,cancellation:(()=>{const flag=new SharedArrayBuffer(4);Atomics.store(new Int32Array(flag),0,1);return flag;})()}}";
fn render(ir: &IrProgram, policy: RuntimePolicy, control: JavaScriptWorkerControl) -> String {
    backend::render_javascript_worker_with_policy_and_control(ir, policy, control)
}
fn worker(script: &str, data: &str, cancel_on_stdout: bool) -> std::process::Output {
    // fd 1 belongs to this subprocess, not worker.stdout. Observe writes solely
    // in the test supervisor to handshake once the source reaches its busy callee.
    let supervisor = format!(
        r#"
const fs=require('node:fs'), {{Worker}}=require('node:worker_threads');
const script=fs.readFileSync(0,'utf8');
const data={data};
let failure='', signalled=false;
const instance=new Worker(script,{{eval:true,workerData:data,resourceLimits:{{stackSizeMb:0.5}}}});
const deadline=setTimeout(()=>{{instance.terminate();console.error('HARNESS DEADLINE');process.exitCode=90;}},10000);
instance.on('error',error=>{{failure=error.message;}});
instance.on('exit',code=>{{clearTimeout(deadline);console.error('RESULT:'+code+':'+failure);}});
{cancel}
"#,
        cancel = if cancel_on_stdout {
            r#"
// A pipe owned by the containing Rust test cannot be read by this supervisor.
// The worker sends readiness through a test-only host write observer below.
instance.on('message', message=>{if(message==='source-ready'){signalled=true;Atomics.store(new Int32Array(data.sovraControl.cancellation),0,1);}});
"#
        } else {
            ""
        }
    );
    let mut child = Command::new("node")
        .args(["--eval", &supervisor])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    result
}
fn expect(script: &str, data: &str, output: &str, error: Option<&str>, code: u8) {
    let result = worker(script, data, false);
    assert_eq!(String::from_utf8(result.stdout).unwrap(), output);
    assert_eq!(
        String::from_utf8(result.stderr).unwrap().trim(),
        format!("RESULT:{code}:{}", error.unwrap_or(""))
    );
}
#[test]
fn transport_shapes_worker_only_mode_and_error_priority_are_explicit() {
    let ir = program(vec![]);
    let script = render(&ir, RuntimePolicy::UNLIMITED, cancellation());
    for data in [
        "undefined",
        "null",
        "{}",
        "{sovraControl:null}",
        "{sovraControl:[]}",
        "{sovraControl:{version:0,cancellation:new SharedArrayBuffer(4)}}",
        "{sovraControl:{version:1,cancellation:new ArrayBuffer(4)}}",
        "{sovraControl:{version:1,cancellation:new Int32Array(new SharedArrayBuffer(4))}}",
        "{sovraControl:{version:1,cancellation:new SharedArrayBuffer(8)}}",
        "{sovraControl:{version:1,cancellation:new SharedArrayBuffer(4),extra:1}}",
    ] {
        expect(
            &script,
            data,
            "",
            Some("invalid JavaScript worker control"),
            1,
        );
    }
    // Growable SAB is optional in older hosts; check where present rather than count a skip as qualification.
    let growable = Command::new("node")
        .args([
            "--eval",
            "process.exit(new SharedArrayBuffer(4,{maxByteLength:8}).growable===true?0:1)",
        ])
        .status()
        .unwrap();
    if growable.success() {
        expect(
            &script,
            "{sovraControl:{version:1,cancellation:new SharedArrayBuffer(4,{maxByteLength:8})}}",
            "",
            Some("invalid JavaScript worker control"),
            1,
        );
    }
    expect(&script, FLAG, "", None, 0);
    let ordinary = Command::new("node")
        .args(["--eval", &script])
        .output()
        .unwrap();
    assert!(!ordinary.status.success());
    assert!(String::from_utf8_lossy(&ordinary.stderr)
        .contains("JavaScript control requires a Node worker"));
    let disabled = render(
        &ir,
        RuntimePolicy::UNLIMITED,
        JavaScriptWorkerControl::UNLIMITED,
    );
    expect(&disabled, "undefined", "", None, 0);
    for data in [FLAG, "{sovraControl:undefined}", "{sovraControl:null}"] {
        expect(
            &disabled,
            data,
            "",
            Some("invalid JavaScript worker control"),
            1,
        );
    }
    let mut invalid = ir.clone();
    invalid.functions.push(invalid.functions[0].clone());
    let error = interpreter::run(&invalid).unwrap_err();
    expect(
        &render(&invalid, RuntimePolicy::UNLIMITED, cancellation()),
        "{}",
        "",
        Some(&error),
        1,
    );
    let missing = IrProgram { functions: vec![] };
    expect(
        &render(&missing, RuntimePolicy::UNLIMITED, cancellation()),
        CANCELLED,
        "",
        Some("entry function `main` was not found"),
        1,
    );
    let mut arity = ir;
    arity.functions[0].parameters.push("arg".into());
    expect(
        &render(&arity, RuntimePolicy::UNLIMITED, cancellation()),
        CANCELLED,
        "",
        Some("function `main` expects 1 argument(s), found 0"),
        1,
    );
}
#[test]
fn precancellation_zero_elapsed_and_disabled_controls_preserve_fuel_contract() {
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
    let ir = program(vec![]);
    let settings = JavaScriptWorkerControl {
        max_elapsed: Some(Duration::ZERO),
        ..cancellation()
    };
    expect(
        &render(&ir, zero, settings),
        CANCELLED,
        "",
        Some("execution cancelled"),
        1,
    );
    expect(
        &render(&ir, zero, settings),
        FLAG,
        "",
        Some("execution elapsed time allowance exceeded"),
        1,
    );
    let ir = source("fn main(){let original=[\"λ\",\"😀\"];let copy=original;print(copy);}");
    for (nodes, text, error, stdout) in [
        (11, 28, None, "[λ, 😀]\n"),
        (
            10,
            28,
            Some("execution value node allowance exceeded"),
            "[λ, 😀]\n",
        ),
        (
            11,
            27,
            Some("execution text materialization allowance exceeded"),
            "",
        ),
    ] {
        let policy = RuntimePolicy {
            materialization: MaterializationLimits {
                max_value_nodes: Some(nodes),
                max_text_bytes: Some(text),
            },
            ..RuntimePolicy::UNLIMITED
        };
        expect(
            &render(
                &ir,
                policy,
                JavaScriptWorkerControl {
                    enable_cancellation: false,
                    max_elapsed: Some(Duration::MAX),
                },
            ),
            "undefined",
            stdout,
            error,
            if error.is_some() { 1 } else { 0 },
        );
    }
    assert!(!backend::render_javascript(&ir).contains("svrCheckpoint"));
    let exit = source("fn main(){std::set_exit_code(7);}");
    expect(
        &render(
            &exit,
            RuntimePolicy::UNLIMITED,
            JavaScriptWorkerControl::UNLIMITED,
        ),
        "undefined",
        "",
        None,
        7,
    );
}
#[test]
fn actual_generated_callee_loops_observe_supervisor_cancellation_and_elapsed() {
    let ir = source("fn busy(){print(\"ready\");while true {}} fn main(){busy();}");
    // Test-only observer signals readiness after the actual source write returns.
    // It does not implement cancellation; generated dispatch polls the shared flag.
    let script=render(&ir,RuntimePolicy::UNLIMITED,cancellation()).replace("svrFunctions.main();",r#"
const write=svrFs.writeSync;
svrFs.writeSync=(...args)=>{const written=write(...args);require('node:worker_threads').parentPort.postMessage('source-ready');return written;};
svrFunctions.main();
"#);
    let result = worker(&script, FLAG, true);
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "ready\n");
    assert_eq!(
        String::from_utf8(result.stderr).unwrap().trim(),
        "RESULT:1:execution cancelled"
    );
    let ir = source("fn busy(){while true {}} fn main(){busy();}");
    expect(
        &render(
            &ir,
            RuntimePolicy::UNLIMITED,
            JavaScriptWorkerControl {
                enable_cancellation: false,
                max_elapsed: Some(Duration::from_millis(10)),
            },
        ),
        "undefined",
        "",
        Some("execution elapsed time allowance exceeded"),
        1,
    );
}
#[test]
fn post_host_checks_preserve_effects_errors_and_precede_unknown_payload_ingress() {
    for (instructions, hook, result, nodes, bytes, expected) in [
        (
            vec![
                Instruction::LoadLiteral(Literal::String("p".into())),
                Instruction::LoadLiteral(Literal::Boolean(false)),
                Instruction::LoadLiteral(Literal::String("".into())),
                Instruction::LoadLiteral(Literal::String("x".into())),
                Instruction::Call {
                    name: "std::write_text_if".into(),
                    arguments: 4,
                },
            ],
            "svrWriteTextIf",
            "svrResult('std::TextWrite',{ok:false,error:'oversized'})",
            7,
            23,
            "execution cancelled",
        ),
        (
            vec![Instruction::Call {
                name: "std::read_line".into(),
                arguments: 0,
            }],
            "svrReadLine",
            "svrResult('std::InputLine',{eof:false,text:'oversized'})",
            3,
            21,
            "execution cancelled",
        ),
        (
            vec![
                Instruction::LoadLiteral(Literal::String("p".into())),
                Instruction::Call {
                    name: "std::read_text".into(),
                    arguments: 1,
                },
            ],
            "svrReadText",
            "svrResult('std::TextRead',{ok:true,text:'oversized',error:''})",
            5,
            25,
            "execution cancelled",
        ),
        (
            vec![
                Instruction::LoadLiteral(Literal::String("p".into())),
                Instruction::LoadLiteral(Literal::String("x".into())),
                Instruction::Call {
                    name: "std::write_text".into(),
                    arguments: 2,
                },
            ],
            "svrWriteText",
            "svrResult('std::TextWrite',{ok:false,error:'oversized'})",
            5,
            23,
            "execution cancelled",
        ),
        (
            vec![Instruction::Call {
                name: "std::read_line".into(),
                arguments: 0,
            }],
            "svrReadLine",
            "FAIL",
            3,
            21,
            "host failure",
        ),
    ] {
        let policy = RuntimePolicy {
            materialization: MaterializationLimits {
                max_value_nodes: Some(nodes),
                max_text_bytes: Some(bytes),
            },
            ..RuntimePolicy::UNLIMITED
        };
        let effect = if result == "FAIL" {
            "throw new Error('host failure');".into()
        } else {
            format!("return {result};")
        };
        let replacement=format!("{hook}=()=>{{require('node:fs').writeSync(1,'effect'+String.fromCharCode(10));Atomics.store(svrCancellation,0,1);{effect}}};svrFunctions.main();");
        let script = render(&program(instructions), policy, cancellation())
            .replace("svrFunctions.main();", &replacement);
        expect(&script, FLAG, "effect\n", Some(expected), 1);
        // Tighten just the known minimum: host code must never run at all.
        let refused = script.replacen(
            &format!("let svrTextRemaining = {bytes}n;"),
            &format!("let svrTextRemaining = {}n;", bytes - 1),
            1,
        );
        expect(
            &refused,
            FLAG,
            "",
            Some("execution text materialization allowance exceeded"),
            1,
        );
    }
}

#[test]
fn private_clock_checks_exact_boundaries_priority_and_huge_durations() {
    for duration in [Duration::from_nanos(10), Duration::MAX] {
        let control = JavaScriptWorkerControl {
            enable_cancellation: true,
            max_elapsed: Some(duration),
        };
        let script = render(&program(vec![]), RuntimePolicy::UNLIMITED, control).replace(
            "svrFunctions.main();",
            r#"
let tick=svrControlOrigin;
process.hrtime.bigint=()=>tick;
svrCheckpoint();
tick=svrControlOrigin+svrElapsedAllowance-1n;
svrCheckpoint();
tick++;
let failure;
try {svrCheckpoint();} catch(error) {failure=error.message;}
if(failure!=='execution elapsed time allowance exceeded') throw new Error('exact boundary');
Atomics.store(svrCancellation,0,1);
try {svrCheckpoint();} catch(error) {failure=error.message;}
if(failure!=='execution cancelled') throw new Error('cancellation priority');
require('node:fs').writeSync(1,'clock-success'+String.fromCharCode(10));
// Restore successful final-check state by ending this test worker, not resetting the flag.
process.exit(0);
"#,
        );
        expect(&script, FLAG, "clock-success\n", None, 0);
    }
}

#[test]
fn deep_parameter_copies_and_partial_control_refusal_are_stack_safe() {
    let mut ir = program(vec![]);
    ir.functions.push(IrFunction {
        name: "pair".into(),
        parameters: vec!["tree".into(), "second".into()],
        instructions: vec![],
    });
    for records in [false, true] {
        let script = render(
            &ir,
            RuntimePolicy::UNLIMITED,
            JavaScriptWorkerControl {
                enable_cancellation: false,
                max_elapsed: Some(Duration::from_secs(1)),
            },
        );
        let body = format!(
            r#"
let value='λ';
for(let index=0;index<16000;index++) value={records} && index%2===0 ? svrResult('Node',{{next:value}}) : [value];
const size=svrFootprint(value);
if(size.nodes!==16001n || size.text!==({records}?64002n:2n)) throw new Error('deep shape');
let polls=0;
process.hrtime.bigint=()=>svrControlOrigin+(++polls>=3?svrElapsedAllowance:0n);
svrNodeRemaining=size.nodes+1n;
svrTextRemaining=size.text;
let error;
try {{svrFunctions.pair(value,0n);}} catch(failure) {{error=failure.message;}}
if(error!=='execution elapsed time allowance exceeded' || svrNodeRemaining!==1n || svrTextRemaining!==0n) throw new Error('partial parameter admission');
// A new test invocation of the helper with a fresh test clock/fuel is independent.
process.hrtime.bigint=()=>svrControlOrigin;
svrNodeRemaining=size.nodes+2n;
svrTextRemaining=size.text;
svrFunctions.pair(value,0n);
if(svrNodeRemaining!==0n || svrTextRemaining!==0n) throw new Error('deep exact admission');
require('node:fs').writeSync(1,'deep-parameter-success'+String.fromCharCode(10));
"#
        );
        expect(
            &script.replace("svrFunctions.main();", &body),
            "undefined",
            "deep-parameter-success\n",
            None,
            0,
        );
    }
}

#[test]
fn print_checks_surround_the_line_without_polling_between_partial_writes() {
    for fail in [false, true] {
        let ir = source("fn main(){print(\"λ😀\");}");
        let hook = format!(
            r#"
const original=svrFs.writeSync;
let writes=0;
svrFs.writeSync=(fd,buffer,offset,length)=>{{
  writes++;
  Atomics.store(svrCancellation,0,1);
  {failure}
  // Deliberately split the line: checkpoint must wait for its full publication.
  return original(fd,buffer,offset,Math.min(length,1));
}};
svrFunctions.main();
"#,
            failure = if fail {
                "throw new Error('deliberate output failure');"
            } else {
                ""
            }
        );
        let script = render(
            &ir,
            RuntimePolicy {
                materialization: MaterializationLimits {
                    max_value_nodes: Some(2),
                    max_text_bytes: Some(12),
                },
                ..RuntimePolicy::UNLIMITED
            },
            cancellation(),
        )
        .replace("svrFunctions.main();", &hook);
        expect(
            &script,
            FLAG,
            if fail { "" } else { "λ😀\n" },
            Some(if fail {
                "standard output write failed: deliberate output failure"
            } else {
                "execution cancelled"
            }),
            1,
        );
    }
}

#[test]
fn registered_package_runs_with_worker_limits_and_has_no_unlimited_fallback() {
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
    ))]
    {
        let package = sovra::compiler::project::packages::compile_registered_records(
            "examples/typed-array-packages/app",
        )
        .unwrap();
        expect(
            &render(
                package.ir(),
                RuntimePolicy {
                    materialization: MaterializationLimits {
                        max_value_nodes: Some(1000),
                        max_text_bytes: Some(10000),
                    },
                    ..RuntimePolicy::UNLIMITED
                },
                JavaScriptWorkerControl::UNLIMITED,
            ),
            "undefined",
            "9\n1.5\n0\n1\n4.5\n",
            None,
            0,
        );
        expect(
            &render(
                package.ir(),
                RuntimePolicy {
                    materialization: MaterializationLimits {
                        max_value_nodes: Some(0),
                        max_text_bytes: Some(0),
                    },
                    ..RuntimePolicy::UNLIMITED
                },
                JavaScriptWorkerControl::UNLIMITED,
            ),
            "undefined",
            "",
            Some("execution value node allowance exceeded"),
            1,
        );
    }
}
