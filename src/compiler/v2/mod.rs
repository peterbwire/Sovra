//! Versioned extensions beside unchanged v1 AST, checked stages and runtime APIs.
//! Additive enum/match checking, source-owned facts, IR and opt-in Rust execution.
//! A bounded root source-unit API now derives types and one entry from the same
//! original tokens. Multi-function/module/package/tooling and backend integration
//! remain separate; existing v1 stages and default CLI behavior are unchanged.
mod enum_types;
pub use enum_types::{
    check_enum_types, check_scoped_enum_types, CheckedEnumTypes, EnumConstructorArgument,
    EnumConstructorPlan, EnumDeclarationFact, EnumId, EnumMatchArm, EnumMatchPlan,
    EnumPayloadCoercion, EnumPayloadType, EnumSource, EnumTypeDiagnostic, EnumTypeError,
    EnumTypeId, EnumTypeLimit, EnumTypeLimits, EnumVariantFact, ScopedEnumSource,
};
mod match_syntax;
pub use match_syntax::{
    parse_match_statement, MatchArmSyntax, MatchStatementSyntax, MatchSyntaxLimits,
};
mod match_bodies;
pub use match_bodies::{CheckedMatchBodies, MatchBodyError, MatchBodyLimits, MatchOuterBinding};
mod function;
pub use function::{
    CheckedEnumFunction, EnumFunctionError, EnumFunctionLimits, EnumFunctionStatement,
    EnumFunctionSyntax,
};
pub(crate) mod callable;
pub use callable::{
    CallableCall, CallableContext, CallableEvidence, CallableLimits, CallableTarget,
    CheckedCallableFunction,
};

mod typed_callable;
pub use typed_callable::{
    CheckedTypedCallableFunction, ExpressionEvidenceLimits, TypedCallLink, TypedCallableEvidence,
    TypedCallableLimits, TypedContextLink, TypedExpressionFact, TypedExpressionId,
};
mod scoped_callable;
pub use scoped_callable::{
    BindingEvidenceLimits, CheckedScopedCallableFunction, ScopedCallableEvidence,
    ScopedCallableLimits, ScopedDeclaration, ScopedDeclarationId, ScopedDeclarationKind, ScopedUse,
    ScopedUseId, ScopedUseKind,
};
mod mapping;
pub use mapping::{
    MappedCallable, MappedExpressionRole, MappedFact, MappedFactLink, MappedNode, MappedNodeId,
    MappedSyntax, MappingError, MappingLimits,
};
mod expression_ir;
pub use expression_ir::{
    ExpressionInstruction, ExpressionIr, ExpressionIrError, ExpressionIrLimits, ExpressionLiteral,
    ExpressionOperation, ExpressionRoot, ExpressionValue, ExpressionValueId,
};
mod function_ir;
pub use function_ir::{
    FunctionInstruction, FunctionIr, FunctionIrError, FunctionIrLimits, FunctionOperand,
    FunctionValidation,
};
mod runtime_values;
pub use runtime_values::{RuntimeValueError, RuntimeValueFuel, RuntimeValueV2, VALUE_MODEL_V2};
mod runtime;
pub use runtime::{
    RuntimeVmCommandOutcome, RuntimeVmError, RuntimeVmFiles, RuntimeVmInput, RuntimeVmOutcome,
    RuntimeVmPolicy, RuntimeVmServices,
};
mod entry_source;
pub use entry_source::{
    check_entry_source, CheckedEntrySource, EntryDeclaration, EntryDeclarationKind,
    EntryExecutionError, EntryLoweringLimits, EntrySourceError, EntrySourceLimits,
};
mod program_source;
pub use program_source::{
    check_program_source, CheckedProgramSource, ProgramCallLink, ProgramFunctionId,
    ProgramSourceError, ProgramSourceLimits,
};
mod program_execution;
pub use program_execution::{ProgramExecutionError, ProgramLoweringLimits};
mod recursive_match;
pub use recursive_match::{RecursiveControlSyntax, RecursiveMatchSyntax};
mod program_diagnostics;
mod recursive_check;
pub use program_diagnostics::PROGRAM_DIAGNOSTIC_LIMIT;

mod program_testing;
pub use program_testing::ProgramTestLimits;

mod program_ir;
pub use program_ir::{ProgramIrError, ProgramIrLimits};

pub use entry_source::{parse_program_syntax, ParsedProgramSyntax};

mod native_image;
pub use native_image::{
    command_runtime_policy, decode_native_image, execute_native_image, is_native_image,
    NATIVE_SOURCE_LIMIT,
};

mod program_symbols;
pub use program_symbols::ProgramSymbolLimits;
