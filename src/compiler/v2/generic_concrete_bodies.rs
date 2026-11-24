//! Concrete substitutions and operation/context rechecking before executable lowering.
use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Cumulative admission for rechecking a complete finite specialization graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericConcreteBodyLimits {
    /// Concrete instances admitted for rechecking.
    pub max_instances: usize,
    /// Signature/binding/expression facts, member contexts and propagation descriptors/projections.
    pub max_types: usize,
    /// Generated type nodes, including unused eager nodes.
    pub max_nodes: usize,
    /// Generated type edges.
    pub max_edges: usize,
    /// Concrete compatibility comparison pairs.
    pub max_pairs: usize,
    /// Concrete compatibility comparison edges.
    pub max_comparison_edges: usize,
    /// Substitution, concreteness, operations, context checks and comparison work.
    pub max_work: usize,
    /// Fixed structural depth ceiling, in 1..=128.
    pub max_depth: usize,
    /// Independent per-type substitution profile, capped by remaining totals.
    pub expansion: GenericAliasExpansionLimits,
    /// Independent compatibility comparison profile, capped by remaining totals.
    pub comparison: GenericTypeEquivalenceLimits,
}
impl GenericConcreteBodyLimits {
    /// Finite library profile; no executable generic lowering is authorized.
    pub const DEFAULT: Self = Self {
        max_instances: 1024,
        max_types: 131_072,
        max_nodes: 1_048_576,
        max_edges: 2_097_152,
        max_pairs: 262_144,
        max_comparison_edges: 524_288,
        max_work: 8_000_000,
        max_depth: 128,
        expansion: GenericAliasExpansionLimits::DEFAULT,
        comparison: GenericTypeEquivalenceLimits::DEFAULT,
    };
}
/// Complete concrete refusal; partial rechecked bodies are never published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericConcreteBodyError {
    /// Question-bearing graphs require explicit concrete propagation admission.
    PropagationUnavailable,
    /// Concrete compatibility refusal with original question provenance.
    Propagation {
        /// Concrete graph instance.
        instance: usize,
        /// Original function-local question range.
        span: crate::compiler::diagnostics::Span,
        /// Exact bounded failure compatibility refusal.
        error: Box<GenericPropagationError>,
    },
    /// Selected dimension exhausted or depth profile invalid.
    Limit(&'static str),
    /// Original type substitution/provenance refusal.
    Substitution(GenericAliasExpansionError),
    /// Generated concrete graph/depth validation refusal.
    Concrete(GenericSpecializationError),
    /// Exact type comparison refused.
    Comparison(GenericTypeEquivalenceError),
    /// A concrete operation, conversion, bound or retained identity is inconsistent.
    Context {
        /// Graph-local concrete instance index.
        instance: usize,
        /// Refused operation/context, without an unbounded diagnostic expansion.
        reason: &'static str,
    },
}
impl std::fmt::Display for GenericConcreteBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "concrete generic body recheck refused: {self:?}")
    }
}
impl std::error::Error for GenericConcreteBodyError {}
/// Recomputed closed propagation contract for one concrete source expression.
#[derive(Debug, Clone)]
pub struct ConcreteGenericPropagation {
    expression: GenericCopyExpressionId,
    operand: GenericCopyExpressionId,
    question: crate::compiler::diagnostics::Span,
    types: GenericPropagationTypes,
}
impl ConcreteGenericPropagation {
    /// Original body-owned extraction expression.
    pub fn expression(&self) -> GenericCopyExpressionId {
        self.expression
    }
    /// Earlier body-owned wrapped value.
    pub fn operand(&self) -> GenericCopyExpressionId {
        self.operand
    }
    /// Original question token, shared across instances of the same source.
    pub fn question(&self) -> crate::compiler::diagnostics::Span {
        self.question
    }
    /// Closed success/failure/return contexts, recomputed after substitution.
    pub fn types(&self) -> &GenericPropagationTypes {
        &self.types
    }
}
/// Original symbolic body with a sealed concrete type for every retained fact.
#[derive(Debug, Clone)]
pub struct ConcreteGenericBody {
    propagations: Arc<[ConcreteGenericPropagation]>,
    body: CheckedGenericCopyBody,
    parameters: Arc<[GenericCopyType]>,
    result: GenericCopyType,
    bindings: Arc<[GenericCopyType]>,
    expressions: Arc<[GenericCopyType]>,
}
impl ConcreteGenericBody {
    /// Recomputed concrete postfix facts in source expression order.
    pub fn propagations(&self) -> &[ConcreteGenericPropagation] {
        &self.propagations
    }

    /// Symbolic source/identity/flow proof retained by this instance.
    pub fn body(&self) -> &CheckedGenericCopyBody {
        &self.body
    }
    /// Concrete value parameter types in source order.
    pub fn parameter_types(&self) -> &[GenericCopyType] {
        &self.parameters
    }
    /// Concrete explicit result or existing implicit Unit convention.
    pub fn return_type(&self) -> &GenericCopyType {
        &self.result
    }
    /// Concrete binding type only for this original body's arena.
    pub fn binding_type(&self, id: GenericCopyBindingId) -> Option<&GenericCopyType> {
        self.body.binding(id)?;
        self.bindings.get(id.ordinal())
    }
    /// Concrete expression type only for this original body's arena.
    pub fn expression_type(&self, id: GenericCopyExpressionId) -> Option<&GenericCopyType> {
        self.body.expression(id)?;
        self.expressions.get(id.ordinal())
    }
}
/// Logical rechecking usage, including temporary constructor/field/payload contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericConcreteBodyUsage {
    /// Rechecked concrete instances.
    pub instances: usize,
    /// Generated type facts.
    pub types: usize,
    /// Generated type nodes.
    pub nodes: usize,
    /// Generated type edges.
    pub edges: usize,
    /// Compatibility comparison pairs.
    pub pairs: usize,
    /// Compatibility comparison edges.
    pub comparison_edges: usize,
    /// Consumed logical work.
    pub work: usize,
}
/// Complete concrete body/type consistency proof for one retained specialization graph.
#[derive(Debug, Clone)]
pub struct CheckedConcreteGenericBodies {
    propagation: Option<GenericPropagationUsage>,
    graph: CheckedGenericSpecializations,
    bodies: Arc<[ConcreteGenericBody]>,
    usage: GenericConcreteBodyUsage,
}
impl CheckedConcreteGenericBodies {
    /// Whole-graph concrete propagation costs, when explicitly selected.
    pub fn propagation_usage(&self) -> Option<GenericPropagationUsage> {
        self.propagation
    }

