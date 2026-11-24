//! Linked Rust execution with callee-owned graphs and shared invocation policy.
use super::*;
/// Per-function compiler quotas plus cumulative retained linked-graph allowances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramLoweringLimits {
    /// Existing logical compiler limits applied to each function.
    pub function: EntryLoweringLimits,
    /// Cumulative structural nodes across functions.
    pub max_mapping_nodes: usize,
    /// Cumulative structural fact links across functions.
    pub max_fact_links: usize,
    /// Cumulative expression and function instructions, both counted.
    pub max_instructions: usize,
    /// Cumulative expression register definitions across functions.
    pub max_values: usize,
}
impl ProgramLoweringLimits {
    /// Finite opt-in profile; runtime instruction units remain a separate allowance.
    pub const DEFAULT: Self = Self {
        function: EntryLoweringLimits::DEFAULT,
        max_mapping_nodes: 1_000_000,
        max_fact_links: 2_000_000,
        max_instructions: 1_000_000,
        max_values: 1_000_000,
    };
}
/// No partially compiled program or fallback execution is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramExecutionError {
    /// Entry identity is not owned by this checked program.
    Entry,
    /// Checked aggregate graph accounting overflowed or exhausted.
    Limit,
    /// Structural mapping refused.
    Mapping(MappingError),
    /// Expression graph emission/validation refused.
    Expression(ExpressionIrError),
    /// Function instruction emission/validation refused.
    Function(FunctionIrError),
    /// Shared invocation execution refused.
    Runtime(RuntimeVmError),
}
impl std::fmt::Display for ProgramExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "program execution refused: {self:?}")
    }
}
impl std::error::Error for ProgramExecutionError {}
impl CheckedProgramSource {
    pub(super) fn with_linked_ir<T>(
        &self,
        entry: ProgramFunctionId,
        limits: ProgramLoweringLimits,
        run: impl FnOnce(&[FunctionIr<'_>], &[ProgramFunctionId], usize) -> Result<T, RuntimeVmError>,
    ) -> Result<T, ProgramExecutionError> {
        self.function(entry).ok_or(ProgramExecutionError::Entry)?;
        let (mut nodes, mut links, mut instructions, mut values) = (
            limits.max_mapping_nodes,
            limits.max_fact_links,
            limits.max_instructions,
            limits.max_values,
        );
        let mut maps = Vec::with_capacity(self.functions().len());
        for f in self.functions() {
            let mut local = limits.function.mapping;
            local.max_nodes = local.max_nodes.min(nodes);
            local.max_fact_links = local.max_fact_links.min(links);
            let map = f
                .map_for_lowering(local)
                .map_err(ProgramExecutionError::Mapping)?;
            nodes = nodes
                .checked_sub(map.nodes().len())
                .ok_or(ProgramExecutionError::Limit)?;
            links = links
                .checked_sub(map.links().len())
                .ok_or(ProgramExecutionError::Limit)?;
            maps.push(map);
        }
        let mut expressions = Vec::with_capacity(maps.len());
        for map in &maps {
            let mut local = limits.function.expressions;
            local.max_instructions = local.max_instructions.min(instructions);
            local.max_values = local.max_values.min(values);
            let graph = map
                .emit_expression_ir(local)
                .map_err(ProgramExecutionError::Expression)?;
            instructions = instructions
                .checked_sub(graph.instructions().len())
                .ok_or(ProgramExecutionError::Limit)?;
            values = values
                .checked_sub(graph.values().len())
                .ok_or(ProgramExecutionError::Limit)?;
            expressions.push(graph);
        }
        let mut functions = Vec::with_capacity(expressions.len());
        for graph in &expressions {
            let mut local = limits.function.function;
            local.max_instructions = local.max_instructions.min(instructions);
            let function = graph
                .emit_function_ir(local)
                .map_err(ProgramExecutionError::Function)?;
            instructions = instructions
                .checked_sub(function.instructions().len())
                .ok_or(ProgramExecutionError::Limit)?;
            functions.push(function);
        }
        let ids: Vec<_> = (0..self.functions().len())
            .map(|index| {
                self.function_id_at(index)
                    .ok_or(ProgramExecutionError::Entry)
            })
            .collect::<Result<_, _>>()?;
        run(&functions, &ids, entry.ordinal()).map_err(ProgramExecutionError::Runtime)
    }
    /// Lower all checked functions and execute the chosen owned entry with shared budgets.
    /// Lowering occurs before the VM elapsed clock; no ambient services are granted.
    pub fn execute(
        &self,
        entry: ProgramFunctionId,
        arguments: &[RuntimeValueV2],
        lowering: ProgramLoweringLimits,
        policy: RuntimeVmPolicy,
    ) -> Result<RuntimeVmOutcome, ProgramExecutionError> {
        self.with_linked_ir(entry, lowering, |functions, ids, index| {
            super::runtime::execute_linked(
                functions,
                ids,
                index,
                arguments,
                None,
                RuntimeVmServices::default(),
                policy,
            )
            .map(|o| o.execution)
        })
    }
    /// Execute with explicit program arguments/input/files, shared across every callee.
    /// All functions are validated/preflighted before effects, including unused bodies.
    /// Completed host effects are not rolled back on a later refusal.
    pub fn execute_with_services<'a>(
        &self,
        entry: ProgramFunctionId,
        arguments: &[RuntimeValueV2],
        program_arguments: &'a [String],
        services: RuntimeVmServices<'a>,
        lowering: ProgramLoweringLimits,
        policy: RuntimeVmPolicy,
    ) -> Result<RuntimeVmCommandOutcome, ProgramExecutionError> {
        self.with_linked_ir(entry, lowering, |functions, ids, index| {
            super::runtime::execute_linked(
                functions,
                ids,
                index,
                arguments,
                Some(program_arguments),
                services,
                policy,
            )
        })
    }
}
