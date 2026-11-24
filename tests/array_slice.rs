//! G13 snapshot slicing across checked/legacy lowering and real Node engines.
use sovra::compiler::{
    backend,
    execution::{ExecutionLimits, MaterializationLimits, RuntimePolicy},
    interpreter::{self, RuntimeHost},
    ir::{self, Instruction, IrFunction, IrProgram, Literal},
    parser::Parser,
    semantic::SemanticAnalyzer,
};
use std::{
    io::Write,
    process::{Command, Stdio},
};
fn compiled(text: &str) -> IrProgram {
    let ast = Parser::new().parse_source(text).unwrap();
    sovra::compiler::compile_program(&ast).unwrap().0
}
#[derive(Default)]
struct Host(Vec<String>);
impl RuntimeHost for Host {
    fn arguments(&self) -> &[String] {
        &[]
    }
    fn read_line(&mut self) -> Result<Option<String>, String> {
        Err("unexpected input".into())
    }
    fn write_line(&mut self, text: &str) -> Result<(), String> {
        self.0.push(text.into());
        Ok(())
    }
}
fn compare(ir: &IrProgram, policy: RuntimePolicy, output: &str, error: Option<&str>) {
    let mut host = Host::default();
    let result = interpreter::run_with_host_status_and_policy(ir, &mut host, policy);
    assert_eq!(
        host.0
            .iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>(),
        output
    );
    assert_eq!(result.is_ok(), error.is_none(), "{result:?}");
    if let Some(error) = error {
        assert_eq!(result.unwrap_err(), error);
    }
    for worker in [false, true] {
        let script = if worker {
            backend::render_javascript_worker_with_policy_and_control(
                ir,
                policy,
                backend::JavaScriptWorkerControl::UNLIMITED,
            )
        } else {
            backend::render_javascript_with_policy(ir, policy)
        };
        let supervisor = if worker {
            r#"const fs=require('node:fs'),{Worker}=require('node:worker_threads');const worker=new Worker(fs.readFileSync(0,'utf8'),{eval:true});const timer=setTimeout(()=>{worker.terminate();process.exitCode=90;},10000);worker.on('error',error=>console.error('FAIL:'+error.message));worker.on('exit',code=>{clearTimeout(timer);process.exitCode=code;});"#
        } else {
            r#"const fs=require('node:fs');try{eval(fs.readFileSync(0,'utf8'));}catch(error){console.error('FAIL:'+error.message);process.exitCode=1;}"#
        };
        let mut child = Command::new("node")
            .args(["--eval", supervisor])
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
        assert_eq!(String::from_utf8(result.stdout).unwrap(), output);
        assert_eq!(
            result.status.success(),
            error.is_none(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            String::from_utf8(result.stderr).unwrap().trim(),
            error.map_or(String::new(), |error| format!("FAIL:{error}"))
        );
    }
}
fn execute(text: &str, output: &str) {
    let ast = Parser::new().parse_source(text).unwrap();
    let checked = sovra::compiler::compile_program(&ast).unwrap().0;
    let typed = SemanticAnalyzer::new().analyze(&ast).unwrap();
    let legacy = ir::lower(&typed);
    assert_eq!(interpreter::run(&legacy).unwrap().join("\n") + "\n", output);
    compare(&checked, RuntimePolicy::UNLIMITED, output, None);
}
fn policy(nodes: u64, text: u64) -> RuntimePolicy {
    RuntimePolicy {
        materialization: MaterializationLimits {
            max_value_nodes: Some(nodes),
            max_text_bytes: Some(text),
        },
        ..RuntimePolicy::UNLIMITED
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
fn integer(text: &str) -> Instruction {
    Instruction::LoadLiteral(Literal::Integer(text.into()))
}
fn call() -> Instruction {
    Instruction::Call {
        name: "std::array_slice".into(),
        arguments: 3,
    }
}
#[test]
fn types_aliases_records_empty_and_snapshot_values_are_preserved() {
    execute("type Values=[Float];struct Point{x:Int}fn sliced(values:[Int])->Values{return std::array_slice(values,0,1);}fn main(){let original=[1,2,3];let mut selected=std::array_slice(original,1,3);selected[0]=9;print(original);print(selected);let empty:[Int]=[];print(std::array_slice(empty,0,0));print(std::array_slice(original,3,3));print(std::array_slice(original,0,3));let points=[Point{x:7}];print(std::array_slice(points,0,1)[0].x);print(sliced([3])[0]/2);let nested=[[1],[2]];let mut rows=std::array_slice(nested,0,1);rows[0]=[9];print(nested);print(rows);}","[1, 2, 3]\n[9, 3]\n[]\n[]\n[1, 2, 3]\n7\n1.5\n[[1], [2]]\n[[9]]\n");
    execute("fn main(){let original=[3.0];let selected=std::array_slice(original,0,1);print(selected[0]/2);let rows=[[3.0]];let selected_rows=std::array_slice(rows,0,1);print(selected_rows[0][0]/2);let widened:[Float]=std::array_slice([3],0,1);print(widened[0]/2);}","1.5\n1.5\n1.5\n");
}
#[test]
fn invalid_source_arguments_and_dependent_results_are_rejected() {
    for text in [
        "fn main(){std::array_slice(1,0,1);}",
        "fn main(){std::array_slice([1],0.0,1);}",
        "fn main(){std::array_slice([1],0,true);}",
        "fn main(){std::array_slice([1],0);}",
        "fn main(){std::array_slice([1],0,1,2);}",
        "fn main(){let wrong:[String]=std::array_slice([1],0,1);}",
        "fn main()->Int{return std::array_slice([1],0,1);}",
        "struct A{x:Int}struct B{x:Int}fn main(){let wrong:[B]=std::array_slice([A{x:1}],0,1);}",
    ] {
        let ast = Parser::new().parse_source(text).unwrap();
        assert!(sovra::compiler::compile_program(&ast).is_err(), "{text}");
    }
}
#[test]
fn checked_bounds_are_identical_in_three_engines() {
    for (start, end) in [
        ("-1", "0"),
        ("0", "-1"),
        ("2", "1"),
        ("0", "4"),
        ("4", "4"),
        ("9223372036854775807", "9223372036854775807"),
        ("-9223372036854775808", "0"),
    ] {
        let bound = |value: &str| {
            if value == "-9223372036854775808" {
                "(0-9223372036854775807-1)".to_owned()
            } else if let Some(value) = value.strip_prefix('-') {
                format!("(0-{value})")
            } else {
                value.to_owned()
            }
        };
        let source = format!(
            "fn main(){{std::array_slice([1,2,3],{},{});}}",
            bound(start),
            bound(end)
        );
        compare(
            &compiled(&source),
            RuntimePolicy::UNLIMITED,
            "",
            Some("array slice bounds out of range"),
        );
    }
    compare(
        &compiled("fn main(){let empty:[Int]=[];std::array_slice(empty,0,1);}"),
        RuntimePolicy::UNLIMITED,
        "",
        Some("array slice bounds out of range"),
    );
}
#[test]
fn exact_model_costs_and_validation_before_result_admission() {
    let ir=compiled("fn main(){let original=[1,2,3];let selected=std::array_slice(original,1,3);print(selected);}");
    compare(&ir, policy(18, 6), "[2, 3]\n", None);
    compare(
        &ir,
        policy(17, 6),
        "[2, 3]\n",
        Some("execution value node allowance exceeded"),
    );
    compare(
        &ir,
        policy(18, 5),
        "",
        Some("execution text materialization allowance exceeded"),
    );
    for (start, end, expected) in [
        (0, 1, "execution value node allowance exceeded"),
        (1, 0, "array slice bounds out of range"),
    ] {
        compare(
            &program(vec![
                integer("1"),
                Instruction::MakeArray { length: 1 },
                integer(&start.to_string()),
                integer(&end.to_string()),
                call(),
            ]),
            policy(4, 0),
            "",
            Some(expected),
        );
    }
    compare(
        &program(vec![integer("1"), integer("0"), integer("1"), call()]),
        policy(3, 0),
        "",
        Some("std::array_slice expects Array, Int, Int arguments"),
    );
    // A selected Unicode String is copied once; unselected payload is not charged again.
    let ir = program(vec![
        Instruction::LoadLiteral(Literal::String("λ".into())),
        Instruction::LoadLiteral(Literal::String("😀".into())),
        Instruction::MakeArray { length: 2 },
        integer("0"),
        integer("1"),
        call(),
        Instruction::Return,
    ]);
    compare(&ir, policy(7, 8), "", None);
    compare(
        &ir,
        policy(7, 7),
        "",
        Some("execution text materialization allowance exceeded"),
    );
    let mut output_policy = policy(18, 6);
    output_policy.execution = ExecutionLimits {
        max_instructions: None,
        max_output_bytes: Some(6),
    };
    compare(&compiled("fn main(){let original=[1,2,3];let selected=std::array_slice(original,1,3);print(selected);}"),output_policy,"",Some("execution output byte allowance exceeded"));
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn independent_registered_package_preserves_aliases_nominal_records_and_widening() {
    let package = sovra::compiler::project::packages::compile_registered_records(
        "examples/array-slice-packages/app",
    )
    .unwrap();
    compare(
        package.ir(),
        policy(1000, 10000),
        "[2, 3, 4]\n[9, 4]\n1.5\n[]\n9\n",
        None,
    );
    compare(
        package.ir(),
        policy(0, 0),
        "",
        Some("execution value node allowance exceeded"),
    );
}

#[test]
fn generated_deep_slice_copies_only_selected_snapshots_without_recursion() {
    let mut ir = program(vec![]);
    ir.functions.push(IrFunction {
        name: "slice".into(),
        parameters: vec!["values".into()],
        instructions: vec![
            Instruction::LoadName("values".into()),
            integer("0"),
            integer("1"),
            call(),
            Instruction::Return,
        ],
    });
    for records in [false, true] {
        let script = backend::render_javascript_worker_with_policy_and_control(
            &ir,
            RuntimePolicy::UNLIMITED,
            backend::JavaScriptWorkerControl::UNLIMITED,
        );
        let body = format!(
            r#"
let value='λ';for(let i=0;i<16000;i++)value={records} && i%2===0?svrResult('Node',{{next:value}}):[value];
const input=[value], size=svrFootprint(input);
svrNodeRemaining=3n*size.nodes+2n;svrTextRemaining=3n*size.text;
const copied=svrFunctions.slice(input);
if(svrDisplay(copied)!==svrDisplay(input)||copied===input||copied[0]===input[0]||svrNodeRemaining!==0n||svrTextRemaining!==0n)throw new Error('deep slice cost/copy');
svrNodeRemaining=3n*size.nodes+1n;svrTextRemaining=3n*size.text;
let failure;try{{svrFunctions.slice(input);}}catch(error){{failure=error.message;}}
if(failure!=='execution value node allowance exceeded'||svrNodeRemaining!==size.nodes-1n||svrTextRemaining!==size.text)throw new Error('deep refusal');
"#
        );
        let script = script.replace("svrFunctions.main();", &body);
        let supervisor = r#"const fs=require('node:fs'),{Worker}=require('node:worker_threads');const worker=new Worker(fs.readFileSync(0,'utf8'),{eval:true,resourceLimits:{stackSizeMb:0.5}});const timer=setTimeout(()=>{worker.terminate();process.exitCode=90;},10000);worker.on('error',error=>{console.error(error);process.exitCode=1;});worker.on('exit',code=>{clearTimeout(timer);if(code!==0)process.exitCode=code;});"#;
        let mut child = Command::new("node")
            .args(["--eval", supervisor])
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
    }
}
