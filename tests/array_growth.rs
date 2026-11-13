//! G13 snapshot array growth across checked/legacy lowering and real Node engines.
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

#[test]
fn growth_preserves_snapshots_and_first_directed_numeric_coercions() {
    execute("type Fs=[Float];struct Point{x:Int}fn main(){let original=[1];let mut grown=std::array_append(original,2);grown[0]=9;print(original);print(grown);let joined=std::array_concat(original,[3]);print(joined);let floats:Fs=[1];let a=std::array_append(floats,3);print(a[1]/2);let b=std::array_concat(floats,[3]);print(b[1]/2);let c=std::array_append([[1.0]],[3]);print(c[1][0]/2);let d=std::array_concat([[1.0]],[[3]]);print(d[1][0]/2);let empty:[Float]=[];print(std::array_append(empty,3)[0]/2);print(std::array_concat([1],[]));print(std::array_append([[1]],[]));let points=[Point{x:7}];print(std::array_append(points,Point{x:9})[1].x);let units:[Unit]=[];let result=std::array_append(units,print(\"unit\"));print(std::array_len(result));}","[1]\n[9, 2]\n[1, 3]\n1.5\n1.5\n1.5\n1.5\n1.5\n[1]\n[[1], []]\n9\nunit\n1\n");
}
#[test]
fn growth_rejects_unknown_first_types_backward_widening_and_nominal_mismatches() {
    for text in [
        "fn main(){std::array_append([],1);}",
        "fn main(){let x:[Int]=std::array_append([],1);}",
        "fn main(){std::array_concat([],[1]);}",
        "fn main(){std::array_append([[]],[1]);}",
        "fn main(){std::array_append([1],2.5);}",
        "fn main(){std::array_concat([1],[2.5]);}",
        "fn main(){std::array_append([1],\"x\");}",
        "fn main(){std::array_append(1,2);}",
        "fn main(){std::array_concat([1],2);}",
        "fn main(){std::array_append([1]);}",
        "fn main(){std::array_concat([1],[2],[3]);}",
        "struct A{x:Int}struct B{x:Int}fn main(){std::array_append([A{x:1}],B{x:2});}",
        "struct A{x:Int}struct B{x:Int}fn main(){std::array_concat([A{x:1}],[B{x:2}]);}",
    ] {
        let ast = Parser::new().parse_source(text).unwrap();
        assert!(sovra::compiler::compile_program(&ast).is_err(), "{text}");
    }
    execute("fn main(){let empty:[Int]=[];print(std::array_concat(empty,empty));print(std::array_append(empty,2));let target:[[Int]]=[];print(std::array_append(target,[]));}","[]\n[2]\n[[]]\n");
}
#[test]
fn growth_exact_forecasts_and_empty_results_obey_atomic_fuel() {
    for (expression, nodes) in [
        ("std::array_append([1],2)", 11),
        ("std::array_concat([1],[2])", 12),
    ] {
        let ir = compiled(&format!(
            "fn main(){{let result={expression};print(result);}}"
        ));
        compare(&ir, policy(nodes, 6), "[1, 2]\n", None);
        compare(
            &ir,
            policy(nodes - 1, 6),
            "[1, 2]\n",
            Some("execution value node allowance exceeded"),
        );
        compare(
            &ir,
            policy(nodes, 5),
            "",
            Some("execution text materialization allowance exceeded"),
        );
        let mut limited = policy(nodes, 6);
        limited.execution = ExecutionLimits {
            max_instructions: None,
            max_output_bytes: Some(6),
        };
        compare(
            &ir,
            limited,
            "",
            Some("execution output byte allowance exceeded"),
        );
    }
    // Empty input containers are paid separately from the one new result container.
    compare(
        &program(vec![
            Instruction::MakeArray { length: 0 },
            Instruction::MakeArray { length: 0 },
            Instruction::Call {
                name: "std::array_concat".into(),
                arguments: 2,
            },
            Instruction::Return,
        ]),
        policy(3, 0),
        "",
        None,
    );
}
#[test]
fn growth_runtime_validation_and_unicode_labels_precede_result_admission() {
    for (ir, nodes, error) in [
        (
            program(vec![
                integer("1"),
                integer("2"),
                Instruction::Call {
                    name: "std::array_append".into(),
                    arguments: 2,
                },
            ]),
            2,
            "std::array_append expects an array argument",
        ),
        (
            program(vec![
                Instruction::MakeArray { length: 0 },
                integer("2"),
                Instruction::Call {
                    name: "std::array_concat".into(),
                    arguments: 2,
                },
            ]),
            2,
            "std::array_concat expects array arguments",
        ),
        (
            program(vec![
                integer("1"),
                Instruction::MakeArray { length: 1 },
                integer("2"),
                Instruction::Call {
                    name: "std::array_append".into(),
                    arguments: 2,
                },
            ]),
            3,
            "execution value node allowance exceeded",
        ),
    ] {
        compare(&ir, policy(nodes, 0), "", Some(error));
    }
    for name in ["std::array_append", "std::array_concat"] {
        let mut instructions = vec![
            Instruction::LoadLiteral(Literal::String("λ".into())),
            Instruction::MakeStruct {
                type_name: "Box".into(),
                fields: vec!["text".into()],
            },
            Instruction::MakeArray { length: 1 },
            Instruction::LoadLiteral(Literal::String("😀".into())),
            Instruction::MakeStruct {
                type_name: "Box".into(),
                fields: vec!["text".into()],
            },
        ];
        if name == "std::array_concat" {
            instructions.push(Instruction::MakeArray { length: 1 });
        }
        instructions.extend([
            Instruction::Call {
                name: name.into(),
                arguments: 2,
            },
            Instruction::Return,
        ]);
        let ir = program(instructions);
        let nodes = if name == "std::array_concat" { 11 } else { 10 };
        compare(&ir, policy(nodes, 40), "", None);
        compare(
            &ir,
            policy(nodes, 39),
            "",
            Some("execution text materialization allowance exceeded"),
        );
    }
}
#[test]
fn node_length_guard_checks_boundaries_without_allocating_the_result() {
    let script = backend::render_javascript(&program(vec![]))
        + r#"
if(svrGrowthLength(4294967294n,1n)!==4294967295)throw new Error('exact limit');
for(const pair of [[4294967295n,1n],[18446744073709551615n,1n],[0n,-1n]]){
 let error;try{svrGrowthLength(...pair);}catch(failure){error=failure.message;}
 if(error!=='array result length exceeds limit')throw new Error('length guard');
}
"#;
    let result = Command::new("node")
        .args(["--eval", &script])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
fn independent_growth_consumer_preserves_package_coercions_and_nominal_records() {
    let package = sovra::compiler::project::packages::compile_registered_records(
        "examples/array-slice-packages/growth-app",
    )
    .unwrap();
    compare(
        package.ir(),
        policy(10000, 100000),
        "[-2, 0, 3, 4]\n[9, 4]\n1.5\n7\n",
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
fn deep_worker_growth_copies_descendants_once_and_refuses_atomically() {
    let mut ir = program(vec![]);
    for (name, builtin) in [
        ("append", "std::array_append"),
        ("concat", "std::array_concat"),
    ] {
        let mut instructions = vec![Instruction::LoadName("values".into()), integer("0")];
        if name == "concat" {
            instructions.push(Instruction::MakeArray { length: 1 });
        }
        instructions.extend([
            Instruction::Call {
                name: builtin.into(),
                arguments: 2,
            },
            Instruction::Return,
        ]);
        ir.functions.push(IrFunction {
            name: name.into(),
            parameters: vec!["values".into()],
            instructions,
        });
    }
    for records in [false, true] {
        let script = backend::render_javascript_worker_with_policy_and_control(
            &ir,
            RuntimePolicy::UNLIMITED,
            backend::JavaScriptWorkerControl::UNLIMITED,
        );
        let body = format!(
            r#"
let value='λ';for(let i=0;i<16000;i++)value={records}&&i%2===0?svrResult('Node',{{next:value}}):[value];
const input=[value], size=svrFootprint(input);
for(const name of ['append','concat']){{
 const extra=name==='append'?0n:1n;
 svrNodeRemaining=3n*size.nodes+2n+extra;svrTextRemaining=3n*size.text;
 const result=svrFunctions[name](input);
 if(result.length!==2||result[0]===input[0]||svrDisplay(result[0])!==svrDisplay(input[0])||svrNodeRemaining!==0n||svrTextRemaining!==0n)throw new Error('deep growth copy/cost');
 svrNodeRemaining=3n*size.nodes+1n+extra;svrTextRemaining=3n*size.text;
 let failure;try{{svrFunctions[name](input);}}catch(error){{failure=error.message;}}
 if(failure!=='execution value node allowance exceeded'||svrNodeRemaining!==size.nodes||svrTextRemaining!==size.text)throw new Error('deep growth admission');
}}
"#
        );
        let script = script.replace("svrFunctions.main();", &body);
        let supervisor = r#"const fs=require('node:fs'),{Worker}=require('node:worker_threads');const worker=new Worker(fs.readFileSync(0,'utf8'),{eval:true,resourceLimits:{stackSizeMb:0.5}});const timer=setTimeout(()=>{worker.terminate();process.exitCode=90;},10000);worker.on('error',error=>console.error(error));worker.on('exit',code=>{clearTimeout(timer);if(code!==0)process.exitCode=code;});"#;
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

#[test]
fn wide_append_copies_all_elements_with_exact_logical_fuel() {
    let mut instructions = vec![integer("1"); 50000];
    instructions.push(Instruction::MakeArray { length: 50000 });
    instructions.push(integer("2"));
    instructions.extend([
        Instruction::Call {
            name: "std::array_append".into(),
            arguments: 2,
        },
        Instruction::Return,
    ]);
    let ir = program(instructions);
    assert!(interpreter::run_with_policy(&ir, policy(100004, 0)).is_ok());
    assert_eq!(
        interpreter::run_with_policy(&ir, policy(100003, 0)).unwrap_err(),
        "execution value node allowance exceeded"
    );
    // Keep JavaScript compilation small: host construction is outside VM fuel,
    // while actual generated parameter/load/result copies remain fully charged.
    let mut ir = program(vec![]);
    ir.functions.push(IrFunction {
        name: "grow".into(),
        parameters: vec!["values".into()],
        instructions: vec![
            Instruction::LoadName("values".into()),
            integer("2"),
            Instruction::Call {
                name: "std::array_append".into(),
                arguments: 2,
            },
            Instruction::Return,
        ],
    });
    let script = backend::render_javascript_worker_with_policy_and_control(&ir, RuntimePolicy::UNLIMITED, backend::JavaScriptWorkerControl::UNLIMITED).replace("svrFunctions.main();", r#"
const input=new Array(50000).fill(1n), size=svrFootprint(input);
svrNodeRemaining=3n*size.nodes+2n;svrTextRemaining=0n;
const result=svrFunctions.grow(input);
if(result.length!==50001||result[0]!==1n||result[50000]!==2n||svrNodeRemaining!==0n)throw new Error('wide copy/cost');
svrNodeRemaining=3n*size.nodes+1n;
let failure;try{svrFunctions.grow(input);}catch(error){failure=error.message;}
if(failure!=='execution value node allowance exceeded'||svrNodeRemaining!==size.nodes)throw new Error('wide refusal');
"#);
    let supervisor = r#"const fs=require('node:fs'),{Worker}=require('node:worker_threads');const worker=new Worker(fs.readFileSync(0,'utf8'),{eval:true});const timer=setTimeout(()=>{worker.terminate();process.exitCode=90;},10000);worker.on('error',error=>console.error(error));worker.on('exit',code=>{clearTimeout(timer);if(code!==0)process.exitCode=code;});"#;
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