    /// Original finite graph; body indices below match its instance indices.
    pub fn graph(&self) -> &CheckedGenericSpecializations {
        &self.graph
    }
    /// All concrete bodies in graph instance order.
    pub fn bodies(&self) -> &[ConcreteGenericBody] {
        &self.bodies
    }
    /// Measured cumulative rechecking usage.
    pub fn usage(&self) -> GenericConcreteBodyUsage {
        self.usage
    }
}
fn charge(
    left: &mut usize,
    n: usize,
    dimension: &'static str,
) -> Result<(), GenericConcreteBodyError> {
    *left = left
        .checked_sub(n)
        .ok_or(GenericConcreteBodyError::Limit(dimension))?;
    Ok(())
}
fn primitive(value: GenericPrimitive) -> GenericCopyType {
    GenericCopyType::concrete_primitive(value)
}
fn expanded(value: ExpandedGenericType) -> GenericCopyType {
    GenericCopyType::concrete_expanded(value)
}
struct Admission<'a> {
    types: &'a CheckedGenericDeclarations,
    limits: GenericConcreteBodyLimits,
    left: GenericConcreteBodyLimits,
    instance: usize,
}
impl Admission<'_> {
    fn fail(&self, reason: &'static str) -> GenericConcreteBodyError {
        GenericConcreteBodyError::Context {
            instance: self.instance,
            reason,
        }
    }
    fn step(&mut self) -> Result<(), GenericConcreteBodyError> {
        charge(&mut self.left.max_work, 1, "concrete body work")
    }
    fn expansion(&self) -> GenericAliasExpansionLimits {
        GenericAliasExpansionLimits {
            max_nodes: self.limits.expansion.max_nodes.min(self.left.max_nodes),
            max_edges: self.limits.expansion.max_edges.min(self.left.max_edges),
            max_work: self.limits.expansion.max_work.min(self.left.max_work),
            max_depth: self.limits.expansion.max_depth.min(self.limits.max_depth),
            ..self.limits.expansion
        }
    }
    fn generated(&mut self, ty: &ExpandedGenericType) -> Result<(), GenericConcreteBodyError> {
        charge(
            &mut self.left.max_nodes,
            ty.nodes().len(),
            "concrete type nodes",
        )?;
        charge(&mut self.left.max_edges, ty.edges(), "concrete type edges")?;
        charge(&mut self.left.max_work, ty.work(), "concrete body work")?;
        super::generic_specialization::concrete(ty, self.limits.max_depth, &mut self.left.max_work)
            .map_err(GenericConcreteBodyError::Concrete)?;
        self.bound_nodes(ty)
    }
    fn instantiate(
        &mut self,
        ty: &GenericCopyType,
        body: &CheckedGenericCopyBody,
        args: &[ExpandedGenericType],
        scope: &GenericParameters,
    ) -> Result<GenericCopyType, GenericConcreteBodyError> {
        charge(&mut self.left.max_types, 1, "concrete type facts")?;
        self.step()?;
        if let Some(ty) = ty.expanded() {
            let mut foreign = false;
            for node in ty.nodes() {
                self.step()?;
                if let ResolvedGenericTypeKind::Parameter(id) = node.kind {
                    foreign |= body
                        .signature()
                        .syntax()
                        .parameters()
                        .parameter(id)
                        .is_none();
                }
            }
            let projection = if foreign {
                charge(&mut self.left.max_types, 1, "concrete type facts")?;
                let projected = ty
                    .reachable_projection(self.expansion())
                    .map_err(GenericConcreteBodyError::Substitution)?;
                // Projection is an input preparation step and may still contain
                // legitimate caller parameters. Admit its copies before the
                // exact-owner substitution and concrete-result validation.
                charge(
                    &mut self.left.max_nodes,
                    projected.nodes().len(),
                    "concrete type nodes",
                )?;
                charge(
                    &mut self.left.max_edges,
                    projected.edges(),
                    "concrete type edges",
                )?;
                charge(
                    &mut self.left.max_work,
                    projected.work(),
                    "concrete body work",
                )?;
                Some(projected)
            } else {
                None
            };
            let ty = projection.as_ref().unwrap_or(ty);
            let ty = ty
                .substitute_arguments(
                    body.signature().syntax().parameters(),
                    args,
                    scope,
                    self.expansion(),
                )
                .map_err(GenericConcreteBodyError::Substitution)?;
            self.generated(&ty)?;
            Ok(expanded(ty))
        } else {
            Ok(ty.clone())
        }
    }
    fn member(
        &mut self,
        receiver: &GenericCopyType,
        template: &ExpandedGenericType,
        declaration: GenericTypeDeclarationId,
        parameters: &GenericParameters,
    ) -> Result<GenericCopyType, GenericConcreteBodyError> {
        charge(&mut self.left.max_types, 1, "concrete type facts")?;
        self.step()?;
        let receiver = receiver
            .expanded()
            .ok_or_else(|| self.fail("member receiver type"))?;
        let ty = receiver
            .substitute_field(template, declaration, parameters, self.expansion())
            .map_err(GenericConcreteBodyError::Substitution)?;
        self.generated(&ty)?;
        Ok(expanded(ty))
    }
    fn same(
        &mut self,
        a: &GenericCopyType,
        b: &GenericCopyType,
    ) -> Result<(), GenericConcreteBodyError> {
        self.step()?;
        let equal = if let (Some(a), Some(b)) = (a.expanded(), b.expanded()) {
            let result = compare_generic_types(
                a,
                b,
                GenericTypeEquivalenceLimits {
                    max_pairs: self.limits.comparison.max_pairs.min(self.left.max_pairs),
                    max_edges: self
                        .limits
                        .comparison
                        .max_edges
                        .min(self.left.max_comparison_edges),
                    max_work: self.limits.comparison.max_work.min(self.left.max_work),
                },
            )
            .map_err(GenericConcreteBodyError::Comparison)?;
            charge(
                &mut self.left.max_pairs,
                result.pairs(),
                "concrete comparison pairs",
            )?;
            charge(
                &mut self.left.max_comparison_edges,
                result.edges(),
                "concrete comparison edges",
            )?;
            charge(&mut self.left.max_work, result.work(), "concrete body work")?;
            result.equivalent()
        } else {
            if self.limits.comparison.max_pairs == 0 || self.limits.comparison.max_work == 0 {
                return Err(GenericConcreteBodyError::Comparison(
                    GenericTypeEquivalenceError::Limit("scalar comparison profile"),
                ));
            }
            charge(&mut self.left.max_pairs, 1, "concrete comparison pairs")?;
            a.primitive().is_some() && a.primitive() == b.primitive()
        };
        if equal {
            Ok(())
        } else {
            Err(self.fail("concrete type mismatch"))
        }
    }
    fn conversion(
        &mut self,
        a: &GenericCopyType,
        b: &GenericCopyType,
        conversion: GenericCopyConversion,
    ) -> Result<(), GenericConcreteBodyError> {
        self.step()?;
        match conversion {
            GenericCopyConversion::Identity => self.same(a, b),
            GenericCopyConversion::IntToFloat
                if a.primitive() == Some(GenericPrimitive::Int)
                    && b.primitive() == Some(GenericPrimitive::Float) =>
            {
                Ok(())
            }
            _ => Err(self.fail("concrete conversion mismatch")),
        }
    }
    fn bound_nodes(&mut self, ty: &ExpandedGenericType) -> Result<(), GenericConcreteBodyError> {
        for node in ty.nodes() {
            self.step()?;
            if let ResolvedGenericTypeKind::Declared {
                declaration,
                arguments,
            } = &node.kind
            {
                let parameters = match self.types.definition(*declaration) {
                    Some(CheckedGenericDefinition::Record(r)) => r.syntax().parameters(),
                    Some(CheckedGenericDefinition::Enum(e)) => e.syntax().parameters(),
                    Some(CheckedGenericDefinition::Collection(c)) => c.syntax().parameters(),
                    Some(CheckedGenericDefinition::Data(d)) => d.syntax().parameters(),
                    _ => return Err(self.fail("concrete nominal definition")),
                };
                if parameters.parameters().len() != arguments.len() {
                    return Err(self.fail("concrete nominal arity"));
                }
                for (parameter, argument) in parameters.parameters().iter().zip(arguments) {
                    self.step()?;
                    if parameter.bound == Some(GenericParameterBound::Key)
                        && !matches!(
                            ty.nodes()[*argument].kind,
                            ResolvedGenericTypeKind::Primitive(
                                GenericPrimitive::Int
                                    | GenericPrimitive::Bool
                                    | GenericPrimitive::String
                            )
                        )
                        && !matches!(ty.nodes()[*argument].kind,ResolvedGenericTypeKind::Declared{declaration,..} if super::generic_data_types::is_byte_header(self.types.environment().registry(),declaration))
                    {
                        return Err(self.fail("concrete nominal Key bound"));
                    }
                }
            }
        }
        Ok(())
    }
    fn key_bound(&mut self, ty: &GenericCopyType) -> Result<(), GenericConcreteBodyError> {
        self.step()?;
        if matches!(
            ty.primitive(),
            Some(GenericPrimitive::Int | GenericPrimitive::Bool | GenericPrimitive::String)
        ) || ty.expanded().is_some_and(|e|matches!(e.nodes()[e.root()].kind,ResolvedGenericTypeKind::Declared{declaration,..} if super::generic_data_types::is_byte_header(self.types.environment().registry(),declaration))) {
            Ok(())
        } else {
            Err(self.fail("concrete Key bound"))
        }
    }
}
fn nominal(ty: &GenericCopyType) -> Option<GenericTypeDeclarationId> {
    let ty = ty.expanded()?;
    match ty.nodes()[ty.root()].kind {
        ResolvedGenericTypeKind::Declared { declaration, .. } => Some(declaration),
        _ => None,
    }
}
fn element(ty: &GenericCopyType) -> Option<GenericCopyType> {
    ty.expanded()?.array_element().map(expanded)
}
fn expression(
    body: &ConcreteGenericBody,
    id: GenericCopyExpressionId,
) -> Result<&GenericCopyType, GenericConcreteBodyError> {
    body.expression_type(id)
        .ok_or(GenericConcreteBodyError::Limit(
            "concrete expression ownership",
        ))
}
fn binding(
    body: &ConcreteGenericBody,
    id: GenericCopyBindingId,
) -> Result<&GenericCopyType, GenericConcreteBodyError> {
    body.binding_type(id).ok_or(GenericConcreteBodyError::Limit(
        "concrete binding ownership",
    ))
}
fn check_expression(
    a: &mut Admission<'_>,
    body: &ConcreteGenericBody,
    source: &GenericCopyExpression,
    calls: &BTreeMap<usize, usize>,
    all: &[ConcreteGenericBody],
) -> Result<(), GenericConcreteBodyError> {
    a.step()?;
    let result = expression(body, source.id)?;
    let types = body.body.signature().types();
    match &source.kind {
        GenericCopyExpressionKind::Propagate { .. } => {
            Err(a.fail("concrete propagation checking is unavailable"))
        }
        GenericCopyExpressionKind::Builtin { builtin, arguments } => {
            if arguments.len() != builtin.arity() {
                return Err(a.fail("concrete builtin arity"));
            }
            if matches!(
                builtin,
                GenericBuiltin::JsonParse
                    | GenericBuiltin::JsonStringify
                    | GenericBuiltin::HostCall
            ) {
                let input = expression(body, arguments[0])?;
                let valid = if *builtin == GenericBuiltin::JsonParse {
                    input.primitive() == Some(GenericPrimitive::String)
                        && super::generic_copy_bodies::is_json_result(result, types)
                } else {
                    super::generic_copy_bodies::is_json_type(input, types)
                        && if *builtin == GenericBuiltin::HostCall {
                            super::generic_copy_bodies::is_json_result(result, types)
                        } else {
                            super::generic_copy_bodies::is_string_result(result, types)
                        }
                };
                return if valid {
                    Ok(())
                } else {
                    Err(a.fail("JSON operand/result schema"))
                };
            }
            if matches!(
                builtin,
                GenericBuiltin::BytesFromArray
                    | GenericBuiltin::BytesFromText
                    | GenericBuiltin::BytesLen
                    | GenericBuiltin::BytesGet
                    | GenericBuiltin::BytesConcat
                    | GenericBuiltin::BytesSlice
                    | GenericBuiltin::BytesToArray
                    | GenericBuiltin::BytesToHex
                    | GenericBuiltin::BytesFromHex
                    | GenericBuiltin::BytesToBase64
                    | GenericBuiltin::BytesFromBase64
                    | GenericBuiltin::BytesToUtf8
            ) {
                let input = expression(body, arguments[0])?;
                let valid = match builtin {
                    GenericBuiltin::BytesFromArray => {
                        super::generic_copy_bodies::is_byte_array(input, types)
                    }
                    GenericBuiltin::BytesFromText
                    | GenericBuiltin::BytesFromHex
                    | GenericBuiltin::BytesFromBase64 => {
                        input.primitive() == Some(GenericPrimitive::String)
                    }
                    _ => super::generic_copy_bodies::is_bytes_type(input, types),
                };
                if !valid {
                    return Err(a.fail("byte buffer operand"));
                }
                if *builtin == GenericBuiltin::BytesGet {
                    a.same(
                        expression(body, arguments[1])?,
                        &primitive(GenericPrimitive::Int),
                    )?;
                    if !super::generic_copy_bodies::is_byte_result(result, types) {
                        return Err(a.fail("bytes_get Result schema"));
                    }
                } else if matches!(
                    builtin,
                    GenericBuiltin::BytesToHex
                        | GenericBuiltin::BytesToBase64
                        | GenericBuiltin::BytesToUtf8
                ) {
                    if !super::generic_copy_bodies::is_string_result(result, types) {
                        return Err(a.fail("bytes_to_hex Result schema"));
                    }
                } else if *builtin == GenericBuiltin::BytesToArray {
                    if !super::generic_copy_bodies::is_byte_array_result(result, types) {
                        return Err(a.fail("bytes_to_array Result schema"));
                    }
                } else if *builtin == GenericBuiltin::BytesSlice {
                    a.same(
                        expression(body, arguments[1])?,
                        &primitive(GenericPrimitive::Int),
                    )?;
                    a.same(
                        expression(body, arguments[2])?,
                        &primitive(GenericPrimitive::Int),
                    )?;
                    if !super::generic_copy_bodies::is_bytes_result(result, types) {
                        return Err(a.fail("bytes_slice Result schema"));
                    }
                } else if *builtin == GenericBuiltin::BytesConcat {
                    a.same(input, expression(body, arguments[1])?)?;
                    if !super::generic_copy_bodies::is_bytes_result(result, types) {
                        return Err(a.fail("bytes_concat Result schema"));
                    }
                } else if *builtin == GenericBuiltin::BytesLen {
                    a.same(result, &primitive(GenericPrimitive::Int))?;
                } else if !super::generic_copy_bodies::is_bytes_result(result, types) {
                    return Err(a.fail("byte buffer result"));
                }
                return Ok(());
            }
            if *builtin == GenericBuiltin::ByteFromInt {
                a.same(
                    expression(body, arguments[0])?,
                    &primitive(GenericPrimitive::Int),
                )?;
                if !super::generic_copy_bodies::is_byte_result(result, types) {
                    return Err(a.fail("byte conversion Result schema"));
                }
                return Ok(());
            }
            if *builtin == GenericBuiltin::ByteToInt {
                if !super::generic_copy_bodies::is_byte_type(expression(body, arguments[0])?, types)
                {
                    return Err(a.fail("byte conversion operand"));
                }
                a.same(result, &primitive(GenericPrimitive::Int))?;
                return Ok(());
            }
            if let Some(kind) = builtin.collection_kind() {
                if builtin.collection_insert()
                    || builtin.collection_contains()
                    || builtin.collection_get()
                    || builtin.collection_remove()
                    || builtin.collection_iteration()
                {
                    let receiver = expression(body, arguments[0])?;
                    if builtin.collection_iteration() {
                        let actual =
                            element(result).ok_or_else(|| a.fail("iteration result array"))?;
                        if *builtin == GenericBuiltin::MapEntries {
                            let (key, value) =
                                super::generic_copy_bodies::standard_map_entry_arguments(
                                    &actual, types,
                                )
                                .ok_or_else(|| a.fail("iteration standard MapEntry"))?;
                            for (index, actual) in [key, value].iter().enumerate() {
                                let expected = super::generic_copy_bodies::collection_argument(
                                    receiver, index,
                                )
                                .ok_or_else(|| a.fail("entry argument type"))?;
                                a.same(actual, &expected)?;
                            }
                        } else {
                            let expected = super::generic_copy_bodies::collection_argument(
                                receiver,
                                usize::from(*builtin == GenericBuiltin::MapValues),
                            )
                            .ok_or_else(|| a.fail("iteration element type"))?;
                            a.same(&actual, &expected)?;
                        }
                    } else if builtin.collection_remove() {
                        let payload =
                            super::generic_copy_bodies::standard_option_argument(result, types)
                                .ok_or_else(|| a.fail("removal standard Option result"))?;
                        a.same(&payload, receiver)?;
                    } else if builtin.collection_get() {
                        let actual =
                            super::generic_copy_bodies::standard_option_argument(result, types)
                                .ok_or_else(|| a.fail("lookup standard Option result"))?;
                        let expected = super::generic_copy_bodies::collection_argument(
                            receiver,
                            usize::from(*builtin == GenericBuiltin::MapGet),
                        )
                        .ok_or_else(|| a.fail("lookup payload type"))?;
                        a.same(&actual, &expected)?;
                    } else if builtin.collection_contains() {
                        a.same(result, &primitive(GenericPrimitive::Bool))?;
                    } else {
                        a.same(result, receiver)?;
                    }
                    if super::generic_copy_bodies::collection_type_kind(receiver, types)
                        != Some(kind)
                    {
                        return Err(a.fail("concrete collection operation receiver"));
                    }
                    for (index, arg) in arguments[1..].iter().enumerate() {
                        let expected =
                            super::generic_copy_bodies::collection_argument(receiver, index)
                                .ok_or_else(|| a.fail("collection argument type"))?;
                        a.same(expression(body, *arg)?, &expected)?;
                    }
                    return Ok(());
                }
                let operand = if builtin.collection_new() {
                    result
                } else {
                    expression(body, arguments[0])?
                };
                if super::generic_copy_bodies::collection_type_kind(operand, types) != Some(kind) {
                    return Err(a.fail("concrete collection builtin constructor"));
                }
                if !builtin.collection_new() {
                    a.same(result, &primitive(GenericPrimitive::Int))?;
                }
                return Ok(());
            }
            if builtin.array_operation() {
                let first = expression(body, arguments[0])?;
                let target =
                    element(first).ok_or_else(|| a.fail("array operation first argument"))?;
                a.same(result, first)?;
                if *builtin == GenericBuiltin::ArraySlice {
                    for id in &arguments[1..] {
                        a.same(expression(body, *id)?, &primitive(GenericPrimitive::Int))?;
                    }
                } else {
                    let mut actual = expression(body, arguments[1])?.clone();
                    let mut expected = if *builtin == GenericBuiltin::ArrayAppend {
                        target
                    } else {
                        first.clone()
                    };
                    loop {
                        a.step()?;
                        match (element(&actual), element(&expected)) {
                            (Some(x), Some(y)) => {
                                actual = x;
                                expected = y;
                            }
                            _ => {
                                if !(actual.primitive() == Some(GenericPrimitive::Int)
                                    && expected.primitive() == Some(GenericPrimitive::Float))
                                {
                                    a.same(&actual, &expected)?;
                                }
                                break;
                            }
                        }
                    }
                }
                return Ok(());
            }
            for id in arguments {
                let actual = expression(body, *id)?;
                if *builtin == GenericBuiltin::ToString {
                    continue;
                }
                if *builtin == GenericBuiltin::ArrayLen {
                    if element(actual).is_none() {
                        return Err(a.fail("array_len concrete argument"));
                    }
                    continue;
                }
                a.same(actual, &primitive(GenericPrimitive::String))?;
            }
            if let Some(kind) = builtin.result_primitive() {
                return a.same(result, &primitive(kind));
            }
            let id = nominal(result).ok_or_else(|| a.fail("concrete builtin result"))?;
            if !matches!(types.definition(id),Some(CheckedGenericDefinition::Record(record)) if record.syntax().is_standard_result() && record.syntax().qualified_name()==builtin.result_name())
            {
                return Err(a.fail("concrete builtin nominal identity"));
            }
            Ok(())
        }
        GenericCopyExpressionKind::Literal(kind) => a.same(result, &primitive(*kind)),
        GenericCopyExpressionKind::Read(id) => a.same(result, binding(body, *id)?),
        GenericCopyExpressionKind::Call {
            signature,
            arguments,
            ..
        } => {
            let target = *calls
                .get(&source.id.ordinal())
                .ok_or_else(|| a.fail("missing concrete call edge"))?;
            let callee = all
                .get(target)
                .ok_or_else(|| a.fail("concrete call target"))?;
            if callee.body.signature().id() != signature.binding().callee()
                || arguments.len() != callee.parameters.len()
            {
                return Err(a.fail("concrete call signature"));
            }
            for (argument, expected) in arguments.iter().zip(callee.parameters.iter()) {
                a.conversion(
                    expression(body, argument.expression)?,
                    expected,
                    argument.conversion,
                )?;
            }
            a.same(result, &callee.result)
        }
        GenericCopyExpressionKind::Array(items) => {
            let element = element(result).ok_or_else(|| a.fail("array result type"))?;
            for item in items {
                a.conversion(expression(body, item.value)?, &element, item.conversion)?;
            }
            Ok(())
        }
        GenericCopyExpressionKind::Index { target, index } => {
            a.same(expression(body, *index)?, &primitive(GenericPrimitive::Int))?;
            let element =
                element(expression(body, *target)?).ok_or_else(|| a.fail("indexed array type"))?;
            a.same(result, &element)
        }
        GenericCopyExpressionKind::Logical { left, right, .. } => {
            for ty in [expression(body, *left)?, expression(body, *right)?, result] {
                a.same(ty, &primitive(GenericPrimitive::Bool))?;
            }
            Ok(())
        }
        GenericCopyExpressionKind::String {
            operator,
            left,
            right,
        } => {
            for ty in [expression(body, *left)?, expression(body, *right)?] {
                a.same(ty, &primitive(GenericPrimitive::String))?;
            }
            a.same(
                result,
                &primitive(if *operator == GenericStringOperator::Concatenate {
                    GenericPrimitive::String
                } else {
                    GenericPrimitive::Bool
                }),
            )
        }
        GenericCopyExpressionKind::Numeric {
            operator,
            left,
            right,
            left_conversion,
            right_conversion,
        } => {
            let left = expression(body, *left)?;
            let right = expression(body, *right)?;
            let numeric = match (left.primitive(), right.primitive()) {
                (Some(GenericPrimitive::Int), Some(GenericPrimitive::Int)) => GenericPrimitive::Int,
                (
                    Some(GenericPrimitive::Float),
                    Some(GenericPrimitive::Int | GenericPrimitive::Float),
                )
                | (Some(GenericPrimitive::Int), Some(GenericPrimitive::Float)) => {
                    GenericPrimitive::Float
                }
                _ => return Err(a.fail("concrete numeric operands")),
            };
            a.conversion(left, &primitive(numeric), *left_conversion)?;
            a.conversion(right, &primitive(numeric), *right_conversion)?;
            let ordering = matches!(
                operator,
                GenericNumericOperator::Less
                    | GenericNumericOperator::LessEqual
                    | GenericNumericOperator::Greater
                    | GenericNumericOperator::GreaterEqual
            );
            a.same(
                result,
                &primitive(if ordering {
                    GenericPrimitive::Bool
                } else {
                    numeric
                }),
            )
        }
        GenericCopyExpressionKind::Equality {
            left,
            right,
            left_conversion,
            right_conversion,
            ..
        } => {
            let left = expression(body, *left)?;
            let right = expression(body, *right)?;
            if *left_conversion == GenericCopyConversion::Identity
                && *right_conversion == GenericCopyConversion::Identity
            {
                a.same(left, right)?;
            } else {
                a.conversion(left, &primitive(GenericPrimitive::Float), *left_conversion)?;
                a.conversion(
                    right,
                    &primitive(GenericPrimitive::Float),
                    *right_conversion,
                )?;
            }
            a.same(result, &primitive(GenericPrimitive::Bool))
        }
        GenericCopyExpressionKind::Field {
            receiver,
            declaration,
            field_index,
            ..
        } => {
            let receiver = expression(body, *receiver)?;
            if nominal(receiver) != Some(*declaration) {
                return Err(a.fail("concrete field nominal identity"));
            }
            let Some(CheckedGenericDefinition::Record(record)) = types.definition(*declaration)
            else {
                return Err(a.fail("concrete field definition"));
            };
            let template = record
                .field_types()
                .get(*field_index)
                .ok_or_else(|| a.fail("concrete field index"))?;
            let expected = a.member(
                receiver,
                template,
                *declaration,
                record.syntax().parameters(),
            )?;
            a.same(result, &expected)
        }
        GenericCopyExpressionKind::Record {
            declaration,
            fields,
            ..
        } => {
            if nominal(result) != Some(*declaration) {
                return Err(a.fail("concrete record nominal identity"));
            }
            let Some(CheckedGenericDefinition::Record(record)) = types.definition(*declaration)
            else {
                return Err(a.fail("concrete record definition"));
            };
            if fields.len() != record.field_types().len() {
                return Err(a.fail("concrete field coverage"));
            }
            let mut seen = BTreeSet::new();
            for field in fields {
                a.step()?;
                if !seen.insert(field.field_index) {
                    return Err(a.fail("duplicate concrete field"));
                }
                let template = record
                    .field_types()
                    .get(field.field_index)
                    .ok_or_else(|| a.fail("concrete field index"))?;
                let expected =
                    a.member(result, template, *declaration, record.syntax().parameters())?;
                a.conversion(expression(body, field.value)?, &expected, field.conversion)?;
            }
            Ok(())
        }
        GenericCopyExpressionKind::Enum {
            declaration,
            variant_index,
            payload,
            ..
        } => {
            if nominal(result) != Some(*declaration) {
                return Err(a.fail("concrete enum nominal identity"));
            }
            let Some(CheckedGenericDefinition::Enum(definition)) = types.definition(*declaration)
            else {
                return Err(a.fail("concrete enum definition"));
            };
            let template = definition
                .payload_types()
                .get(*variant_index)
                .ok_or_else(|| a.fail("concrete variant index"))?;
            match (template, payload) {
                (Some(template), Some(payload)) => {
                    let expected = a.member(
                        result,
                        template,
                        *declaration,
                        definition.syntax().parameters(),
                    )?;
                    a.conversion(
                        expression(body, payload.value)?,
                        &expected,
                        payload.conversion,
                    )
                }
                (None, None) => Ok(()),
                _ => Err(a.fail("concrete payload shape")),
            }
        }
    }
}
fn check_statement(
    a: &mut Admission<'_>,
    body: &ConcreteGenericBody,
    statement: &GenericCopyStatement,
) -> Result<(), GenericConcreteBodyError> {
    a.step()?;
    match statement {
        GenericCopyStatement::Let {
            binding: id,
            value,
            conversion,
            ..
        } => a.conversion(expression(body, *value)?, binding(body, *id)?, *conversion),
        GenericCopyStatement::Assign {
            binding: id,
            value,
            conversion,
            ..
        } => {
            if body.body.binding(*id).map(|b| b.kind) != Some(GenericCopyBindingKind::MutableLocal)
            {
                return Err(a.fail("concrete assignment mutability"));
            }
            a.conversion(expression(body, *value)?, binding(body, *id)?, *conversion)
        }
        GenericCopyStatement::IndexAssign {
            binding: id,
            index,
            value,
            conversion,
            ..
        } => {
            if body.body.binding(*id).map(|b| b.kind) != Some(GenericCopyBindingKind::MutableLocal)
            {
                return Err(a.fail("concrete indexed assignment mutability"));
            }
            a.same(expression(body, *index)?, &primitive(GenericPrimitive::Int))?;
            let element =
                element(binding(body, *id)?).ok_or_else(|| a.fail("concrete mutable array"))?;
            a.conversion(expression(body, *value)?, &element, *conversion)
        }
        GenericCopyStatement::If { condition, .. }
        | GenericCopyStatement::While { condition, .. } => a.same(
            expression(body, *condition)?,
            &primitive(GenericPrimitive::Bool),
        ),
        GenericCopyStatement::Expression { value } => {
            expression(body, *value)?;
            Ok(())
        }
        GenericCopyStatement::Return {
            value, conversion, ..
        } => {
            let actual = if let Some(value) = value {
                expression(body, *value)?.clone()
            } else {
                primitive(GenericPrimitive::Unit)
            };
            a.conversion(&actual, &body.result, *conversion)
        }
        GenericCopyStatement::Match {
            scrutinee,
            declaration,
            arms,
            ..
        } => {
            let scrutinee_type = expression(body, *scrutinee)?;
            if nominal(scrutinee_type) != Some(*declaration) {
                return Err(a.fail("concrete match nominal identity"));
            }
            let Some(CheckedGenericDefinition::Enum(definition)) =
                body.body.signature().types().definition(*declaration)
            else {
                return Err(a.fail("concrete match definition"));
            };
            if arms.len() != definition.payload_types().len() {
                return Err(a.fail("concrete match coverage"));
            }
            let mut seen = BTreeSet::new();
            for arm in arms {
                a.step()?;
                if !seen.insert(arm.variant_index) {
                    return Err(a.fail("duplicate concrete match arm"));
                }
                let template = definition
                    .payload_types()
                    .get(arm.variant_index)
                    .ok_or_else(|| a.fail("concrete match variant"))?;
                match (template, arm.payload_binding) {
                    (Some(template), Some(id)) => {
                        if body.body.binding(id).map(|b| b.kind)
                            != Some(GenericCopyBindingKind::MatchPayload {
                                scrutinee: *scrutinee,
                                variant_index: arm.variant_index,
                            })
                        {
                            return Err(a.fail("concrete payload binding origin"));
                        }
                        let expected = a.member(
                            scrutinee_type,
                            template,
                            *declaration,
                            definition.syntax().parameters(),
                        )?;
                        a.same(binding(body, id)?, &expected)?;
                    }
                    (None, None) => {}
                    _ => return Err(a.fail("concrete match payload shape")),
                }
            }
            Ok(())
        }
    }
}
fn check_propagation(
    a: &mut Admission<'_>,
    body: &ConcreteGenericBody,
    source: &GenericCopyExpression,
    args: &[ExpandedGenericType],
    scope: &GenericParameters,
    left: &mut GenericPropagationLimits,
) -> Result<ConcreteGenericPropagation, GenericConcreteBodyError> {
    a.step()?;
    let GenericCopyExpressionKind::Propagate {
        operand,
        question,
        types: original,
    } = &source.kind
    else {
        return Err(a.fail("propagation expression kind"));
    };
    let input = body
        .body
        .expression(*operand)
        .ok_or_else(|| a.fail("propagation operand ownership"))?;
    if operand.ordinal() >= source.id.ordinal()
        || source.span.start > input.span.start
        || input.span.end > question.start
        || question.end > source.span.end
        || body
            .body
            .signature()
            .syntax()
            .source()
            .get(question.start..question.end)
            != Some("?")
    {
        return Err(a.fail("propagation question ownership"));
    }
    let actual_operand = expression(body, *operand)?;
    let result = body
        .result
        .expanded()
        .ok_or_else(|| a.fail("propagation return wrapper"))?;
    let mut local = *left;
    local.max_work = local.max_work.min(a.left.max_work);
    local.max_pairs = local
        .max_pairs
        .min(a.left.max_pairs)
        .min(a.limits.comparison.max_pairs);
    local.max_edges = local
        .max_edges
        .min(a.left.max_comparison_edges)
        .min(a.limits.comparison.max_edges);
    let before = local;
    let types = super::generic_propagation::check_propagation_context(
        a.types,
        body.body.signature().id(),
        actual_operand,
        result,
        input.span,
        &mut local,
        a.limits.comparison,
    )
    .map_err(|error| GenericConcreteBodyError::Propagation {
        instance: a.instance,
        span: *question,
        error: Box::new(error),
    })?;
    let work = before.max_work - local.max_work;
    let pairs = before.max_pairs - local.max_pairs;
    let edges = before.max_edges - local.max_edges;
    charge(&mut a.left.max_work, work, "concrete propagation work")?;
    charge(&mut a.left.max_pairs, pairs, "concrete propagation pairs")?;
    charge(
        &mut a.left.max_comparison_edges,
        edges,
        "concrete propagation comparison edges",
    )?;
    let contexts = before.max_contexts - local.max_contexts;
    charge(
        &mut a.left.max_types,
        contexts,
        "concrete propagation contexts",
    )?;
    local.max_work = left.max_work - work;
    local.max_pairs = left.max_pairs - pairs;
    local.max_edges = left.max_edges - edges;
    *left = local;
    if types.kind() != original.kind()
        || types.declaration() != original.declaration()
        || types.success_variant() != original.success_variant()
        || types.failure_variant() != original.failure_variant()
    {
        return Err(a.fail("propagation nominal descriptor"));
    }
    let expected = a.instantiate(
        &expanded(original.success_type().clone()),
        &body.body,
        args,
        scope,
    )?;
    a.same(&expected, expression(body, source.id)?)?;
    a.same(
        &expanded(types.success_type().clone()),
        expression(body, source.id)?,
    )?;
    let expected = a.instantiate(
        &expanded(original.return_type().clone()),
        &body.body,
        args,
        scope,
    )?;
    a.same(&expected, &body.result)?;
    match (original.failure_type(), types.failure_type()) {
        (Some(original), Some(actual)) => {
            let expected = a.instantiate(&expanded(original.clone()), &body.body, args, scope)?;
            a.same(&expected, &expanded(actual.clone()))?;
        }
        (None, None) => {}
        _ => return Err(a.fail("propagation failure descriptor")),
    }
    Ok(ConcreteGenericPropagation {
        expression: source.id,
        operand: *operand,
        question: *question,
        types,
    })
}
/// Recheck ordinary concrete bodies; postfix graphs require explicit propagation admission.
pub fn recheck_generic_concrete_bodies(
    graph: &CheckedGenericSpecializations,
    limits: GenericConcreteBodyLimits,
) -> Result<CheckedConcreteGenericBodies, GenericConcreteBodyError> {
    recheck_mode(graph, limits, None)
}
/// Recheck every reachable concrete propagation operand, descriptor and failure
/// contract under shared budgets. Does not authorize instruction emission.
pub fn recheck_generic_propagation_concrete_bodies(
    graph: &CheckedGenericSpecializations,
    limits: GenericConcreteBodyLimits,
    propagation: GenericPropagationLimits,
) -> Result<CheckedConcreteGenericBodies, GenericConcreteBodyError> {
    recheck_mode(graph, limits, Some(propagation))
}
fn recheck_mode(
    graph: &CheckedGenericSpecializations,
    limits: GenericConcreteBodyLimits,
    propagation: Option<GenericPropagationLimits>,
) -> Result<CheckedConcreteGenericBodies, GenericConcreteBodyError> {
    if propagation.is_none()
        && graph
            .bodies()
            .propagation_usage()
            .is_some_and(|usage| usage.sites > 0)
    {
        return Err(GenericConcreteBodyError::PropagationUnavailable);
    }
    let mut propagation_left = propagation;
    if limits.max_depth == 0 || limits.max_depth > 128 {
        return Err(GenericConcreteBodyError::Limit("concrete depth profile"));
    }
    let scope = graph
        .bodies()
        .body(graph.instances()[0].function())
        .expect("sealed root")
        .signature()
        .syntax()
        .parameters();
    let mut a = Admission {
        limits,
        left: limits,
        instance: 0,
        types: graph.bodies().signatures().types(),
    };
    let mut bodies = Vec::new();
    for (index, instance) in graph.instances().iter().enumerate() {
        a.instance = index;
        charge(&mut a.left.max_instances, 1, "concrete instances")?;
        a.step()?;
        let body = graph
            .bodies()
            .body(instance.function())
            .expect("sealed instance");
        for (parameter, argument) in body
            .signature()
            .syntax()
            .parameters()
            .parameters()
            .iter()
            .zip(instance.arguments())
        {
            a.step()?;
            a.bound_nodes(argument)?;
            if parameter.bound == Some(GenericParameterBound::Key) {
                a.key_bound(&expanded(argument.clone()))?;
            }
        }
        let parameters = body
            .signature()
            .parameter_types()
            .iter()
            .map(|ty| a.instantiate(&expanded(ty.clone()), body, instance.arguments(), scope))
            .collect::<Result<Vec<_>, _>>()?;
        let result = match body.signature().return_type() {
            GenericSignatureReturn::ImplicitUnit => primitive(GenericPrimitive::Unit),
            GenericSignatureReturn::Explicit(ty) => expanded(ty.clone()),
        };
        let result = a.instantiate(&result, body, instance.arguments(), scope)?;
        let bindings = body
            .bindings()
            .iter()
            .map(|b| a.instantiate(&b.ty, body, instance.arguments(), scope))
            .collect::<Result<Vec<_>, _>>()?;
        let expressions = body
            .expressions()
            .iter()
            .map(|e| a.instantiate(&e.ty, body, instance.arguments(), scope))
            .collect::<Result<Vec<_>, _>>()?;
        bodies.push(ConcreteGenericBody {
            propagations: Arc::from([]),
            body: body.clone(),
            parameters: parameters.into(),
            result,
            bindings: bindings.into(),
            expressions: expressions.into(),
        });
    }
    let mut all_propagations = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        let mut facts = Vec::new();
        a.instance = index;
        a.step()?;
        let mut calls = BTreeMap::new();
        for call in graph.instances()[index].calls() {
            a.step()?;
            if body.body.expression(call.expression).is_none()
                || calls
                    .insert(call.expression.ordinal(), call.target)
                    .is_some()
            {
                return Err(a.fail("concrete call edge ownership"));
            }
        }
        for source in body.body.bindings() {
            a.step()?;
            if let GenericCopyBindingKind::Parameter(ordinal) = source.kind {
                let expected = body
                    .parameters
                    .get(ordinal)
                    .ok_or_else(|| a.fail("concrete parameter origin"))?;
                a.same(binding(body, source.id)?, expected)?;
            }
        }
        for source in body.body.expressions() {
            if let (GenericCopyExpressionKind::Propagate { .. }, Some(left)) =
                (&source.kind, &mut propagation_left)
            {
                facts.push(check_propagation(
                    &mut a,
                    body,
                    source,
                    graph.instances()[index].arguments(),
                    scope,
                    left,
                )?);
            } else {
                check_expression(&mut a, body, source, &calls, &bodies)?;
            }
        }
        for statement in body.body.statements() {
            check_statement(&mut a, body, statement)?;
        }
        if body.body.implicit_unit_return() {
            a.same(&body.result, &primitive(GenericPrimitive::Unit))?;
        }
        all_propagations.push(facts);
    }
    for (body, facts) in bodies.iter_mut().zip(all_propagations) {
        body.propagations = facts.into();
    }
    Ok(CheckedConcreteGenericBodies {
        propagation: propagation.map(|initial| {
            let left = propagation_left.expect("selected concrete propagation profile");
            GenericPropagationUsage {
                sites: initial.max_sites - left.max_sites,
                contexts: initial.max_contexts - left.max_contexts,
                pairs: initial.max_pairs - left.max_pairs,
                edges: initial.max_edges - left.max_edges,
                work: initial.max_work - left.max_work,
            }
        }),
        graph: graph.clone(),
        bodies: bodies.into(),
        usage: GenericConcreteBodyUsage {
            instances: limits.max_instances - a.left.max_instances,
            types: limits.max_types - a.left.max_types,
            nodes: limits.max_nodes - a.left.max_nodes,
            edges: limits.max_edges - a.left.max_edges,
            pairs: limits.max_pairs - a.left.max_pairs,
            comparison_edges: limits.max_comparison_edges - a.left.max_comparison_edges,
            work: limits.max_work - a.left.max_work,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn checked() -> CheckedConcreteGenericBodies {
        let registry = register_generic_types(&[], GenericRegistryLimits::DEFAULT).unwrap();
        let env = prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT)
            .unwrap();
        let types =
            check_generic_declarations(&env, &[], GenericDeclarationLimits::DEFAULT).unwrap();
        let root = parse_specialization_root(
            "fn root(){identity::<Int>(1);identity::<String>(\"s\");let mut n:Float=1;n=2;}",
            GenericFunctionLimits::DEFAULT,
        )
        .unwrap();
        let id = parse_generic_function(
            "fn identity<T>(x:T)->T{return x;}",
            GenericFunctionLimits::DEFAULT,
        )
        .unwrap();
        let signatures =
            check_generic_signatures(&types, &[&root, &id], GenericSignatureLimits::DEFAULT)
                .unwrap();
        let bodies =
            check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
        let graph = discover_generic_specializations(
            &bodies,
            signatures.find("root").unwrap(),
            GenericSpecializationLimits::DEFAULT,
        )
        .unwrap();
        recheck_generic_concrete_bodies(&graph, GenericConcreteBodyLimits::DEFAULT).unwrap()
    }
    fn admission(checked: &CheckedConcreteGenericBodies) -> Admission<'_> {
        Admission {
            types: checked.graph.bodies().signatures().types(),
            limits: GenericConcreteBodyLimits::DEFAULT,
            left: GenericConcreteBodyLimits::DEFAULT,
            instance: 0,
        }
    }
    #[test]
    fn inconsistent_stored_concrete_results_and_call_targets_are_refused() {
        let checked = checked();
        let body = &checked.bodies[0];
        let source = body
            .body
            .expressions()
            .iter()
            .find(|e| {
                matches!(e.kind, GenericCopyExpressionKind::Call { .. })
                    && e.ty.primitive() == Some(GenericPrimitive::String)
            })
            .unwrap();
        let calls: BTreeMap<_, _> = checked.graph.instances()[0]
            .calls()
            .iter()
            .map(|c| (c.expression.ordinal(), c.target))
            .collect();
        let mut stale = body.clone();
        let mut facts = stale.expressions.to_vec();
        facts[source.id.ordinal()] = primitive(GenericPrimitive::Bool);
        stale.expressions = facts.into();
        assert!(matches!(
            check_expression(
                &mut admission(&checked),
                &stale,
                source,
                &calls,
                &checked.bodies
            ),
            Err(GenericConcreteBodyError::Context { .. })
        ));
        let mut stale_targets = calls.clone();
        stale_targets.insert(source.id.ordinal(), 1);
        assert!(matches!(
            check_expression(
                &mut admission(&checked),
                body,
                source,
                &stale_targets,
                &checked.bodies
            ),
            Err(GenericConcreteBodyError::Context { .. })
        ));
        assert!(check_expression(
            &mut admission(&checked),
            body,
            source,
            &BTreeMap::new(),
            &checked.bodies
        )
        .is_err());
    }
    #[test]
    fn root_projection_does_not_erase_reachable_foreign_parameters() {
        let checked = checked();
        let foreign_scope =
            parse_generic_parameters("<Foreign>", GenericParameterLimits::DEFAULT).unwrap();
        let syntax = parse_generic_type("Foreign", GenericTypeLimits::DEFAULT).unwrap();
        let foreign = checked
            .graph
            .bodies()
            .signatures()
            .types()
            .environment()
            .expand(
                &syntax,
                &foreign_scope,
                GenericAliasExpansionLimits::DEFAULT,
            )
            .unwrap();
        let source = &checked.bodies[1].body;
        let scope = checked.bodies[0].body.signature().syntax().parameters();
        let result = admission(&checked).instantiate(
            &expanded(foreign),
            source,
            checked.graph.instances()[1].arguments(),
            scope,
        );
        assert!(matches!(
            result,
            Err(GenericConcreteBodyError::Substitution(
                GenericAliasExpansionError::Environment("foreign callee parameter")
            ))
        ));
    }
    #[test]
    fn incorrect_scalar_conversion_edges_do_not_get_silently_recomputed() {
        let checked = checked();
        let body = &checked.bodies[0];
        let mut source = body
            .body
            .statements()
            .iter()
            .find(|s| matches!(s, GenericCopyStatement::Assign { .. }))
            .unwrap()
            .clone();
        let GenericCopyStatement::Assign { conversion, .. } = &mut source else {
            unreachable!()
        };
        assert_eq!(*conversion, GenericCopyConversion::IntToFloat);
        *conversion = GenericCopyConversion::Identity;
        assert!(matches!(
            check_statement(&mut admission(&checked), body, &source),
            Err(GenericConcreteBodyError::Context { .. })
        ));
    }
}

