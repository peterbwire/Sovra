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
    EnumTypeId, EnumTypeLimit, EnumTypeLimits, EnumVariantFact, RuntimeRecordId, ScopedEnumSource,
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
mod runtime_maps;
pub use runtime_maps::{
    RuntimeMapEntry, RuntimeMapError, RuntimeMapPrimitive, RuntimeMapRemoval, RuntimeMapV2,
    RuntimeMapValueType,
};
mod runtime_bytes;
pub use runtime_bytes::{
    RuntimeByteV2, RuntimeBytesLimits, RuntimeBytesV2, RuntimeDataError, RuntimeDataErrorKind,
    RuntimeDataWork,
};
mod runtime_sets;
pub use runtime_sets::{RuntimeKey, RuntimeKeyType, RuntimeSetError, RuntimeSetV2, RuntimeSetWork};
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

mod generic_types;
pub use generic_types::{
    parse_generic_type, GenericTypeError, GenericTypeKind, GenericTypeLimits, GenericTypeNode,
    GenericTypeSyntax,
};

mod generic_parameters;
pub use generic_parameters::{
    parse_generic_parameters, GenericParameter, GenericParameterBound, GenericParameterId,
    GenericParameterLimits, GenericParameterUse, GenericParameterUses, GenericParameters,
};

mod generic_aliases;
pub use generic_aliases::{parse_generic_alias, GenericAliasLimits, GenericAliasSyntax};

mod generic_records;
pub use generic_records::{
    parse_generic_record, GenericRecordField, GenericRecordLimits, GenericRecordSyntax,
};

mod generic_enums;
pub use generic_enums::{
    parse_generic_enum, GenericEnumLimits, GenericEnumPayload, GenericEnumSyntax,
    GenericEnumVariant, GenericStandardEnum,
};

mod generic_functions;
pub use generic_functions::{
    parse_generic_function, parse_generic_propagation_function, parse_specialization_root,
    GenericFunctionAnnotation, GenericFunctionLimits, GenericFunctionParameter,
    GenericFunctionSyntax,
};

mod generic_resolution;
pub use generic_resolution::{
    register_generic_types, GenericPrimitive, GenericRegistryLimits, GenericResolutionError,
    GenericResolutionLimits, GenericTypeDeclaration, GenericTypeDeclarationId,
    GenericTypeDeclarationKind, GenericTypeHeader, GenericTypeRegistry, ResolvedGenericType,
    ResolvedGenericTypeKind,
};

mod generic_alias_expansion;
pub use generic_alias_expansion::{
    prepare_generic_aliases, ExpandedGenericType, ExpandedGenericTypeNode, GenericAliasEnvironment,
    GenericAliasEnvironmentLimits, GenericAliasExpansionError, GenericAliasExpansionLimits,
    GenericExpansionOrigin,
};

mod generic_declarations;
pub use generic_declarations::{
    check_generic_declarations, CheckedGenericAlias, CheckedGenericDeclarations,
    CheckedGenericDefinition, CheckedGenericEnum, CheckedGenericRecord, GenericDeclarationError,
    GenericDeclarationLimits,
};

mod generic_signatures;
pub use generic_signatures::{
    check_generic_signatures, CheckedGenericSignature, CheckedGenericSignatures,
    GenericSignatureError, GenericSignatureId, GenericSignatureLimits, GenericSignaturePosition,
    GenericSignatureReturn,
};

mod generic_type_equivalence;
pub use generic_type_equivalence::{
    compare_generic_types, generic_type_key, GenericTypeComparison, GenericTypeEquivalenceError,
    GenericTypeEquivalenceLimits, GenericTypeKey, GenericTypeKeyLimits, GenericTypeKeyResult,
};

pub(crate) mod generic_copy_bodies;
pub use generic_copy_bodies::{
    check_generic_copy_bodies, check_generic_propagation_bodies, CheckedGenericCopyBodies,
    CheckedGenericCopyBody, GenericCopyBinding, GenericCopyBindingId, GenericCopyBindingKind,
    GenericCopyBodyError, GenericCopyBodyLimits, GenericCopyConversion, GenericCopyExpression,
    GenericCopyExpressionId, GenericCopyExpressionKind, GenericCopyMatchArm, GenericCopyNameUse,
    GenericCopyStatement, GenericCopyType,
};

