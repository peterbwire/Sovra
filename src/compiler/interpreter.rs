//! Minimal stack-based interpreter for the current IR.

use std::collections::HashMap;

use crate::compiler::ir::{Instruction, IrFunction, IrProgram, Literal, MAX_CALL_DEPTH};
use crate::compiler::stdlib;

/// Runtime values supported by the interpreter.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// An integer.
    Int(i64),
    /// A floating-point number.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A string.
    String(String),
    /// A homogeneous collection of values.
    Array(Vec<Value>),
    /// A user-defined record with fields in source construction order.
    Struct(Box<StructValue>),
    /// No value.
    Unit,
}

/// Runtime representation of a user-defined record.
#[derive(Debug, Clone, PartialEq)]
pub struct StructValue {
    /// Declared record type name.
    pub type_name: String,
    /// Record fields in source construction order.
    pub fields: Vec<(String, Value)>,
}

impl Value {
    fn display(&self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::Bool(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::Array(values) => {
                let parts: Vec<_> = values.iter().map(Self::display).collect();
                format!("[{}]", parts.join(", "))
            }
            Self::Struct(record) => {
                let parts: Vec<_> = record
                    .fields
                    .iter()
                    .map(|(name, value)| format!("{name}: {}", value.display()))
                    .collect();
                format!("{} {{ {} }}", record.type_name, parts.join(", "))
            }
            Self::Unit => String::new(),
        }
    }
}

/// Execute the `main` function and return captured `print` output.
pub fn run(program: &IrProgram) -> Result<Vec<String>, String> {
    crate::compiler::ir::validate_declarations(program)?;
    let function = program
        .functions
        .iter()
        .find(|function| function.name == "main")
        .ok_or_else(|| "entry function `main` was not found".to_owned())?;
    let (output, _) = execute_function(function, &[], program)?;
    Ok(output)
}

struct CallFrame<'a> {
    function: &'a IrFunction,
    stack: Vec<Value>,
    names: HashMap<String, Value>,
    pc: usize,
}

fn call_frame<'a>(function: &'a IrFunction, arguments: &[Value]) -> Result<CallFrame<'a>, String> {
    if arguments.len() != function.parameters.len() {
        return Err(format!(
            "function `{}` expects {} argument(s), found {}",
            function.name,
            function.parameters.len(),
            arguments.len()
        ));
    }
    let names = function
        .parameters
        .iter()
        .cloned()
        .zip(arguments.iter().cloned())
        .collect();
    Ok(CallFrame {
        function,
        stack: Vec::new(),
        names,
        pc: 0,
    })
}