#[cfg(test)]
mod propagation_tests {
    use super::*;
    fn input() -> CheckedGenericCopyBodies {
        let base =
            compile_generic_source("fn main(){}", "main", GenericSourceLimits::DEFAULT).unwrap();
        let types = base
            .ir()
            .plans()
            .layouts()
            .literals()
            .concrete()
            .graph()
            .bodies()
            .signatures()
            .types();
        let source = [
            "fn root()->std::Option<Int>{return test(std::Option::<Int>::Some(1));}",
            "fn test(x:std::Option<Int>)->std::Option<Int>{let value=x?;return x;}",
            "fn donor(x:std::Option<String>)->std::Option<String>{let value=x?;return x;}",
        ];
        let syntax: Vec<_> = source
            .iter()
            .map(|s| parse_generic_propagation_function(s, GenericFunctionLimits::DEFAULT).unwrap())
            .collect();
        let signatures = check_generic_signatures(
            types,
            &syntax.iter().collect::<Vec<_>>(),
            GenericSignatureLimits::DEFAULT,
        )
        .unwrap();
        check_generic_propagation_bodies(
            &signatures,
            GenericCopyBodyLimits::DEFAULT,
            GenericPropagationLimits::DEFAULT,
        )
        .unwrap()
    }
    fn check(
        bodies: &CheckedGenericCopyBodies,
    ) -> Result<CheckedConcreteGenericBodies, GenericConcreteBodyError> {
        let graph = discover_generic_propagation_specializations(
            bodies,
            bodies.signatures().find("root").unwrap(),
            GenericSpecializationLimits::DEFAULT,
        )
        .unwrap();
        recheck_generic_propagation_concrete_bodies(
            &graph,
            GenericConcreteBodyLimits::DEFAULT,
            GenericPropagationLimits::DEFAULT,
        )
    }
    #[test]
    fn stale_symbolic_payload_descriptor_is_refused_not_repaired() {
        let mut bodies = input();
        check(&bodies).unwrap();
        super::super::generic_copy_bodies::corrupt_propagation_for_test(&mut bodies, false);
        assert!(check(&bodies).is_err());
    }
    #[test]
    fn cyclic_source_operand_identity_is_refused_before_context_reconstruction() {
        let mut bodies = input();
        check(&bodies).unwrap();
        super::super::generic_copy_bodies::corrupt_propagation_for_test(&mut bodies, true);
        assert!(matches!(
            check(&bodies),
            Err(GenericConcreteBodyError::Context {
                reason: "propagation question ownership",
                ..
            })
        ));
    }
}