// Explicit local annotation profiles extend the symbolic copy body stage.
pub use generic_copy_bodies::{
    GenericCopyLocalAnnotation, GenericLocalAnnotationError, GenericLocalAnnotationLimits,
    GenericLocalAnnotationUsage,
};

pub use generic_copy_bodies::GenericNumericOperator;

pub use generic_copy_bodies::GenericLogicalOperator;

pub use generic_copy_bodies::GenericStringOperator;

pub use generic_copy_bodies::GenericCopyArrayElement;

pub use generic_copy_bodies::GenericFieldSubstitutionLimits;

pub use generic_types::{
    parse_generic_type_arguments, GenericTypeArgumentLimits, GenericTypeArgumentsSyntax,
};

mod generic_argument_checking;
pub use generic_argument_checking::{
    check_generic_type_arguments, CheckedGenericTypeArguments, GenericArgumentCheckError,
    GenericArgumentCheckLimits,
};

mod generic_call_binding;
pub use generic_call_binding::{
    bind_generic_type_arguments, CheckedGenericCallBinding, GenericCallBindingError,
    GenericCallBindingLimits, GenericCallTypeBinding,
};

mod generic_signature_instantiation;
pub use generic_signature_instantiation::{
    instantiate_generic_signature, GenericInstantiationPosition,
    GenericSignatureInstantiationError, GenericSignatureInstantiationLimits,
    InstantiatedGenericSignature,
};

mod generic_call_values;
pub use generic_call_values::{
    check_generic_call_values, CheckedGenericCallValues, GenericCallValueArgument,
    GenericCallValueError, GenericCallValueLimits,
};

mod generic_call_syntax;
pub use generic_call_syntax::{parse_generic_call, GenericCallSyntax, GenericCallSyntaxLimits};

mod generic_call_expressions;
pub use generic_call_expressions::{
    parse_generic_call_expressions, GenericCallExpressionError, GenericCallExpressionLimits,
    ParsedGenericCallExpressions,
};

pub use generic_copy_bodies::{GenericBodyCallError, GenericBodyCallLimits, GenericBodyCallUsage};

pub use generic_copy_bodies::GenericRecordFieldValue;

pub use generic_copy_bodies::GenericEnumPayloadValue;

mod generic_enum_patterns;
pub use generic_enum_patterns::{
    check_generic_enum_patterns, CheckedGenericEnumPatterns, GenericEnumPatternArm,
    GenericEnumPatternError, GenericEnumPatternLimits, GenericEnumPatternPayload,
};

// Bounded concrete call closure; no executable specialization is authorized.
mod generic_specialization;
pub use generic_specialization::{
    discover_generic_propagation_specializations, discover_generic_specializations,
    CheckedGenericSpecializations, GenericSpecializationCall, GenericSpecializationError,
    GenericSpecializationInstance, GenericSpecializationLimits, GenericSpecializationUsage,
};

// Concrete operation/context consistency before executable lowering.
mod generic_concrete_bodies;
pub use generic_concrete_bodies::{
    recheck_generic_concrete_bodies, recheck_generic_propagation_concrete_bodies,
    CheckedConcreteGenericBodies, ConcreteGenericBody, ConcreteGenericPropagation,
    GenericConcreteBodyError, GenericConcreteBodyLimits, GenericConcreteBodyUsage,
};

// Source-backed primitive payloads for future executable generic lowering.
mod generic_literals;
pub use generic_literals::{
    decode_generic_literals, CheckedGenericLiterals, GenericDecodedLiteral, GenericLiteralBody,
    GenericLiteralError, GenericLiteralLimits, GenericLiteralUsage, GenericLiteralValue,
};

// Backend-neutral logical type layouts, before executable generic lowering.
mod generic_layouts;
pub use generic_layouts::{
    plan_generic_collection_layouts, plan_generic_data_layouts, plan_generic_layouts,
    CheckedGenericLayouts, GenericInstanceLayouts, GenericLayoutError, GenericLayoutLimits,
    GenericLayoutUsage, GenericRuntimeLayout, GenericRuntimeLayoutKind,
};