fn execute_function(
    function: &IrFunction,
    arguments: &[Value],
    program: &IrProgram,
) -> Result<(Vec<String>, Value), String> {
    let mut frames = vec![call_frame(function, arguments)?];
    let mut output = Vec::new();
    loop {
        let active_frames = frames.len();
        let frame = frames.last_mut().expect("entry call frame exists");
        if frame.pc >= frame.function.instructions.len() {
            let return_value = frame.stack.pop().unwrap_or(Value::Unit);
            frames.pop();
            if let Some(caller) = frames.last_mut() {
                caller.stack.push(return_value);
            } else {
                return Ok((output, return_value));
            }
            continue;
        }

        let mut pending_frame = None;
        let mut return_value = None;
        {
            let frame = frames.last_mut().expect("active call frame exists");
            let instruction: &Instruction = &frame.function.instructions[frame.pc];
            frame.pc += 1;
            match instruction {
                Instruction::LoadLiteral(value) => frame.stack.push(value_from_literal(value)?),
                Instruction::LoadName(name) => frame.stack.push(
                    frame
                        .names
                        .get(name)
                        .cloned()
                        .ok_or_else(|| format!("runtime name `{name}` was not found"))?,
                ),
                Instruction::StoreName(name) => {
                    let value = frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on store".to_owned())?;
                    frame.names.insert(name.clone(), value);
                }
                Instruction::WidenFloat => {
                    let value = frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on widening".to_owned())?;
                    frame.stack.push(match value {
                        Value::Int(value) => Value::Float(value as f64),
                        Value::Float(_) => value,
                        _ => return Err("Float conversion expects Int or Float".to_owned()),
                    });
                }
                Instruction::Binary(operator) => {
                    let right = frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on binary operator".to_owned())?;
                    let left = frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on binary operator".to_owned())?;
                    frame.stack.push(binary(operator, left, right)?);
                }
                Instruction::MakeArray { length } => {
                    if *length > frame.stack.len() {
                        return Err("stack underflow on array literal".to_owned());
                    }
                    let mut items = Vec::with_capacity(*length);
                    for _ in 0..*length {
                        items.push(
                            frame
                                .stack
                                .pop()
                                .ok_or_else(|| "stack underflow on array literal".to_owned())?,
                        );
                    }
                    items.reverse();
                    frame.stack.push(Value::Array(items));
                }
                Instruction::MakeStruct { type_name, fields } => {
                    if fields.len() > frame.stack.len() {
                        return Err("stack underflow on struct literal".to_owned());
                    }
                    let mut values = Vec::with_capacity(fields.len());
                    for _ in fields {
                        values.push(
                            frame
                                .stack
                                .pop()
                                .ok_or_else(|| "stack underflow on struct literal".to_owned())?,
                        );
                    }
                    values.reverse();
                    let mut seen = std::collections::HashSet::new();
                    if fields.iter().any(|field| !seen.insert(field)) {
                        return Err(format!("duplicate field in struct `{type_name}`"));
                    }
                    frame.stack.push(Value::Struct(Box::new(StructValue {
                        type_name: type_name.clone(),
                        fields: fields.iter().cloned().zip(values).collect(),
                    })));
                }
                Instruction::LoadField(field) => {
                    let target = frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on field access".to_owned())?;
                    match target {
                        Value::Struct(record) => {
                            let value = record
                                .fields
                                .iter()
                                .find(|(name, _)| name == field)
                                .map(|(_, value)| value.clone())
                                .ok_or_else(|| format!("struct value has no field `{field}`"))?;
                            frame.stack.push(value);
                        }
                        _ => return Err("field access requires a struct value".to_owned()),
                    }
                }
                Instruction::Index => {
                    let index = frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on index".to_owned())?;
                    let target = frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on index".to_owned())?;
                    match (target, index) {
                        (Value::Array(values), Value::Int(index)) => {
                            let index = index as usize;
                            if index >= values.len() {
                                return Err("array index out of bounds".to_owned());
                            }
                            frame.stack.push(values[index].clone());
                        }
                        _ => return Err("index requires an Array and Int index".to_owned()),
                    }
                }
                Instruction::StoreIndex => {
                    let value = frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on store-index".to_owned())?;
                    let index = frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on store-index".to_owned())?;
                    let mut target = frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on store-index".to_owned())?;
                    match (&mut target, index) {
                        (Value::Array(values), Value::Int(index)) => {
                            let index = index as usize;
                            if index >= values.len() {
                                return Err("array index out of bounds".to_owned());
                            }
                            values[index] = value;
                            frame.stack.push(target);
                        }
                        _ => {
                            return Err(
                                "index assignment requires an Array and Int index".to_owned()
                            )
                        }
                    }
                }
                Instruction::Jump { target } => {
                    if *target > frame.function.instructions.len() {
                        return Err("jump target out of bounds".to_owned());
                    }
                    frame.pc = *target;
                }
                Instruction::JumpIfFalse { target } => {
                    if *target > frame.function.instructions.len() {
                        return Err("jump target out of bounds".to_owned());
                    }
                    let condition = frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on jump-if-false".to_owned())?;
                    let is_true = match condition {
                        Value::Bool(value) => value,
                        _ => return Err("if condition must be Bool".to_owned()),
                    };
                    if !is_true {
                        frame.pc = *target;
                    }
                }
                Instruction::Call { name, arguments } => {
                    if *arguments > frame.stack.len() {
                        return Err("stack underflow on call".to_owned());
                    }
                    let mut call_arguments = Vec::with_capacity(*arguments);
                    for _ in 0..*arguments {
                        call_arguments.push(
                            frame
                                .stack
                                .pop()
                                .ok_or_else(|| "stack underflow on call".to_owned())?,
                        );
                    }
                    call_arguments.reverse();
                    if stdlib::lookup(name).is_some() {
                        execute_std_call(name, &call_arguments, &mut output, &mut frame.stack)?;
                    } else {
                        let callee = program
                            .functions
                            .iter()
                            .find(|function| function.name == name.as_str())
                            .ok_or_else(|| format!("runtime function `{name}` was not found"))?;
                        if active_frames >= MAX_CALL_DEPTH {
                            return Err(format!(
                                "maximum call depth of {MAX_CALL_DEPTH} exceeded in `{}`",
                                callee.name
                            ));
                        }
                        pending_frame = Some(call_frame(callee, &call_arguments)?);
                    }
                }
                Instruction::Pop => {
                    frame
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on pop".to_owned())?;
                }
                Instruction::Return => {
                    return_value = Some(frame.stack.pop().unwrap_or(Value::Unit));
                }
            }
        }

        if let Some(return_value) = return_value {
            frames.pop();
            if let Some(caller) = frames.last_mut() {
                caller.stack.push(return_value);
            } else {
                return Ok((output, return_value));
            }
        } else if let Some(callee) = pending_frame {
            frames.push(callee);
        }
    }
}

