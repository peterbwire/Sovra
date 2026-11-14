//! Deterministic bounded inspection text over sealed linked IR; not an executable image.
use super::*;
use std::fmt::{self, Write};
/// Independent output admission, including escaping and metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramIrLimits {
    /// Maximum complete UTF-8 inspection bytes; no partial artifact is returned.
    pub max_bytes: usize,
}
impl ProgramIrLimits {
    /// Finite opt-in inspection profile.
    pub const DEFAULT: Self = Self {
        max_bytes: 4 * 1024 * 1024,
    };
}
/// Compilation or complete-artifact refusal; never a backend fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramIrError {
    /// Existing linked lowering failed before inspection.
    Lowering(ProgramExecutionError),
    /// Formatted output exceeded its allowance or overflowed.
    Limit,
    /// Sealed ownership/table agreement was unavailable.
    Invalid(&'static str),
}
impl fmt::Display for ProgramIrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "versioned IR inspection refused: {self:?}")
    }
}
impl std::error::Error for ProgramIrError {}
impl From<fmt::Error> for ProgramIrError {
    fn from(_: fmt::Error) -> Self {
        Self::Limit
    }
}
struct Output {
    text: String,
    max: usize,
}
impl Write for Output {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let next = self.text.len().checked_add(text.len()).ok_or(fmt::Error)?;
        if next > self.max {
            return Err(fmt::Error);
        }
        self.text.push_str(text);
        Ok(())
    }
}
fn enum_index(env: &CheckedEnumTypes, id: EnumId) -> Result<usize, ProgramIrError> {
    env.declarations()
        .get(id.ordinal())
        .filter(|d| d.id == id)
        .map(|_| id.ordinal())
        .ok_or(ProgramIrError::Invalid("enum owner"))
}
fn span(
    ir: &ExpressionIr<'_>,
    id: MappedNodeId,
) -> Result<crate::compiler::diagnostics::Span, ProgramIrError> {
    ir.mapping()
        .nodes()
        .get(id.ordinal())
        .filter(|node| node.id == id)
        .map(|node| node.syntax.span())
        .ok_or(ProgramIrError::Invalid("source node owner"))
}
fn types(out: &mut Output, env: &CheckedEnumTypes) -> Result<(), ProgramIrError> {
    for (index, ty) in env.types().iter().enumerate() {
        write!(out, "type t{index} = ")?;
        match ty {
            EnumPayloadType::Enum(id) => write!(out, "enum e{}", enum_index(env, *id)?)?,
            EnumPayloadType::Array(id) => {
                env.payload_type(*id)
                    .ok_or(ProgramIrError::Invalid("payload type owner"))?;
                write!(out, "array t{}", id.ordinal())?;
            }
            _ => write!(out, "{ty:?}")?,
        }
        writeln!(out)?;
    }
    for declaration in env.declarations() {
        writeln!(
            out,
            "enum e{} {:?} exported={}",
            declaration.id.ordinal(),
            declaration.name,
            declaration.is_exported
        )?;
        for (index, variant) in declaration.variants.iter().enumerate() {
            write!(out, "  variant {index} {:?} ", variant.name)?;
            match variant.payload {
                Some(id) => {
                    env.payload_type(id)
                        .ok_or(ProgramIrError::Invalid("variant payload owner"))?;
                    writeln!(out, "payload t{}", id.ordinal())?;
                }
                None => writeln!(out, "nullary")?,
            }
        }
    }
    // Maps are sorted explicitly; no hash iteration order becomes artifact identity.
    let mut records: Vec<_> = env.body_record_fields().iter().collect();
    records.sort_by(|a, b| a.0.cmp(b.0));
    for (name, fields) in records {
        writeln!(out, "record {name:?}")?;
        let mut fields: Vec<_> = fields.iter().collect();
        fields.sort_by(|a, b| a.0.cmp(b.0));
        for (name, ty) in fields {
            writeln!(out, "  field {name:?} : {ty:?}")?;
        }
    }
    Ok(())
}
fn values(out: &mut Output, operands: &[ExpressionValueId]) -> fmt::Result {
    write!(out, "[")?;
    for (i, value) in operands.iter().enumerate() {
        if i > 0 {
            write!(out, ", ")?;
        }
        write!(out, "v{}", value.ordinal())?;
    }
    write!(out, "]")
}
fn operation(
    out: &mut Output,
    op: &ExpressionOperation<'_>,
    env: &CheckedEnumTypes,
    functions: &[ProgramFunctionId],
    current: usize,
) -> Result<(), ProgramIrError> {
    use ExpressionOperation::*;
    match op {
        Literal(literal) => write!(out, "literal {literal:?}")?,
        Read { usage, declaration } => {
            write!(out, "read u{} d{}", usage.ordinal(), declaration.ordinal())?
        }
        Binary {
            operator,
            left,
            right,
        } => write!(
            out,
            "binary {operator:?} v{} v{}",
            left.ordinal(),
            right.ordinal()
        )?,
        Call {
            target, arguments, ..
        } => {
            write!(out, "call ")?;
            match target {
                CallableTarget::Standard(name) => write!(out, "standard {name:?}")?,
                CallableTarget::SelfFunction => write!(out, "function f{current}")?,
                CallableTarget::ProgramFunction(id) => {
                    if functions.get(id.ordinal()) != Some(id) {
                        return Err(ProgramIrError::Invalid("function owner"));
                    }
                    write!(out, "function f{}", id.ordinal())?;
                }
                CallableTarget::Constructor { enum_id, variant } => write!(
                    out,
                    "constructor e{} variant {variant}",
                    enum_index(env, *enum_id)?
                )?,
            }
            write!(out, " ")?;
            values(out, arguments)?;
        }
        Array(items) => {
            write!(out, "array ")?;
            values(out, items)?;
        }
        Record(fields) => {
            write!(out, "record [")?;
            for (i, (name, value)) in fields.iter().enumerate() {
                if i > 0 {
                    write!(out, ", ")?;
                }
                write!(out, "{name:?}: v{}", value.ordinal())?;
            }
            write!(out, "]")?;
        }
        Field { receiver, field } => write!(out, "field v{} {field:?}", receiver.ordinal())?,
        Index { target, index } => write!(out, "index v{} v{}", target.ordinal(), index.ordinal())?,
        Convert { value, context } => write!(out, "convert v{} c{context}", value.ordinal())?,
    }
    Ok(())
}
fn operand(out: &mut Output, value: FunctionOperand) -> fmt::Result {
    match value {
        FunctionOperand::Root(i) => write!(out, "r{i}"),
        FunctionOperand::Converted(i) => write!(out, "c{i}"),
    }
}
fn instruction(
    out: &mut Output,
    op: &FunctionInstruction,
    env: &CheckedEnumTypes,
) -> Result<(), ProgramIrError> {
    use FunctionInstruction::*;
    match op {
        EnterScope(i) => write!(out, "enter s{i}")?,
        ExitScope(i) => write!(out, "exit s{i}")?,
        Parameter { index, declaration } => {
            write!(out, "parameter {index} d{}", declaration.ordinal())?
        }
        Evaluate { root } => write!(out, "evaluate r{root}")?,
        Convert { context, root } => write!(out, "convert r{root} c{context}")?,
        Bind { declaration, value } | Store { declaration, value } => {
            write!(
                out,
                "{} d{} ",
                if matches!(op, Bind { .. }) {
                    "bind"
                } else {
                    "store"
                },
                declaration.ordinal()
            )?;
            operand(out, *value)?;
        }
        StoreIndex {
            declaration,
            array,
            index,
            value,
        } => {
            write!(
                out,
                "store_index d{} r{array} r{index} ",
                declaration.ordinal()
            )?;
            operand(out, *value)?;
        }
        Release { root } => write!(out, "release r{root}")?,
        SnapshotMatch { index, declaration } => {
            write!(out, "snapshot m{index} d{}", declaration.ordinal())?
        }
        Switch {
            index,
            enum_id,
            arms,
        } => {
            write!(out, "switch m{index} e{} [", enum_index(env, *enum_id)?)?;
            for (i, (variant, target)) in arms.iter().enumerate() {
                if i > 0 {
                    write!(out, ", ")?;
                }
                write!(out, "variant {variant} -> pc{target}")?;
            }
            write!(out, "]")?;
        }
        Payload {
            index,
            variant,
            declaration,
        } => write!(
            out,
            "payload m{index} variant {variant} d{}",
            declaration.ordinal()
        )?,
        ReleaseMatch(i) => write!(out, "release_match m{i}")?,
        BranchFalse { condition, target } => write!(out, "branch_false r{condition} pc{target}")?,
        Jump(i) => write!(out, "jump pc{i}")?,
        Return(value) => {
            write!(out, "return ")?;
            match value {
                Some(value) => operand(out, *value)?,
                None => write!(out, "Unit")?,
            }
        }
        End => write!(out, "end")?,
    }
    Ok(())
}
fn render(
    functions: &[FunctionIr<'_>],
    ids: &[ProgramFunctionId],
    limits: ProgramIrLimits,
) -> Result<String, ProgramIrError> {
    let mut out = Output {
        text: String::new(),
        max: limits.max_bytes,
    };
    writeln!(out, "sovra-ir v2 inspection-only")?;
    writeln!(out, "materialization-model {VALUE_MODEL_V2}")?;
    let first = functions
        .first()
        .ok_or(ProgramIrError::Invalid("empty program"))?;
    types(
        &mut out,
        first
            .expressions()
            .mapping()
            .checked()
            .typed()
            .callable()
            .function()
            .environment(),
    )?;
    for (index, function) in functions.iter().enumerate() {
        let expressions = function.expressions();
        let checked = expressions.mapping().checked();
        let source = checked.typed().callable().function();
        let header = source.syntax();
        let env = source.environment();
        writeln!(
            out,
            "function f{index} {:?} return {:?} span {}..{}",
            source
                .program_target_name(ids[index])
                .ok_or(ProgramIrError::Invalid("function identity"))?,
            function.return_type(),
            header.span.start,
            header.span.end
        )?;
        for binding in checked.bindings().declarations() {
            writeln!(
                out,
                "  binding d{} {:?} {:?} : {:?} span {}..{}",
                binding.id.ordinal(),
                binding.name,
                binding.kind,
                binding.ty,
                binding.span.start,
                binding.span.end
            )?;
        }
        for usage in checked.bindings().uses() {
            writeln!(
                out,
                "  use u{} d{} {:?} span {}..{}",
                usage.id.ordinal(),
                usage.declaration.ordinal(),
                usage.kind,
                usage.span.start,
                usage.span.end
            )?;
        }
        for (index, context) in checked
            .typed()
            .callable()
            .evidence()
            .contexts()
            .iter()
            .enumerate()
        {
            writeln!(
                out,
                "  context c{index} {:?} : {:?} -> {:?} span {}..{}",
                context.kind,
                context.actual,
                context.expected,
                context.value_span.start,
                context.value_span.end
            )?;
        }
        for op in expressions.instructions() {
            let value = expressions
                .value(op.result)
                .ok_or(ProgramIrError::Invalid("expression owner"))?;
            let span = span(expressions, value.source)?;
            write!(
                out,
                "  value v{} : {:?} span {}..{} = ",
                op.result.ordinal(),
                value.ty,
                span.start,
                span.end
            )?;
            operation(&mut out, &op.operation, env, ids, index)?;
            writeln!(out)?;
        }
        for (i, root) in expressions.roots().iter().enumerate() {
            let span = span(expressions, root.source)?;
            writeln!(
                out,
                "  root r{i} = v{} span {}..{}",
                root.result.ordinal(),
                span.start,
                span.end
            )?;
        }
        for (i, op) in function.instructions().iter().enumerate() {
            write!(out, "  pc{i} ")?;
            instruction(&mut out, op, env)?;
            writeln!(out)?;
        }
        writeln!(out, "end_function")?;
    }
    Ok(out.text)
}
impl CheckedProgramSource {
    /// Lower and validate the complete program, then return bounded deterministic
    /// inspection text. No main, VM execution or hosts are required. Source-order
    /// local IDs replace process-global ownership nonces. This is not an image codec.
    pub fn render_ir(
        &self,
        lowering: ProgramLoweringLimits,
        limits: ProgramIrLimits,
    ) -> Result<String, ProgramIrError> {
        let entry = self
            .function_id_at(0)
            .ok_or(ProgramIrError::Invalid("empty program"))?;
        self.with_linked_ir(entry, lowering, |functions, ids, _| {
            Ok(render(functions, ids, limits))
        })
        .map_err(ProgramIrError::Lowering)?
    }
}