// Nominal operation/dispatch plans before executable generic instruction lowering.
mod generic_construction;
pub use generic_construction::{
    plan_generic_collection_construction, plan_generic_construction,
    plan_generic_data_construction, CheckedGenericConstruction, GenericConstructionBody,
    GenericConstructionError, GenericConstructionExpression, GenericConstructionKind,
    GenericConstructionLimits, GenericConstructionUsage, GenericFieldInitialization,
    GenericMatchArm, GenericMatchDispatch, GenericMatchPayload, GenericPlannedOperand,
};

// Typed generic instruction emission and finite control-flow validation.
mod generic_ir;
pub use generic_ir::{
    lower_generic_collection_propagation_ir, lower_generic_data_propagation_ir, lower_generic_ir,
    lower_generic_propagation_ir, CheckedGenericIr, CheckedGenericPropagationIr, GenericIrBinary,
    GenericIrError, GenericIrFunction, GenericIrInstruction, GenericIrLimits, GenericIrOperation,
    GenericIrUsage,
};

// Host-free iterative execution of checked generic IR.
mod generic_runtime;
pub use generic_runtime::{
    execute_generic_collection_ir, execute_generic_data_ir, execute_generic_ir,
    execute_generic_propagation_ir, GenericCollectionError, GenericCollectionMap,
    GenericCollectionMapEntry, GenericCollectionMapError, GenericCollectionMapRemoval,
    GenericCollectionSelector, GenericCollectionSnapshot, GenericCollectionType, GenericValueRef,
    GenericVmError, GenericVmErrorKind, GenericVmOutcome, GenericVmPolicy,
};

mod generic_data_types;
pub use generic_data_types::{
    standard_generic_data_types, CheckedGenericData, GenericDataSyntax, GenericDataTypeKind,
};
mod generic_collection_types;
pub use generic_collection_types::{
    standard_generic_collection_types, CheckedGenericCollection, GenericCollectionSyntax,
};

mod generic_collection_layouts;
pub use generic_collection_layouts::{
    GenericCollectionLayout, GenericCollectionLayoutError, GenericCollectionLayoutKind,
};

// Bounded complete standalone source collection, compilation and host-free execution.
mod generic_source;
pub(crate) use generic_source::collect_package_fragment;
pub use generic_source::{
    compile_generic_adapter_source, compile_generic_array_source, compile_generic_builtin_source,
    compile_generic_propagation_source, compile_generic_source, compile_generic_string_source,
    compile_generic_value_source, execute_generic_source, test_generic_source,
    CompiledGenericSource, GenericPropagationSourceLimits, GenericSourceDeclaration,
    GenericSourceDeclarationKind, GenericSourceError, GenericSourceErrorKind,
    GenericSourceExecutionError, GenericSourceLimits, GenericSourceOutcome,
    GenericSourceSyntaxLimits, GenericSourceUsage, GenericTestLimits,
};

// Explicit private generic source-image family for host-native linking.
mod generic_native_image;
pub use generic_native_image::{
    decode_generic_native_image, execute_generic_native_image, generic_command_runtime_policy,
    is_generic_native_image, GENERIC_NATIVE_SOURCE_LIMIT,
};

// Sealed symbolic propagation admission; postfix syntax/execution remains separate.
mod generic_propagation;
pub use generic_propagation::{
    plan_generic_propagations, CheckedGenericPropagations, GenericPropagationError,
    GenericPropagationLimits, GenericPropagationPlan, GenericPropagationRequest,
    GenericPropagationTypes, GenericPropagationUsage,
};

// Original-source postfix syntax metadata; executable pipelines remain strict.
mod generic_propagation_syntax;
pub use generic_propagation_syntax::{
    parse_generic_propagation_block, GenericPropagationSyntax, GenericPropagationSyntaxError,
    GenericPropagationSyntaxLimits, GenericPropagationSyntaxSite,
};

