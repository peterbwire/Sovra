//! Initial opt-in Rust execution of sealed versioned single-function IR.
use super::{
    CallableTarget, ExpressionLiteral, ExpressionOperation as Expr, ExpressionValueId,
    FunctionInstruction as Op, FunctionIr, FunctionIrError, FunctionIrLimits, FunctionOperand,
    RuntimeValueError, RuntimeValueFuel as Fuel, RuntimeValueV2 as Value, ScopedDeclarationId,
    ScopedDeclarationKind,
};
use crate::compiler::{
    execution::{ExecutionControl, RuntimePolicy},
    semantic::Type,
};
use std::collections::HashMap;
/// Explicit trusted UTF-8 line input. This interface grants no file/output authority.
/// Host allocations and blocking work are outside VM fuel and cooperative interruption.
pub trait RuntimeVmInput {
    /// Return one line without its terminator, or None at EOF. Errors produce no VM value.
    /// Hosts should enforce MAX_INPUT_LINE_BYTES while reading; the VM also checks returns.
    fn read_line(&mut self) -> Result<Option<String>, String>;
}
// Explicitly supplying this trait grants process stdin only. The legacy reader
// enforces the same byte/UTF-8/terminator contract and publishes no VM output.
impl RuntimeVmInput for crate::compiler::interpreter::ProcessHost {
    fn read_line(&mut self) -> Result<Option<String>, String> {
        crate::compiler::interpreter::RuntimeHost::read_line(self)
    }
}
/// Explicit trusted file authority. Implementations own path confinement, bounded
/// UTF-8 reads, atomic replacement and conditional-write coordination contracts.
/// The VM itself never opens paths; blocking services are not interrupted.
pub trait RuntimeVmFiles {
    /// Read text with the existing standard failure categories and byte ceiling.
    fn read_text(&mut self, path: &str) -> crate::compiler::text_files::TextRead;
    /// Replace text under the existing bounded/atomic write contract.
    fn write_text(&mut self, path: &str, text: &str) -> crate::compiler::text_files::TextWrite;
    /// Conditionally replace text under the existing comparison/coordination contract.
    fn write_text_if(
        &mut self,
        path: &str,
        exists: bool,
        expected: &str,
        text: &str,
    ) -> crate::compiler::text_files::TextWrite;
}
// Explicitly passing ProcessHost grants its existing filesystem authority. This
// delegates its bounded/atomic/conditional operations without installing a default.
impl RuntimeVmFiles for crate::compiler::interpreter::ProcessHost {
    fn read_text(&mut self, path: &str) -> crate::compiler::text_files::TextRead {
        crate::compiler::interpreter::RuntimeHost::read_text(self, path)
    }
    fn write_text(&mut self, path: &str, text: &str) -> crate::compiler::text_files::TextWrite {
        crate::compiler::interpreter::RuntimeHost::write_text(self, path, text)
    }
    fn write_text_if(
        &mut self,
        path: &str,
        exists: bool,
        expected: &str,
        text: &str,
    ) -> crate::compiler::text_files::TextWrite {
        crate::compiler::interpreter::RuntimeHost::write_text_if(self, path, exists, expected, text)
    }
}
/// Independently granted services. Absent capabilities refuse during preflight.
#[derive(Default)]
pub struct RuntimeVmServices<'a> {
    /// Optional line-input authority; print remains memory-only capture.
    pub input: Option<&'a mut dyn RuntimeVmInput>,
    /// Optional file authority; no automatic process filesystem is installed.
    pub files: Option<&'a mut dyn RuntimeVmFiles>,
}
impl std::fmt::Debug for RuntimeVmServices<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeVmServices")
            .field("input", &self.input.is_some())
            .field("files", &self.files.is_some())
            .finish()
    }
}
/// Explicit initial VM profile; unsupported standard/host calls refuse before execution.
#[derive(Debug, Clone)]
pub struct RuntimeVmPolicy {
    /// Shared instruction/output and model-v2 materialization limits for all frames.
    pub runtime: RuntimePolicy,
    /// Cooperative checkpoints; cannot interrupt helpers, cleanup or blocked host work.
    pub control: ExecutionControl,
    /// Active invocation frames, including the entry; fixed ceiling is 256.
    pub max_call_depth: usize,
    /// Pre-execution static/flow validation admission.
    pub validation: FunctionIrLimits,
}
impl RuntimeVmPolicy {
    /// Opt-in execution profile preserving unlimited component budgets; not a CLI default.
    pub const DEFAULT: Self = Self {
        runtime: RuntimePolicy::UNLIMITED,
        control: ExecutionControl::UNLIMITED,
        max_call_depth: 256,
        validation: FunctionIrLimits::DEFAULT,
    };
}
/// Typed execution refusal; no unlimited/legacy backend fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeVmError {
    /// Sealed function/flow validation failed.
    Validation(FunctionIrError),
    /// Existing instruction/output/cancellation/elapsed refusal.
    Execution(String),
    /// Value/fuel/type/refused snapshot operation.
    Value(RuntimeValueError),
    /// This initial VM has no adapter for the named standard callable.
    UnsupportedStandard(&'static str),
    /// Runtime argument count or primitive-operation arity mismatch.
    Arity,
    /// Entry/self-call frame allowance exceeded or exceeds the fixed ceiling.
    CallDepth,
    /// Checked integer overflow, division by zero, or invalid runtime numeric operation.
    Arithmetic(&'static str),
    /// Integer array index is negative or unavailable.
    Bounds,
    /// Selected process status is outside the existing portable 0..=125 range.
    ExitCode,
    /// Supplied input host failed; no result value was materialized.
    Host(String),
    /// Supplied host returned more than MAX_INPUT_LINE_BYTES of UTF-8 text.
    InputLimit,
    /// A supplied file host returned text over the existing file byte ceiling.
    FileLimit,
    /// Internal sealed-IR availability/shape agreement was unavailable.
    Invalid(&'static str),
}
impl std::fmt::Display for RuntimeVmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "versioned execution refused: {self:?}")
    }
}
impl std::error::Error for RuntimeVmError {}
impl From<RuntimeValueError> for RuntimeVmError {
    fn from(e: RuntimeValueError) -> Self {
        Self::Value(e)
    }
}
/// Captured successful execution. Printed lines exclude LF; output limits include it.
#[derive(Debug)]
pub struct RuntimeVmOutcome {
    /// Transferred function result; caller-owned deep cleanup is flat/iterative.
    pub value: Value,
    /// Memory-only print output; separately granted services may already have effects.
    pub output: Vec<String>,
    /// Shared remaining component allowances; drops never refund fuel.
    pub remaining: RuntimePolicy,
}
/// Successful command-context execution; no process status is changed by the VM.
#[derive(Debug)]
pub struct RuntimeVmCommandOutcome {
    /// Captured value/output and shared remaining allowances.
    pub execution: RuntimeVmOutcome,
    /// Requested exit status, initially zero; caller decides how to publish it.
    pub exit_code: u8,
}
struct CommandContext<'a> {
    arguments: Option<&'a [String]>,
    input: Option<&'a mut dyn RuntimeVmInput>,
    files: Option<&'a mut dyn RuntimeVmFiles>,
    exit_code: u8,
}
struct Evaluation {
    root: usize,
    next: usize,
    end: usize,
    registers: HashMap<ExpressionValueId, Value>,
    waiting: Option<ExpressionValueId>,
}
struct Frame {
    function: usize,
    pc: usize,
    arguments: Vec<Option<Value>>,
    bindings: HashMap<ScopedDeclarationId, Value>,
    scopes: Vec<(usize, Vec<ScopedDeclarationId>)>,
    roots: HashMap<usize, Value>,
    converted: HashMap<usize, Value>,
    snapshots: HashMap<usize, Value>,
    evaluation: Option<Evaluation>,
}
impl Frame {
    fn new(
        ir: &FunctionIr<'_>,
        arguments: &[Value],
        fuel: &mut Fuel,
    ) -> Result<Self, RuntimeVmError> {
        let declarations = ir
            .expressions()
            .mapping()
            .checked()
            .bindings()
            .declarations();
        let params: Vec<_> = declarations
            .iter()
            .filter(|d| d.kind == ScopedDeclarationKind::Parameter)
            .collect();
        if arguments.len() != params.len() {
            return Err(RuntimeVmError::Arity);
        }
        let env = ir
            .expressions()
            .mapping()
            .checked()
            .typed()
            .callable()
            .function()
            .environment();
        for (arg, param) in arguments.iter().zip(&params) {
            arg.validate_as(env, &param.ty)?;
        }
        let mut copied = Vec::with_capacity(arguments.len());
        for (arg, param) in arguments.iter().zip(params) {
            fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
            copied.push(Some(arg.snapshot(fuel)?.coerce(env, &param.ty)?));
        }
        Ok(Self {
            function: 0,
            pc: 0,
            arguments: copied,
            bindings: HashMap::new(),
            scopes: Vec::new(),
            roots: HashMap::new(),
            converted: HashMap::new(),
            snapshots: HashMap::new(),
            evaluation: None,
        })
    }
    fn bind(&mut self, id: ScopedDeclarationId, value: Value) -> Result<(), RuntimeVmError> {
        if self.bindings.contains_key(&id) {
            return Err(RuntimeVmError::Invalid("binding already active"));
        }
        self.scopes
            .last_mut()
            .ok_or(RuntimeVmError::Invalid("missing scope"))?
            .1
            .push(id);
        self.bindings.insert(id, value);
        Ok(())
    }
    fn take(&mut self, operand: FunctionOperand) -> Result<Value, RuntimeVmError> {
        match operand {
            FunctionOperand::Root(r) => self.roots.remove(&r),
            FunctionOperand::Converted(c) => self.converted.remove(&c),
        }
        .ok_or(RuntimeVmError::Invalid("value availability"))
    }
}
enum Action {
    Continue,
    Call {
        target: Option<super::ProgramFunctionId>,
        arguments: Vec<Value>,
    },
    Return(Value),
}
fn take(
    registers: &mut HashMap<ExpressionValueId, Value>,
    id: ExpressionValueId,
) -> Result<Value, RuntimeVmError> {
    registers
        .remove(&id)
        .ok_or(RuntimeVmError::Invalid("expression value availability"))
}
fn index(value: &Value) -> Result<usize, RuntimeVmError> {
    usize::try_from(value.as_int().ok_or(RuntimeVmError::Bounds)?)
        .map_err(|_| RuntimeVmError::Bounds)
}
fn binary(op: &str, left: Value, right: Value, fuel: &mut Fuel) -> Result<Value, RuntimeVmError> {
    if matches!(op, "==" | "!=") {
        let same = left.structurally_equal(&right);
        return Ok(Value::boolean(if op == "==" { same } else { !same }, fuel)?);
    }
    if let (Some(a), Some(b)) = (left.as_int(), right.as_int()) {
        if matches!(op, "<" | "<=" | ">" | ">=") {
            return Ok(Value::boolean(
                match op {
                    "<" => a < b,
                    "<=" => a <= b,
                    ">" => a > b,
                    _ => a >= b,
                },
                fuel,
            )?);
        }
        if op == "/" && b == 0 {
            return Err(RuntimeVmError::Arithmetic("division by zero"));
        }
        let value = match op {
            "+" => a.checked_add(b),
            "-" => a.checked_sub(b),
            "*" => a.checked_mul(b),
            "/" => a.checked_div(b),
            _ => return Err(RuntimeVmError::Arithmetic("unsupported Int operation")),
        }
        .ok_or(RuntimeVmError::Arithmetic("integer overflow"))?;
        return Ok(Value::int(value, fuel)?);
    }
    if let (Some(a), Some(b)) = (left.as_float(), right.as_float()) {
        if matches!(op, "<" | "<=" | ">" | ">=") {
            return Ok(Value::boolean(
                match op {
                    "<" => a < b,
                    "<=" => a <= b,
                    ">" => a > b,
                    _ => a >= b,
                },
                fuel,
            )?);
        }
        if op == "/" && b == 0.0 {
            return Err(RuntimeVmError::Arithmetic("division by zero"));
        }
        return Ok(Value::float(
            match op {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "/" => a / b,
                _ => return Err(RuntimeVmError::Arithmetic("unsupported Float operation")),
            },
            fuel,
        )?);
    }
    if let (Some(a), Some(b)) = (left.as_bool(), right.as_bool()) {
        return Ok(Value::boolean(
            match op {
                "&&" => a && b,
                "||" => a || b,
                _ => return Err(RuntimeVmError::Arithmetic("unsupported Bool operation")),
            },
            fuel,
        )?);
    }
    if let (Some(a), Some(b)) = (left.as_str(), right.as_str()) {
        if op == "+" {
            let len = a
                .len()
                .checked_add(b.len())
                .ok_or(RuntimeValueError::SizeOverflow)?;
            fuel.admit(
                1,
                u64::try_from(len).map_err(|_| RuntimeValueError::SizeOverflow)?,
            )?;
            let mut s = String::with_capacity(len);
            s.push_str(a);
            s.push_str(b);
            return Ok(Value::string_admitted(s));
        }
        return Ok(Value::boolean(
            match op {
                "<" => a < b,
                "<=" => a <= b,
                ">" => a > b,
                ">=" => a >= b,
                _ => return Err(RuntimeVmError::Arithmetic("unsupported String operation")),
            },
            fuel,
        )?);
    }
    Err(RuntimeVmError::Arithmetic("operand types"))
}
fn standard(
    name: &'static str,
    mut args: Vec<Value>,
    fuel: &mut Fuel,
    output: &mut Vec<String>,
    command: &mut CommandContext<'_>,
) -> Result<Value, RuntimeVmError> {
    match name {
        "std::read_text" => {
            let [path] = args.as_slice() else {
                return Err(RuntimeVmError::Arity);
            };
            let path = path.as_str().ok_or(RuntimeValueError::TypeMismatch)?;
            let files = command
                .files
                .as_deref_mut()
                .ok_or(RuntimeVmError::UnsupportedStandard(name))?;
            Value::preflight_text_read(fuel)?;
            fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
            let result = files.read_text(path);
            fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
            if result.text.len() > crate::compiler::stdlib::MAX_TEXT_FILE_BYTES {
                return Err(RuntimeVmError::FileLimit);
            }
            return Ok(Value::text_read(result, fuel)?);
        }
        "std::write_text" | "std::write_text_if" => {
            let files = command
                .files
                .as_deref_mut()
                .ok_or(RuntimeVmError::UnsupportedStandard(name))?;
            let result = if name == "std::write_text" {
                let [path, text] = args.as_slice() else {
                    return Err(RuntimeVmError::Arity);
                };
                let path = path.as_str().ok_or(RuntimeValueError::TypeMismatch)?;
                let text = text.as_str().ok_or(RuntimeValueError::TypeMismatch)?;
                Value::preflight_text_write(fuel)?;
                fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
                files.write_text(path, text)
            } else {
                let [path, exists, expected, text] = args.as_slice() else {
                    return Err(RuntimeVmError::Arity);
                };
                let path = path.as_str().ok_or(RuntimeValueError::TypeMismatch)?;
                let exists = exists.as_bool().ok_or(RuntimeValueError::TypeMismatch)?;
                let expected = expected.as_str().ok_or(RuntimeValueError::TypeMismatch)?;
                let text = text.as_str().ok_or(RuntimeValueError::TypeMismatch)?;
                Value::preflight_text_write(fuel)?;
                fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
                files.write_text_if(path, exists, expected, text)
            };
            fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
            return Ok(Value::text_write(result, fuel)?);
        }
        "std::read_line" => {
            if !args.is_empty() {
                return Err(RuntimeVmError::Arity);
            }
            let input = command
                .input
                .as_deref_mut()
                .ok_or(RuntimeVmError::UnsupportedStandard(name))?;
            Value::preflight_input_line(fuel)?;
            fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
            let text = input.read_line().map_err(RuntimeVmError::Host)?;
            fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
            if text
                .as_ref()
                .is_some_and(|s| s.len() > crate::compiler::stdlib::MAX_INPUT_LINE_BYTES)
            {
                return Err(RuntimeVmError::InputLimit);
            }
            return Ok(Value::input_line(text, fuel)?);
        }
        "std::arg_count" => {
            if !args.is_empty() {
                return Err(RuntimeVmError::Arity);
            }
            let arguments = command
                .arguments
                .ok_or(RuntimeVmError::UnsupportedStandard(name))?;
            fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
            let count =
                i64::try_from(arguments.len()).map_err(|_| RuntimeValueError::SizeOverflow)?;
            fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
            return Ok(Value::int(count, fuel)?);
        }
        "std::arg" => {
            let [index] = args.as_slice() else {
                return Err(RuntimeVmError::Arity);
            };
            let index = index.as_int().ok_or(RuntimeValueError::TypeMismatch)?;
            let arguments = command
                .arguments
                .ok_or(RuntimeVmError::UnsupportedStandard(name))?;
            fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
            let value = usize::try_from(index)
                .ok()
                .and_then(|i| arguments.get(i))
                .ok_or(RuntimeVmError::Bounds)?;
            fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
            return Ok(Value::string(value, fuel)?);
        }
        "std::set_exit_code" => {
            let [code] = args.as_slice() else {
                return Err(RuntimeVmError::Arity);
            };
            command
                .arguments
                .ok_or(RuntimeVmError::UnsupportedStandard(name))?;
            let code = code.as_int().ok_or(RuntimeValueError::TypeMismatch)?;
            let code = u8::try_from(code)
                .ok()
                .filter(|c| *c <= 125)
                .ok_or(RuntimeVmError::ExitCode)?;
            let result = Value::unit(fuel)?;
            command.exit_code = code;
            return Ok(result);
        }
        _ => {}
    }
    match name {
        "std::split_once" => {
            let [text, delimiter] = args.as_slice() else {
                return Err(RuntimeVmError::Arity);
            };
            return Ok(Value::split_once(
                text.as_str().ok_or(RuntimeValueError::TypeMismatch)?,
                delimiter.as_str().ok_or(RuntimeValueError::TypeMismatch)?,
                fuel,
            )?);
        }
        "std::array_slice" => {
            let [value, start, end] = args.as_slice() else {
                return Err(RuntimeVmError::Arity);
            };
            let start = usize::try_from(start.as_int().ok_or(RuntimeValueError::TypeMismatch)?)
                .map_err(|_| RuntimeVmError::Bounds)?;
            let end = usize::try_from(end.as_int().ok_or(RuntimeValueError::TypeMismatch)?)
                .map_err(|_| RuntimeVmError::Bounds)?;
            let len = value.array_len().ok_or(RuntimeValueError::TypeMismatch)?;
            if start > end || end > len {
                return Err(RuntimeVmError::Bounds);
            }
            return Ok(value.slice_snapshot(start, end, fuel)?);
        }
        "std::array_append" | "std::array_concat" => {
            let [left, right] = args.as_slice() else {
                return Err(RuntimeVmError::Arity);
            };
            return Ok(left.growth_snapshot(right, name == "std::array_concat", fuel)?);
        }
        _ => {}
    }
    if args.len() != 1 {
        return Err(RuntimeVmError::Arity);
    }
    let value = args.pop().ok_or(RuntimeVmError::Arity)?;
    match name {
        "std::print" | "std::println" => {
            let size = value.logical_footprint()?;
            usize::try_from(size.display_bytes).map_err(|_| RuntimeValueError::SizeOverflow)?;
            fuel.preflight_output(size.display_bytes)
                .map_err(RuntimeVmError::Execution)?;
            fuel.admit(1, size.display_bytes)?;
            fuel.commit_output(size.display_bytes)
                .map_err(RuntimeVmError::Execution)?;
            output.push(value.render_admitted());
            Ok(Value::unit_admitted())
        }
        "std::to_string" => Ok(Value::string_admitted(value.render(fuel, None)?)),
        "std::parse_int" => Ok(Value::parse_int(
            value.as_str().ok_or(RuntimeValueError::TypeMismatch)?,
            fuel,
        )?),
        "std::escape_terminal" => Ok(Value::escape_terminal(
            value.as_str().ok_or(RuntimeValueError::TypeMismatch)?,
            fuel,
        )?),
        "std::lines_unique" => {
            let text = value.as_str().ok_or(RuntimeValueError::TypeMismatch)?;
            // Preserve admission before the helper's unmetered set/split work.
            fuel.admit(1, 0)?;
            Ok(Value::bool_admitted(crate::compiler::stdlib::lines_unique(
                text,
            )))
        }
        "std::array_len" => Ok(Value::int(
            i64::try_from(value.array_len().ok_or(RuntimeValueError::TypeMismatch)?)
                .map_err(|_| RuntimeValueError::SizeOverflow)?,
            fuel,
        )?),
        "std::len" => Ok(Value::int(
            i64::try_from(value.as_str().ok_or(RuntimeValueError::TypeMismatch)?.len())
                .map_err(|_| RuntimeValueError::SizeOverflow)?,
            fuel,
        )?),
        _ => Err(RuntimeVmError::UnsupportedStandard(name)),
    }
}
fn eval(
    ir: &FunctionIr<'_>,
    frame: &mut Frame,
    fuel: &mut Fuel,
    output: &mut Vec<String>,
    command: &mut CommandContext<'_>,
) -> Result<Action, RuntimeVmError> {
    let evaluation = frame
        .evaluation
        .as_mut()
        .ok_or(RuntimeVmError::Invalid("missing evaluation"))?;
    if evaluation.next == evaluation.end {
        let result = ir.expressions().roots()[evaluation.root].result;
        let value = take(&mut evaluation.registers, result)?;
        if !evaluation.registers.is_empty() {
            return Err(RuntimeVmError::Invalid("unused expression temporary"));
        }
        let root = evaluation.root;
        frame.evaluation = None;
        frame.roots.insert(root, value);
        return Ok(Action::Continue);
    }
    fuel.instruction().map_err(RuntimeVmError::Execution)?;
    let instruction = &ir.expressions().instructions()[evaluation.next];
    evaluation.next += 1;
    let env = ir
        .expressions()
        .mapping()
        .checked()
        .typed()
        .callable()
        .function()
        .environment();
    let registers = &mut evaluation.registers;
    let value = match &instruction.operation {
        Expr::Literal(literal) => match literal {
            ExpressionLiteral::Int(v) => Value::int(
                v.parse()
                    .map_err(|_| RuntimeVmError::Arithmetic("integer literal"))?,
                fuel,
            )?,
            ExpressionLiteral::Float(v) => Value::float(
                v.parse()
                    .map_err(|_| RuntimeVmError::Arithmetic("Float literal"))?,
                fuel,
            )?,
            ExpressionLiteral::Bool(v) => Value::boolean(*v, fuel)?,
            ExpressionLiteral::String(v) => Value::string(v, fuel)?,
        },
        Expr::Read { declaration, .. } => frame
            .bindings
            .get(declaration)
            .ok_or(RuntimeVmError::Invalid("read binding"))?
            .snapshot(fuel)?,
        Expr::Convert { value, context } => {
            let value = take(registers, *value)?;
            let expected = &ir
                .expressions()
                .mapping()
                .checked()
                .typed()
                .callable()
                .evidence()
                .contexts()[*context]
                .expected;
            value.coerce(env, expected)?
        }
        Expr::Binary {
            operator,
            left,
            right,
        } => {
            let left = take(registers, *left)?;
            let right = take(registers, *right)?;
            binary(operator, left, right, fuel)?
        }
        Expr::Call {
            target, arguments, ..
        } => {
            let mut args = Vec::with_capacity(arguments.len());
            for arg in arguments {
                args.push(take(registers, *arg)?);
            }
            match target {
                CallableTarget::Constructor { enum_id, variant } => {
                    if args.len() > 1 {
                        return Err(RuntimeVmError::Arity);
                    }
                    Value::enum_value(env, *enum_id, *variant, args.pop(), fuel)?
                }
                CallableTarget::Standard(name) => standard(name, args, fuel, output, command)?,
                CallableTarget::ProgramFunction(id) => {
                    evaluation.waiting = Some(instruction.result);
                    return Ok(Action::Call {
                        target: Some(*id),
                        arguments: args,
                    });
                }
                CallableTarget::SelfFunction => {
                    evaluation.waiting = Some(instruction.result);
                    return Ok(Action::Call {
                        target: None,
                        arguments: args,
                    });
                }
            }
        }
        Expr::Array(items) => {
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                values.push(take(registers, *item)?);
            }
            Value::array(values, fuel)?
        }
        Expr::Record(fields) => {
            let mut values = Vec::with_capacity(fields.len());
            for (name, value) in fields {
                values.push((*name, take(registers, *value)?));
            }
            let Type::Named(name) = ir
                .expressions()
                .value(instruction.result)
                .ok_or(RuntimeVmError::Invalid("record result"))?
                .ty
            else {
                return Err(RuntimeVmError::Invalid("record type"));
            };
            Value::record(name, values, fuel)?
        }
        Expr::Field { receiver, field } => {
            take(registers, *receiver)?.field_snapshot(field, fuel)?
        }
        Expr::Index { target, index: id } => {
            let target = take(registers, *target)?;
            let index = index(&take(registers, *id)?)?;
            target.array_item_snapshot(index, fuel).map_err(|e| {
                if e == RuntimeValueError::MissingMember {
                    RuntimeVmError::Bounds
                } else {
                    e.into()
                }
            })?
        }
    };
    registers.insert(instruction.result, value);
    Ok(Action::Continue)
}
fn step(
    ir: &FunctionIr<'_>,
    frame: &mut Frame,
    ranges: &[(usize, usize)],
    context_roots: &HashMap<usize, usize>,
    fuel: &mut Fuel,
) -> Result<Action, RuntimeVmError> {
    fuel.instruction().map_err(RuntimeVmError::Execution)?;
    let instruction = ir
        .instructions()
        .get(frame.pc)
        .ok_or(RuntimeVmError::Invalid("function pc"))?;
    frame.pc += 1;
    let checked = ir.expressions().mapping().checked();
    let env = checked.typed().callable().function().environment();
    match instruction {
        Op::EnterScope(id) => frame.scopes.push((*id, Vec::new())),
        Op::ExitScope(id) => {
            let (scope, bindings) = frame
                .scopes
                .pop()
                .ok_or(RuntimeVmError::Invalid("scope exit"))?;
            if scope != *id {
                return Err(RuntimeVmError::Invalid("scope label"));
            }
            for binding in bindings {
                frame.bindings.remove(&binding);
            }
        }
        Op::Parameter { index, declaration } => {
            let value = frame
                .arguments
                .get_mut(*index)
                .and_then(Option::take)
                .ok_or(RuntimeVmError::Arity)?;
            frame.bind(*declaration, value)?;
        }
        Op::Evaluate { root } => {
            let (start, end) = ranges[*root];
            frame.evaluation = Some(Evaluation {
                root: *root,
                next: start,
                end,
                registers: HashMap::new(),
                waiting: None,
            });
        }
        Op::Convert { context, root } => {
            let value = frame
                .roots
                .remove(root)
                .ok_or(RuntimeVmError::Invalid("boundary value"))?;
            let expected = &checked.typed().callable().evidence().contexts()[*context].expected;
            frame
                .converted
                .insert(*context, value.coerce(env, expected)?);
        }
        Op::Bind { declaration, value } => {
            let value = frame.take(*value)?;
            frame.bind(*declaration, value)?;
        }
        Op::Store { declaration, value } => {
            let value = frame.take(*value)?;
            let slot = frame
                .bindings
                .get_mut(declaration)
                .ok_or(RuntimeVmError::Invalid("store binding"))?;
            *slot = value;
        }
        Op::StoreIndex {
            declaration,
            array,
            index: id,
            value,
        } => {
            let array = frame
                .roots
                .remove(array)
                .ok_or(RuntimeVmError::Invalid("array root"))?;
            let index = index(
                &frame
                    .roots
                    .remove(id)
                    .ok_or(RuntimeVmError::Invalid("index root"))?,
            )?;
            let value = frame.take(*value)?;
            let updated = array.store_index(index, value).map_err(|e| {
                if e == RuntimeValueError::MissingMember {
                    RuntimeVmError::Bounds
                } else {
                    e.into()
                }
            })?;
            *frame
                .bindings
                .get_mut(declaration)
                .ok_or(RuntimeVmError::Invalid("array binding"))? = updated;
        }
        Op::Release { root } => {
            frame.roots.remove(root);
            frame
                .converted
                .retain(|c, _| context_roots.get(c) != Some(root));
        }
        Op::SnapshotMatch { index, declaration } => {
            let snapshot = frame
                .bindings
                .get(declaration)
                .ok_or(RuntimeVmError::Invalid("match binding"))?
                .snapshot(fuel)?;
            frame.snapshots.insert(*index, snapshot);
        }
        Op::Switch {
            index,
            enum_id,
            arms,
        } => {
            let (id, variant) = frame
                .snapshots
                .get(index)
                .and_then(Value::enum_tag)
                .ok_or(RuntimeValueError::TypeMismatch)?;
            if id != *enum_id {
                return Err(RuntimeValueError::TypeMismatch.into());
            }
            frame.pc = arms
                .iter()
                .find(|(v, _)| *v == variant)
                .map(|(_, pc)| *pc)
                .ok_or(RuntimeValueError::WrongVariant)?;
        }
        Op::Payload {
            index,
            variant,
            declaration,
        } => {
            let plan = &checked.typed().callable().function().matches()[*index];
            let value = frame
                .snapshots
                .get(index)
                .ok_or(RuntimeVmError::Invalid("match snapshot"))?
                .payload_snapshot(plan.enum_id(), *variant, fuel)?;
            frame.bind(*declaration, value)?;
        }
        Op::ReleaseMatch(index) => {
            frame.snapshots.remove(index);
        }
        Op::BranchFalse { condition, target } => {
            let condition = frame
                .roots
                .remove(condition)
                .and_then(|v| v.as_bool())
                .ok_or(RuntimeValueError::TypeMismatch)?;
            if !condition {
                frame.pc = *target;
            }
        }
        Op::Jump(target) => frame.pc = *target,
        Op::Return(value) => {
            let value = if let Some(value) = value {
                frame.take(*value)?
            } else {
                Value::unit(fuel)?
            };
            value.validate_as(env, ir.return_type())?;
            return Ok(Action::Return(value));
        }
        Op::End => return Ok(Action::Return(Value::unit(fuel)?)),
    }
    Ok(Action::Continue)
}
impl FunctionIr<'_> {
    /// Execute this sealed single-function IR with shared per-invocation budgets.
    /// Supported calls include print/println, pure string helpers and array len/slice/append/concat. Other standard/host
    /// calls refuse during preflight, including unreachable calls. No legacy fallback.
    pub fn execute(
        &self,
        arguments: &[Value],
        policy: RuntimeVmPolicy,
    ) -> Result<RuntimeVmOutcome, RuntimeVmError> {
        self.execute_inner(arguments, None, None, None, policy)
            .map(|outcome| outcome.execution)
    }
    /// Execute with explicitly supplied program arguments and captured exit status.
    /// Adds arg_count/arg/set_exit_code; no process arguments or ambient I/O are read.
    /// Entry function arguments and borrowed program argument text are distinct.
    pub fn execute_with_arguments(
        &self,
        arguments: &[Value],
        program_arguments: &[String],
        policy: RuntimeVmPolicy,
    ) -> Result<RuntimeVmCommandOutcome, RuntimeVmError> {
        self.execute_inner(arguments, Some(program_arguments), None, None, policy)
    }
    /// Execute with explicit program arguments and a caller-supplied line-input host.
    /// Adds read_line only; print stays memory capture and file calls remain refused.
    /// Input may be consumed before cancellation or full returned-text admission fails.
    pub fn execute_with_input(
        &self,
        arguments: &[Value],
        program_arguments: &[String],
        input: &mut dyn RuntimeVmInput,
        policy: RuntimeVmPolicy,
    ) -> Result<RuntimeVmCommandOutcome, RuntimeVmError> {
        self.execute_inner(
            arguments,
            Some(program_arguments),
            Some(input),
            None,
            policy,
        )
    }
    /// Execute with explicit input/file capabilities and program arguments.
    /// Host effects are not rolled back on cancellation or returned-result fuel refusal.
    pub fn execute_with_services<'a>(
        &self,
        arguments: &[Value],
        program_arguments: &'a [String],
        services: RuntimeVmServices<'a>,
        policy: RuntimeVmPolicy,
    ) -> Result<RuntimeVmCommandOutcome, RuntimeVmError> {
        self.execute_inner(
            arguments,
            Some(program_arguments),
            services.input,
            services.files,
            policy,
        )
    }
    fn execute_inner<'a>(
        &self,
        arguments: &[Value],
        program_arguments: Option<&'a [String]>,
        input: Option<&'a mut dyn RuntimeVmInput>,
        files: Option<&'a mut dyn RuntimeVmFiles>,
        policy: RuntimeVmPolicy,
    ) -> Result<RuntimeVmCommandOutcome, RuntimeVmError> {
        run(
            &[self],
            None,
            0,
            arguments,
            CommandContext {
                arguments: program_arguments,
                input,
                files,
                exit_code: 0,
            },
            policy,
        )
    }
}
pub(super) fn execute_linked<'a>(
    functions: &[FunctionIr<'_>],
    ids: &[super::ProgramFunctionId],
    entry: usize,
    arguments: &[Value],
    program_arguments: Option<&'a [String]>,
    services: RuntimeVmServices<'a>,
    policy: RuntimeVmPolicy,
) -> Result<RuntimeVmCommandOutcome, RuntimeVmError> {
    let functions: Vec<_> = functions.iter().collect();
    run(
        &functions,
        Some(ids),
        entry,
        arguments,
        CommandContext {
            arguments: program_arguments,
            input: services.input,
            files: services.files,
            exit_code: 0,
        },
        policy,
    )
}
fn run(
    functions: &[&FunctionIr<'_>],
    ids: Option<&[super::ProgramFunctionId]>,
    entry: usize,
    arguments: &[Value],
    mut command: CommandContext<'_>,
    policy: RuntimeVmPolicy,
) -> Result<RuntimeVmCommandOutcome, RuntimeVmError> {
    let has_input = command.input.is_some();
    let has_files = command.files.is_some();
    let program_arguments = command.arguments;
    if functions.is_empty()
        || entry >= functions.len()
        || ids.is_some_and(|v| v.len() != functions.len())
    {
        return Err(RuntimeVmError::Invalid("program function table"));
    }
    let resolve = |id: super::ProgramFunctionId| -> Result<usize, RuntimeVmError> {
        let ids = ids.ok_or(RuntimeVmError::Invalid(
            "program call requires linked execution",
        ))?;
        if ids.get(id.ordinal()) != Some(&id) {
            return Err(RuntimeVmError::Invalid("foreign program function"));
        }
        Ok(id.ordinal())
    };
    let mut units = Vec::with_capacity(functions.len());
    let mut fuel = Fuel::for_runtime(policy.runtime, policy.control);
    fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
    if policy.max_call_depth == 0 || policy.max_call_depth > crate::compiler::ir::MAX_CALL_DEPTH {
        return Err(RuntimeVmError::CallDepth);
    }
    for (index, ir) in functions.iter().enumerate() {
        if let Some(ids) = ids {
            let checked = ir
                .expressions()
                .mapping()
                .checked()
                .typed()
                .callable()
                .function();
            if !checked.owns_program_function(ids[index]) {
                return Err(RuntimeVmError::Invalid("program function ownership"));
            }
        }
        ir.validate(policy.validation)
            .map_err(RuntimeVmError::Validation)?;
        fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
        let checked = ir.expressions().mapping().checked();
        for call in checked.typed().callable().evidence().calls() {
            if let CallableTarget::ProgramFunction(id) = call.target {
                resolve(id)?;
            }
            if let CallableTarget::Standard(name) = call.target {
                if !matches!(
                    name,
                    "std::print"
                        | "std::println"
                        | "std::split_once"
                        | "std::parse_int"
                        | "std::escape_terminal"
                        | "std::lines_unique"
                        | "std::to_string"
                        | "std::len"
                        | "std::array_len"
                        | "std::array_slice"
                        | "std::array_append"
                        | "std::array_concat"
                ) && !(program_arguments.is_some()
                    && matches!(name, "std::arg_count" | "std::arg" | "std::set_exit_code"))
                    && !(has_input && name == "std::read_line")
                    && !(has_files
                        && matches!(
                            name,
                            "std::read_text" | "std::write_text" | "std::write_text_if"
                        ))
                {
                    return Err(RuntimeVmError::UnsupportedStandard(name));
                }
            }
        }
        let mut ranges = Vec::new();
        let mut start = 0;
        for root in ir.expressions().roots() {
            let end = root.result.ordinal() + 1;
            ranges.push((start, end));
            start = end;
        }
        let root_nodes: HashMap<_, _> = ir
            .expressions()
            .roots()
            .iter()
            .enumerate()
            .map(|(i, r)| (r.source, i))
            .collect();
        let mut context_roots = HashMap::new();
        for link in ir.expressions().mapping().links() {
            if let super::MappedFact::Context(c) = link.fact {
                if let Some(root) = root_nodes.get(&link.node) {
                    context_roots.insert(c, *root);
                }
            }
        }
        units.push((ranges, context_roots));
        fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
    }
    let mut first = Frame::new(functions[entry], arguments, &mut fuel)?;
    first.function = entry;
    let mut frames = vec![first];
    let mut output = Vec::new();
    loop {
        fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
        let frame = frames
            .last_mut()
            .ok_or(RuntimeVmError::Invalid("missing frame"))?;
        let current = frame.function;
        let action = if frame.evaluation.is_some() {
            eval(
                functions[current],
                frame,
                &mut fuel,
                &mut output,
                &mut command,
            )?
        } else {
            step(
                functions[current],
                frame,
                &units[current].0,
                &units[current].1,
                &mut fuel,
            )?
        };
        match action {
            Action::Continue => {}
            Action::Call {
                target,
                arguments: args,
            } => {
                if frames.len() >= policy.max_call_depth {
                    return Err(RuntimeVmError::CallDepth);
                }
                let callee = target.map_or(Ok(current), resolve)?;
                let mut next = Frame::new(functions[callee], &args, &mut fuel)?;
                next.function = callee;
                frames.push(next);
            }
            Action::Return(value) => {
                frames.pop();
                if let Some(parent) = frames.last_mut() {
                    let evaluation = parent
                        .evaluation
                        .as_mut()
                        .ok_or(RuntimeVmError::Invalid("caller evaluation"))?;
                    let result = evaluation
                        .waiting
                        .take()
                        .ok_or(RuntimeVmError::Invalid("return target"))?;
                    evaluation.registers.insert(result, value);
                } else {
                    fuel.checkpoint().map_err(RuntimeVmError::Execution)?;
                    return Ok(RuntimeVmCommandOutcome {
                        execution: RuntimeVmOutcome {
                            value,
                            output,
                            remaining: fuel.policy_remaining(),
                        },
                        exit_code: command.exit_code,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod command_tests {
    use super::*;
    use crate::compiler::execution::MaterializationLimits;
    #[test]
    fn status_changes_only_after_unit_result_admission() {
        let mut unlimited = Fuel::new(MaterializationLimits::UNLIMITED);
        let mut command = CommandContext {
            arguments: Some(&[]),
            input: None,
            files: None,
            exit_code: 9,
        };
        let code = Value::int(7, &mut unlimited).unwrap();
        let mut fuel = Fuel::new(MaterializationLimits {
            max_value_nodes: Some(0),
            max_text_bytes: Some(0),
        });
        assert_eq!(
            standard(
                "std::set_exit_code",
                vec![code],
                &mut fuel,
                &mut Vec::new(),
                &mut command
            )
            .unwrap_err(),
            RuntimeVmError::Value(RuntimeValueError::NodeLimit)
        );
        assert_eq!(command.exit_code, 9);
        let code = Value::int(126, &mut unlimited).unwrap();
        assert_eq!(
            standard(
                "std::set_exit_code",
                vec![code],
                &mut fuel,
                &mut Vec::new(),
                &mut command
            )
            .unwrap_err(),
            RuntimeVmError::ExitCode
        );
        assert_eq!(command.exit_code, 9);
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;
    use crate::compiler::execution::MaterializationLimits;
    struct FailingInput(usize);
    impl RuntimeVmInput for FailingInput {
        fn read_line(&mut self) -> Result<Option<String>, String> {
            self.0 += 1;
            Err("io".into())
        }
    }
    #[test]
    fn input_minimum_preflight_and_host_error_never_consume_result_fuel() {
        let labels = ("std::InputLine".len() + "eof".len() + "text".len()) as u64;
        for nodes in [2, 3] {
            let limits = MaterializationLimits {
                max_value_nodes: Some(nodes),
                max_text_bytes: Some(labels),
            };
            let mut fuel = Fuel::new(limits);
            let mut host = FailingInput(0);
            let mut command = CommandContext {
                arguments: None,
                input: Some(&mut host),
                files: None,
                exit_code: 0,
            };
            let error = standard(
                "std::read_line",
                vec![],
                &mut fuel,
                &mut Vec::new(),
                &mut command,
            )
            .unwrap_err();
            assert_eq!(fuel.remaining(), limits);
            if nodes == 2 {
                assert_eq!(error, RuntimeVmError::Value(RuntimeValueError::NodeLimit));
            } else {
                assert_eq!(error, RuntimeVmError::Host("io".into()));
            }
            assert_eq!(host.0, usize::from(nodes == 3));
        }
    }
}

#[cfg(test)]
mod file_tests {
    use super::*;
    use crate::compiler::{
        execution::MaterializationLimits,
        text_files::{TextRead, TextWrite},
    };
    struct Files {
        writes: usize,
    }
    impl RuntimeVmFiles for Files {
        fn read_text(&mut self, _: &str) -> TextRead {
            unreachable!()
        }
        fn write_text(&mut self, _: &str, _: &str) -> TextWrite {
            self.writes += 1;
            TextWrite {
                ok: false,
                error: "late".into(),
            }
        }
        fn write_text_if(&mut self, _: &str, _: bool, _: &str, _: &str) -> TextWrite {
            unreachable!()
        }
    }
    #[test]
    fn unknown_write_result_error_can_refuse_after_effect_without_partial_fuel_charge() {
        let labels = ("std::TextWrite".len() + "okerror".len()) as u64;
        let limits = MaterializationLimits {
            max_value_nodes: Some(3),
            max_text_bytes: Some(labels),
        };
        let mut fuel = Fuel::new(limits);
        let mut unlimited = Fuel::new(MaterializationLimits::UNLIMITED);
        let args = vec![
            Value::string("p", &mut unlimited).unwrap(),
            Value::string("new", &mut unlimited).unwrap(),
        ];
        let mut host = Files { writes: 0 };
        let mut context = CommandContext {
            arguments: None,
            input: None,
            files: Some(&mut host),
            exit_code: 0,
        };
        let error = standard(
            "std::write_text",
            args,
            &mut fuel,
            &mut Vec::new(),
            &mut context,
        )
        .unwrap_err();
        assert_eq!(error, RuntimeVmError::Value(RuntimeValueError::TextLimit));
        assert_eq!(fuel.remaining(), limits);
        assert_eq!(host.writes, 1);
    }
}