fn execute_std_call(
    name: &str,
    arguments: &[Value],
    output: &mut Vec<String>,
    stack: &mut Vec<Value>,
) -> Result<(), String> {
    let function = stdlib::lookup(name).expect("standard-library call was already checked");
    if arguments.len() != function.parameters.len() {
        return Err(format!(
            "{name} expects exactly {} argument(s)",
            function.parameters.len()
        ));
    }
    match function.name {
        "std::print" | "std::println" => {
            output.push(arguments[0].display());
            stack.push(Value::Unit);
        }
        "std::len" => {
            let value = match &arguments[0] {
                Value::String(value) => Value::Int(value.len() as i64),
                _ => return Err(format!("{name} expects a String argument")),
            };
            stack.push(value);
        }
        "std::to_string" => {
            let value = match &arguments[0] {
                Value::Int(value) => Value::String(value.to_string()),
                Value::Float(value) => Value::String(value.to_string()),
                Value::Bool(value) => Value::String(value.to_string()),
                Value::String(value) => Value::String(value.clone()),
                Value::Array(values) => Value::String(format!(
                    "[{}]",
                    values
                        .iter()
                        .map(Value::display)
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                value @ Value::Struct(_) => Value::String(value.display()),
                Value::Unit => Value::String(String::new()),
            };
            stack.push(value);
        }
        _ => {
            return Err(format!(
                "standard-library function `{name}` is not implemented"
            ))
        }
    }
    Ok(())
}

fn value_from_literal(value: &Literal) -> Result<Value, String> {
    match value {
        Literal::Integer(value) => value
            .parse()
            .map(Value::Int)
            .map_err(|_| "invalid or out-of-range integer literal".to_owned()),
        Literal::Float(value) => value
            .parse()
            .map(Value::Float)
            .map_err(|_| "invalid float literal".to_owned()),
        Literal::Boolean(value) => Ok(Value::Bool(*value)),
        Literal::String(value) => Ok(Value::String(value.clone())),
    }
}

fn binary(operator: &str, left: Value, right: Value) -> Result<Value, String> {
    let (left, right) = match (left, right) {
        (Value::Int(left), Value::Float(right)) => (Value::Float(left as f64), Value::Float(right)),
        (Value::Float(left), Value::Int(right)) => (Value::Float(left), Value::Float(right as f64)),
        values => values,
    };
    if operator == "/" {
        let zero = match &right {
            Value::Int(value) => *value == 0,
            Value::Float(value) => *value == 0.0,
            _ => false,
        };
        if zero {
            return Err("division by zero".to_owned());
        }
    }
    match (operator, left, right) {
        ("+", Value::Int(left), Value::Int(right)) => checked_integer(left.checked_add(right)),
        ("-", Value::Int(left), Value::Int(right)) => checked_integer(left.checked_sub(right)),
        ("*", Value::Int(left), Value::Int(right)) => checked_integer(left.checked_mul(right)),
        ("/", Value::Int(left), Value::Int(right)) => checked_integer(left.checked_div(right)),
        ("+", Value::Float(left), Value::Float(right)) => Ok(Value::Float(left + right)),
        ("-", Value::Float(left), Value::Float(right)) => Ok(Value::Float(left - right)),
        ("*", Value::Float(left), Value::Float(right)) => Ok(Value::Float(left * right)),
        ("/", Value::Float(left), Value::Float(right)) => Ok(Value::Float(left / right)),
        ("+", Value::String(left), Value::String(right)) => Ok(Value::String(left + &right)),
        ("==", left, right) => Ok(Value::Bool(left == right)),
        ("!=", left, right) => Ok(Value::Bool(left != right)),
        ("<", Value::Int(left), Value::Int(right)) => Ok(Value::Bool(left < right)),
        ("<=", Value::Int(left), Value::Int(right)) => Ok(Value::Bool(left <= right)),
        (">", Value::Int(left), Value::Int(right)) => Ok(Value::Bool(left > right)),
        (">=", Value::Int(left), Value::Int(right)) => Ok(Value::Bool(left >= right)),
        ("<", Value::Float(left), Value::Float(right)) => Ok(Value::Bool(left < right)),
        ("<=", Value::Float(left), Value::Float(right)) => Ok(Value::Bool(left <= right)),
        (">", Value::Float(left), Value::Float(right)) => Ok(Value::Bool(left > right)),
        (">=", Value::Float(left), Value::Float(right)) => Ok(Value::Bool(left >= right)),
        ("<", Value::String(left), Value::String(right)) => Ok(Value::Bool(left < right)),
        ("<=", Value::String(left), Value::String(right)) => Ok(Value::Bool(left <= right)),
        (">", Value::String(left), Value::String(right)) => Ok(Value::Bool(left > right)),
        (">=", Value::String(left), Value::String(right)) => Ok(Value::Bool(left >= right)),
        _ => Err(format!("unsupported runtime operation `{operator}`")),
    }
}

fn checked_integer(value: Option<i64>) -> Result<Value, String> {
    value
        .map(Value::Int)
        .ok_or_else(|| "integer overflow".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excessive_ir_argument_count_fails_before_allocation() {
        let program = IrProgram {
            functions: vec![IrFunction {
                name: "main".into(),
                parameters: vec![],
                instructions: vec![Instruction::Call {
                    name: "print".into(),
                    arguments: usize::MAX,
                }],
            }],
        };
        assert_eq!(run(&program).unwrap_err(), "stack underflow on call");
    }
    use crate::compiler::{ir, parser::Parser, semantic::SemanticAnalyzer};

    #[test]
    fn numeric_example_preserves_widening_and_integer_precision() {
        let parsed = Parser::new()
            .parse_source(include_str!("../../examples/numbers/main.svr"))
            .expect("valid numeric syntax");
        let program = ir::lower_program(&parsed).expect("valid numeric types");
        let output = run(&program).expect("numeric program must execute");
        let expected: Vec<_> = include_str!("../../tests/fixtures/numbers.stdout")
            .lines()
            .collect();
        assert_eq!(output, expected);
    }

    #[test]
    fn numeric_overflow_is_a_runtime_error() {
        for expression in [
            "9223372036854775807 + 1",
            "(0 - 9223372036854775807 - 1) - 1",
            "9223372036854775807 * 2",
            "(0 - 9223372036854775807 - 1) / (0 - 1)",
        ] {
            let parsed = Parser::new()
                .parse_source(&format!("fn main() {{ print({expression}) }}"))
                .expect("valid numeric syntax");
            let program = ir::lower_program(&parsed).expect("valid numeric types");
            assert_eq!(
                run(&program),
                Err("integer overflow".into()),
                "{expression}"
            );
        }
    }

    #[test]
    fn executes_print_program() {
        let program = Parser::new()
            .parse_source("fn main() { print(\"Hello, Sovra!\") }")
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should type-check");
        let output = run(&ir::lower(&typed)).expect("program should execute");
        assert_eq!(output, vec!["Hello, Sovra!"]);
    }

    #[test]
    fn executes_std_library_helpers() {
        let program = Parser::new()
            .parse_source("fn main() { let text = std::to_string(42); print(std::len(text)); std::println(text) }")
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should type-check");
        let output = run(&ir::lower(&typed)).expect("program should execute");
        assert_eq!(output, vec!["2", "42"]);
    }

    #[test]
    fn preserves_string_literals_that_look_numeric() {
        let program = Parser::new()
            .parse_source("fn main() { print(\"42\") }")
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should type-check");
        let output = run(&ir::lower(&typed)).expect("program should execute");
        assert_eq!(output, vec!["42"]);
    }

    #[test]
    fn executes_user_function_and_return_value() {
        let program = Parser::new()
            .parse_source(
                "fn add(left: Int, right: Int) -> Int { return left + right } \
                 fn main() { print(add(2, 3)) }",
            )
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should type-check");
        let output = run(&ir::lower(&typed)).expect("program should execute");
        assert_eq!(output, vec!["5"]);
    }

    #[test]
    fn executes_float_and_comparison_operations() {
        let program = Parser::new()
            .parse_source("fn main() { print(1.5 + 2.5); print(3 > 2) }")
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should type-check");
        let output = run(&ir::lower(&typed)).expect("program should execute");
        assert_eq!(output, vec!["4", "true"]);
    }

    #[test]
    fn reports_division_by_zero() {
        let program = Parser::new()
            .parse_source("fn main() { print(1 / 0) }")
            .expect("source should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("source should type-check");
        let error = run(&ir::lower(&typed)).expect_err("program should fail");
        assert_eq!(error, "division by zero");
    }

    #[test]
    fn reports_excessive_call_depth() {
        let program = IrProgram {
            functions: vec![
                IrFunction {
                    name: "main".into(),
                    parameters: Vec::new(),
                    instructions: vec![Instruction::Call {
                        name: "loop".into(),
                        arguments: 0,
                    }],
                },
                IrFunction {
                    name: "loop".into(),
                    parameters: Vec::new(),
                    instructions: vec![Instruction::Call {
                        name: "loop".into(),
                        arguments: 0,
                    }],
                },
            ],
        };
        let error = run(&program).expect_err("recursive program should fail");
        assert!(error.contains("maximum call depth"));
    }
}