mod generic_propagation_layouts;
pub use generic_propagation_layouts::{
    plan_generic_propagation_layouts, CheckedGenericPropagationLayouts,
    GenericPropagationLayoutError, GenericPropagationLayoutLimits, GenericPropagationLayoutPlan,
};

/// Explicit reusable Sovra functions for adapting existing standard result records.
/// Include these bytes in a complete source passed to `compile_generic_adapter_source`.
pub const STANDARD_RESULT_ADAPTER_SOURCE: &str = include_str!("standard_result_adapters.svr");

/// Explicit host-free operations; host I/O remains outside this pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericBuiltin {
    /// Checked Int-to-Byte conversion returning Result<Byte,Error>.
    ByteFromInt,
    /// Explicit owned Byte-to-Int conversion.
    ByteToInt,
    /// Copy checked Byte array into an immutable buffer.
    BytesFromArray,
    /// Encode String as its unmodified UTF-8 bytes.
    BytesFromText,
    /// Buffer length in bytes.
    BytesLen,
    /// Bounds-safe immutable buffer access.
    BytesGet,
    /// Concatenate independent immutable buffer snapshots.
    BytesConcat,
    /// Copy a checked half-open byte range into an independent Result buffer.
    BytesSlice,
    /// Copy an immutable buffer into a checked Byte array Result.
    BytesToArray,
    /// Lowercase hexadecimal encoding with owned Result admission.
    BytesToHex,
    /// Strict hexadecimal decoding with original input byte offsets.
    BytesFromHex,
    /// Standard padded Base64 encoding.
    BytesToBase64,
    /// Strict standard Base64 decoding with original input offsets.
    BytesFromBase64,
    /// Strict UTF-8 decoding without replacement or normalization.
    BytesToUtf8,
    /// Strict JSON parsing into the explicit recursive standard value.
    JsonParse,
    /// JSON serialization preserving numeric kinds and object order.
    JsonStringify,
    /// Explicit version-three host request; authority comes only from the embedding host.
    HostCall,
    /// Empty map inferred from an explicit collection result context.
    MapNew,
    /// Insert/replace an exact key/value snapshot without changing the input map.
    MapInsert,
    /// Test an exact key without copying its associated value.
    MapContains,
    /// Copy a present map value into the standard Option wrapper.
    MapGet,
    /// Remove a present key, returning Some(updated map), or None on a miss.
    MapRemove,
    /// Remove a present key, returning Some(updated set), or None on a miss.
    SetRemove,
    /// Copy a present set key into the standard Option wrapper.
    SetGet,
    /// Test membership with metered primitive-key comparisons.
    SetContains,
    /// Insert an exact key snapshot without changing the input set.
    SetInsert,
    /// Empty set inferred from an explicit collection result context.
    SetNew,
    /// Length of a checked map snapshot.
    MapLen,
    /// Snapshot map keys in insertion order.
    MapKeys,
    /// Snapshot map values in insertion order.
    MapValues,
    /// Snapshot public checked key/value entry records in insertion order.
    MapEntries,
    /// Snapshot set keys in insertion order.
    SetValues,
    /// Length of a checked set snapshot.
    SetLen,
    /// Existing strict signed decimal parsing into std::ParsedInt.
    ParseInt,
    /// Existing first-delimiter split into std::SplitOnce.
    SplitOnce,
    /// Existing UTF-8 byte length.
    StringLen,
    /// Existing exact LF-delimited line uniqueness.
    LinesUnique,
    /// Existing display-safe terminal control escaping.
    EscapeTerminal,
    /// Versioned display formatting of any checked value.
    ToString,
    /// Existing checked array length.
    ArrayLen,
    /// Append an owned element snapshot.
    ArrayAppend,
    /// Concatenate owned array snapshots.
    ArrayConcat,
    /// Copy a half-open array interval.
    ArraySlice,
}
impl GenericBuiltin {
    pub(super) fn collection_kind(self) -> Option<GenericCollectionLayoutKind> {
        match self {
            Self::MapNew
            | Self::MapLen
            | Self::MapInsert
            | Self::MapContains
            | Self::MapGet
            | Self::MapRemove
            | Self::MapKeys
            | Self::MapValues
            | Self::MapEntries => Some(GenericCollectionLayoutKind::Map),
            Self::SetNew
            | Self::SetLen
            | Self::SetInsert
            | Self::SetContains
            | Self::SetGet
            | Self::SetRemove
            | Self::SetValues => Some(GenericCollectionLayoutKind::Set),
            _ => None,
        }
    }
    pub(super) fn collection_iteration(self) -> bool {
        matches!(
            self,
            Self::MapKeys | Self::MapValues | Self::MapEntries | Self::SetValues
        )
    }
    pub(super) fn collection_remove(self) -> bool {
        matches!(self, Self::MapRemove | Self::SetRemove)
    }
    pub(super) fn collection_get(self) -> bool {
        matches!(self, Self::MapGet | Self::SetGet)
    }
    pub(super) fn collection_contains(self) -> bool {
        matches!(self, Self::MapContains | Self::SetContains)
    }
    pub(super) fn collection_insert(self) -> bool {
        matches!(self, Self::MapInsert | Self::SetInsert)
    }
    pub(super) fn collection_new(self) -> bool {
        matches!(self, Self::MapNew | Self::SetNew)
    }

    pub(super) fn array_operation(self) -> bool {
        matches!(
            self,
            Self::ArrayAppend | Self::ArrayConcat | Self::ArraySlice
        )
    }
    pub(super) fn arity(self) -> usize {
        match self {
            Self::ByteFromInt
            | Self::ByteToInt
            | Self::BytesFromArray
            | Self::BytesFromText
            | Self::BytesLen
            | Self::BytesToArray
            | Self::BytesToHex
            | Self::BytesFromHex
            | Self::BytesToBase64
            | Self::BytesFromBase64
            | Self::BytesToUtf8
            | Self::JsonParse
            | Self::JsonStringify
            | Self::HostCall => 1,
            Self::BytesGet | Self::BytesConcat => 2,
            Self::MapNew | Self::SetNew => 0,
            Self::MapInsert | Self::BytesSlice => 3,
            Self::SetInsert
            | Self::MapContains
            | Self::SetContains
            | Self::MapGet
            | Self::SetGet
            | Self::MapRemove
            | Self::SetRemove => 2,
            Self::MapLen
            | Self::SetLen
            | Self::MapKeys
            | Self::MapValues
            | Self::MapEntries
            | Self::SetValues => 1,
            Self::ParseInt
            | Self::StringLen
            | Self::LinesUnique
            | Self::EscapeTerminal
            | Self::ToString
            | Self::ArrayLen => 1,
            Self::SplitOnce | Self::ArrayAppend | Self::ArrayConcat => 2,
            Self::ArraySlice => 3,
        }
    }
    pub(super) fn result_primitive(self) -> Option<GenericPrimitive> {
        match self {
            Self::ByteToInt
            | Self::BytesLen
            | Self::MapLen
            | Self::SetLen
            | Self::StringLen
            | Self::ArrayLen => Some(GenericPrimitive::Int),
            Self::LinesUnique | Self::MapContains | Self::SetContains => {
                Some(GenericPrimitive::Bool)
            }
            Self::EscapeTerminal | Self::ToString => Some(GenericPrimitive::String),
            _ => None,
        }
    }
    pub(super) fn result_name(self) -> &'static str {
        match self {
            Self::MapNew | Self::MapInsert => "std::Map",
            Self::SetNew | Self::SetInsert => "std::Set",
            Self::MapGet | Self::SetGet | Self::MapRemove | Self::SetRemove => "std::Option",
            Self::ByteFromInt => "std::Result",
            Self::ByteToInt | Self::BytesLen => "Int",
            Self::BytesGet
            | Self::BytesConcat
            | Self::BytesFromArray
            | Self::BytesFromText
            | Self::BytesSlice
            | Self::BytesToArray
            | Self::BytesToHex
            | Self::BytesFromHex
            | Self::BytesToBase64
            | Self::BytesFromBase64
            | Self::BytesToUtf8
            | Self::JsonParse
            | Self::JsonStringify
            | Self::HostCall => "std::Result",
            Self::ParseInt => "std::ParsedInt",
            Self::SplitOnce => "std::SplitOnce",
            Self::MapLen | Self::SetLen | Self::StringLen | Self::ArrayLen => "Int",
            Self::LinesUnique | Self::MapContains | Self::SetContains => "Bool",
            Self::EscapeTerminal | Self::ToString => "String",
            Self::ArrayAppend
            | Self::ArrayConcat
            | Self::ArraySlice
            | Self::MapKeys
            | Self::MapValues
            | Self::SetValues
            | Self::MapEntries => "Array",
        }
    }
}

mod generic_symbols;
pub use generic_symbols::{generic_source_symbols, GenericSymbolLimits};

mod generic_package;
pub use generic_package::{
    bind_generic_package_declarations, GenericPackageDeclaration, GenericPackageDeclarationId,
    GenericPackageDeclarationLimits, GenericPackageDeclarations, GenericPackageNamespace,
    GenericPackageSourceId,
};

mod generic_package_types;
pub use generic_package_types::{
    check_generic_collection_package_types, check_generic_data_package_types,
    check_generic_package_types, CheckedGenericPackageTypes, GenericPackageTypeLimits,
};

mod generic_package_bodies;
pub use generic_package_bodies::{
    check_generic_collection_package_bodies, check_generic_data_package_bodies,
    check_generic_package_bodies, CheckedGenericPackageBodies,
};

mod generic_package_compile;
mod generic_package_symbols;
pub use generic_package_compile::{
    check_generic_package_overlays_with_symbols, check_generic_package_with_overlays,
    compile_generic_collection_package, compile_generic_data_package, compile_generic_package,
    execute_generic_collection_package, execute_generic_data_package, execute_generic_package,
    CompiledGenericPackage, GenericPackageCompilationLimits, GenericPackageLocation,
};

mod generic_package_testing;
pub use generic_package_testing::{
    test_generic_collection_package, test_generic_data_package, test_generic_package,
    GenericPackageTestLimits,
};

mod generic_package_inputs;
pub use generic_package_inputs::GenericPackageSources;

mod generic_package_native;
pub use generic_package_native::{
    decode_generic_collection_package_native_image, decode_generic_data_package_native_image,
    decode_generic_package_native_image, execute_generic_collection_package_native_image,
    execute_generic_data_package_native_image, execute_generic_package_native_image,
    is_generic_package_native_image, GENERIC_PACKAGE_NATIVE_IMAGE_LIMIT,
    GENERIC_PACKAGE_NATIVE_SOURCE_LIMIT,
};

mod runtime_json;
pub use runtime_json::{parse_json, serialize_json, JsonError, JsonLimits, JsonValue};

mod http_chunked_v4;
pub use http_chunked_v4::{HttpChunkedDecoder, HttpChunkedLimits, HttpTrailerLimits};
mod http_exchange_v4;
pub use http_exchange_v4::HttpExchangeLimits;
mod http_response_v4;
pub use http_response_v4::{
    render_http_response, HttpResponseContext, HttpResponseError, HttpResponseHeader,
    HttpResponseLimits,
};
mod http_request_v4;
pub use http_request_v4::{
    parse_http_request, HttpRequest, HttpRequestError, HttpRequestHeader, HttpRequestLimits,
    HttpRequestParse,
};
mod data_host_policy;
pub use data_host_policy::DataHostPolicy;
mod network_host_v4;
pub use network_host_v4::NetworkHostV4;
mod network_v4;
pub use network_v4::{
    LoopbackListener, LoopbackNetwork, LoopbackStream, NetworkLimits, NetworkRead,
};

mod host_v3;
pub use host_v3::{HostError, HostLimits, HostV3};

mod host_dates;
pub use host_dates::{add_date, format_date, parse_date};

pub use generic_package_compile::{
    execute_generic_data_package_with_host, execute_generic_data_package_with_network_host,
};
pub use generic_runtime::{
    execute_generic_data_ir_with_host, execute_generic_data_ir_with_network_host,
};
