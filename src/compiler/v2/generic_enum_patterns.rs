//! Exhaustive generic enum pattern headers for an explicitly selected typed binding.
use super::{
    parse_match_statement, CheckedGenericCopyBody, CheckedGenericDefinition, ExpandedGenericType,
    GenericAliasExpansionError, GenericAliasExpansionLimits, GenericCopyBinding,
    GenericCopyBindingId, GenericFieldSubstitutionLimits, GenericTypeDeclarationId,
    MatchStatementSyntax, MatchSyntaxLimits, ResolvedGenericTypeKind,
};
use crate::compiler::{
    ast::Statement,
    diagnostics::{Diagnostics, Span},
    resources::FrontendLimits,
};
use std::{collections::BTreeSet, sync::Arc};
/// Syntax and cumulative payload-context admission for one standalone match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericEnumPatternLimits {
    /// Original match source/header/arm admission. Bodies use ordinary parser grammar.
    pub syntax: MatchSyntaxLimits,
    /// Generated payload-context nodes and provenance slots across all arms.
    pub max_nodes: usize,
    /// Generated payload-context edges across all arms.
    pub max_edges: usize,
    /// Binding/coverage/name-copy and substitution work across all arms.
    pub max_work: usize,
    /// Independent per-payload output/work/depth profile.
    pub payload: GenericFieldSubstitutionLimits,
}
impl GenericEnumPatternLimits {
    /// Finite header/coverage profile; no arm body typing or execution.
    pub const DEFAULT: Self = Self {
        syntax: MatchSyntaxLimits {
            frontend: FrontendLimits {
                max_source_bytes: 64 * 1024,
                max_tokens: 16_384,
                max_diagnostics: 64,
            },
            max_arms: 256,
            max_header_text_bytes: 32 * 1024,
        },
        max_nodes: 65_536,
        max_edges: 131_072,
        max_work: 262_144,
        payload: GenericFieldSubstitutionLimits::DEFAULT,
    };
}
/// Complete pattern refusal without partial coverage or payload binding evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericEnumPatternError {
    /// Original standalone match parser failure.
    Syntax(Diagnostics),
    /// Selected logical allowance exhausted.
    Limit(&'static str),
    /// Supplied binding ID does not belong to the supplied body arena.
    ScrutineeBinding,
    /// Selected binding does not have a known checked nominal enum type.
    ScrutineeType,
    /// Source identifier differs from the explicitly selected binding name.
    ScrutineeName {
        /// Original standalone match-source identifier range.
        span: Span,
    },
    /// Arm names another enum, alias or qualified head.
    ForeignEnum {
        /// Source-order arm index.
        arm: usize,
        /// Original standalone arm range.
        span: Span,
    },
    /// Variant does not exist on the selected nominal enum.
    UnknownVariant {
        /// Source-order arm index.
        arm: usize,
        /// Original standalone arm range.
        span: Span,
    },
    /// Variant appears more than once.
    DuplicateVariant {
        /// Source-order repeated arm index.
        arm: usize,
        /// Original standalone arm range.
        span: Span,
    },
    /// Pattern zero/one binding shape differs from the declared payload shape.
    PayloadArity {
        /// Source-order arm index.
        arm: usize,
        /// Original standalone arm range.
        span: Span,
    },
    /// Not every variant appears exactly once.
    MissingVariants {
        /// Number of uncovered variants; no unbounded name list is created.
        count: usize,
        /// Original whole standalone match range.
        span: Span,
    },
    /// Bounded exact payload context substitution failed.
    Payload {
        /// Source-order arm index.
        arm: usize,
        /// Original standalone arm range.
        span: Span,
        /// Original annotation/alias error provenance is retained.
        error: Box<GenericAliasExpansionError>,
    },
}
impl std::fmt::Display for GenericEnumPatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "generic enum patterns refused: {self:?}")
    }
}
impl std::error::Error for GenericEnumPatternError {}
/// Expected immutable arm-local payload binding, not an allocated body binding ID.
#[derive(Debug, Clone)]
pub struct GenericEnumPatternPayload {
    /// Original pattern binding name, allowing ordinary shadowing in a future arm scope.
    pub name: Arc<str>,
    /// Original standalone match-source name range.
    pub span: Span,
    /// Exact substituted payload type and original caller/alias provenance.
    pub ty: ExpandedGenericType,
}
/// Source-order exhaustive arm header; its body is not typed by this stage.
#[derive(Debug, Clone)]
pub struct GenericEnumPatternArm {
    /// Source-order variant index in the original checked enum.
    pub variant_index: usize,
    /// Original standalone whole arm range, including its untyped body.
    pub span: Span,
    /// None is nullary, while Some(Unit-typed binding) remains distinct.
    pub payload: Option<GenericEnumPatternPayload>,
}
/// Sealed pattern/type coverage for a caller-selected binding, not a checked match body.
#[derive(Debug, Clone)]
pub struct CheckedGenericEnumPatterns {
    source: Arc<str>,
    body: CheckedGenericCopyBody,
    scrutinee: GenericCopyBindingId,
    declaration: GenericTypeDeclarationId,
    arms: Arc<[GenericEnumPatternArm]>,
    nodes: usize,
    edges: usize,
    work: usize,
}
impl CheckedGenericEnumPatterns {
    /// Original standalone match source; arm bodies remain untyped source text.
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Checked body arena owning the caller-selected binding.
    pub fn body(&self) -> &CheckedGenericCopyBody {
        &self.body
    }
    /// Original exact selected binding; no runtime read or source-site liveness proof.
    pub fn scrutinee(&self) -> &GenericCopyBinding {
        self.body
            .binding(self.scrutinee)
            .expect("sealed arena owner")
    }
    /// Exact nominal enum declaration; type arguments remain on the selected binding.
    pub fn declaration(&self) -> GenericTypeDeclarationId {
        self.declaration
    }
    /// Source-order exhaustive pattern headers and payload context types.
    pub fn arms(&self) -> &[GenericEnumPatternArm] {
        &self.arms
    }
    /// Generated payload-context nodes.
    pub fn nodes(&self) -> usize {
        self.nodes
    }
    /// Generated payload-context child edges.
    pub fn edges(&self) -> usize {
        self.edges
    }
    /// Consumed logical binding/coverage/copy/substitution work.
    pub fn work(&self) -> usize {
        self.work
    }
}
struct SyntaxGuard(MatchStatementSyntax);
impl Drop for SyntaxGuard {
    fn drop(&mut self) {
        for arm in &mut self.0.arms {
            let mut pending = std::mem::take(&mut arm.body);
            while let Some(statement) = pending.pop() {
                match statement {
                    Statement::Let { value, .. }
                    | Statement::Return {
                        value: Some(value), ..
                    }
                    | Statement::Expression(value) => value.drop_iterative(),
                    Statement::Assign { target, value, .. } => {
                        target.drop_iterative();
                        value.drop_iterative();
                    }
                    Statement::If {
                        condition,
                        then_block,
                        else_block,
                        ..
                    } => {
                        condition.drop_iterative();
                        pending.extend(then_block);
                        if let Some(block) = else_block {
                            pending.extend(block);
                        }
                    }
                    Statement::While {
                        condition, body, ..
                    } => {
                        condition.drop_iterative();
                        pending.extend(body);
                    }
                    Statement::Return { value: None, .. } => {}
                }
            }
        }
    }
}
fn charge(
    left: &mut usize,
    amount: usize,
    dimension: &'static str,
) -> Result<(), GenericEnumPatternError> {
    *left = left
        .checked_sub(amount)
        .ok_or(GenericEnumPatternError::Limit(dimension))?;
    Ok(())
}
/// Check pattern headers for a selected checked enum binding. The standalone source
/// is reparsed internally, so caller-constructed public raw match ASTs are not proofs.
/// Does not establish active source-site scope, type arm bodies, bind payload IDs,
/// prove function returns, evaluate the scrutinee or produce executable match plans.
pub fn check_generic_enum_patterns(
    source: &str,
    body: &CheckedGenericCopyBody,
    scrutinee: GenericCopyBindingId,
    limits: GenericEnumPatternLimits,
) -> Result<CheckedGenericEnumPatterns, GenericEnumPatternError> {
    let binding = body
        .binding(scrutinee)
        .ok_or(GenericEnumPatternError::ScrutineeBinding)?;
    let syntax = SyntaxGuard(
        parse_match_statement(source, limits.syntax).map_err(GenericEnumPatternError::Syntax)?,
    );
    let checked = check_pattern_headers(
        binding,
        body.signature().types(),
        body.signature().syntax().parameters(),
        &syntax.0,
        limits,
    )?;
    Ok(CheckedGenericEnumPatterns {
        source: Arc::from(source),
        body: body.clone(),
        scrutinee,
        declaration: checked.declaration,
        arms: checked.arms.into(),
        nodes: checked.nodes,
        edges: checked.edges,
        work: checked.work,
    })
}
pub(super) struct PatternCoverage {
    pub declaration: GenericTypeDeclarationId,
    pub arms: Vec<GenericEnumPatternArm>,
    pub nodes: usize,
    pub edges: usize,
    pub work: usize,
}
pub(super) fn check_pattern_headers(
    binding: &GenericCopyBinding,
    types: &super::CheckedGenericDeclarations,
    parameters: &super::GenericParameters,
    syntax: &MatchStatementSyntax,
    limits: GenericEnumPatternLimits,
) -> Result<PatternCoverage, GenericEnumPatternError> {
    let mut work = limits.max_work;
    let mut nodes = limits.max_nodes;
    let mut edges = limits.max_edges;
    charge(&mut work, 1, "pattern work")?;
    if syntax.arms.len() > limits.syntax.max_arms {
        return Err(GenericEnumPatternError::Limit("pattern arms"));
    }
    let mut text = limits.syntax.max_header_text_bytes;
    charge(&mut text, syntax.scrutinee.len(), "pattern header text")?;
    for arm in &syntax.arms {
        for bytes in [
            arm.enum_name.len(),
            arm.variant_name.len(),
            arm.binding.as_ref().map_or(0, String::len),
        ] {
            charge(&mut text, bytes, "pattern header text")?;
        }
    }

    let ty = binding
        .ty
        .expanded()
        .ok_or(GenericEnumPatternError::ScrutineeType)?;
    let ResolvedGenericTypeKind::Declared { declaration, .. } = ty.nodes()[ty.root()].kind else {
        return Err(GenericEnumPatternError::ScrutineeType);
    };
    let Some(CheckedGenericDefinition::Enum(definition)) = types.definition(declaration) else {
        return Err(GenericEnumPatternError::ScrutineeType);
    };
    charge(&mut work, syntax.scrutinee.len(), "pattern name work")?;
    if syntax.scrutinee != binding.name.as_ref() {
        return Err(GenericEnumPatternError::ScrutineeName {
            span: syntax.scrutinee_span,
        });
    }
    let mut seen = BTreeSet::new();
    let mut arms = Vec::new();
    for (arm_index, arm) in syntax.arms.iter().enumerate() {
        charge(&mut work, 1, "pattern work")?;
        for amount in [
            arm.enum_name.len(),
            arm.variant_name.len(),
            arm.binding.as_ref().map_or(0, String::len),
        ] {
            charge(&mut work, amount, "pattern name work")?;
        }
        let registry = types.environment().registry();
        if if registry.is_package_scoped() {
            registry
                .find_header_in(&arm.enum_name, parameters)
                .map(|header| header.id)
                != Some(declaration)
        } else {
            arm.enum_name != definition.syntax().qualified_name()
        } {
            return Err(GenericEnumPatternError::ForeignEnum {
                arm: arm_index,
                span: arm.span,
            });
        }
        let mut variant_index = None;
        for (index, variant) in definition.syntax().variants().iter().enumerate() {
            charge(&mut work, 1, "pattern lookup work")?;
            if variant.name == arm.variant_name {
                variant_index = Some(index);
                break;
            }
        }
        let variant_index = variant_index.ok_or(GenericEnumPatternError::UnknownVariant {
            arm: arm_index,
            span: arm.span,
        })?;
        if !seen.insert(variant_index) {
            return Err(GenericEnumPatternError::DuplicateVariant {
                arm: arm_index,
                span: arm.span,
            });
        }
        let template = definition.payload_types()[variant_index].as_ref();
        if template.is_some() != arm.binding.is_some()
            || arm.binding.is_some() != arm.binding_span.is_some()
        {
            return Err(GenericEnumPatternError::PayloadArity {
                arm: arm_index,
                span: arm.span,
            });
        }
        let payload = if let (Some(template), Some(name), Some(span)) =
            (template, arm.binding.as_ref(), arm.binding_span)
        {
            let expansion = GenericAliasExpansionLimits {
                max_nodes: limits.payload.max_nodes.min(nodes),
                max_edges: limits.payload.max_edges.min(edges),
                max_work: limits.payload.max_work.min(work),
                max_depth: limits.payload.max_depth,
                ..GenericAliasExpansionLimits::DEFAULT
            };
            let payload = ty
                .substitute_field(
                    template,
                    declaration,
                    definition.syntax().parameters(),
                    expansion,
                )
                .map_err(|error| GenericEnumPatternError::Payload {
                    arm: arm_index,
                    span: arm.span,
                    error: Box::new(error),
                })?;
            charge(&mut nodes, payload.nodes().len(), "pattern payload nodes")?;
            charge(&mut edges, payload.edges(), "pattern payload edges")?;
            charge(&mut work, payload.work(), "pattern work")?;
            Some(GenericEnumPatternPayload {
                name: Arc::from(name.as_str()),
                span,
                ty: payload,
            })
        } else {
            None
        };
        arms.push(GenericEnumPatternArm {
            variant_index,
            span: arm.span,
            payload,
        });
    }
    if seen.len() != definition.syntax().variants().len() {
        return Err(GenericEnumPatternError::MissingVariants {
            count: definition.syntax().variants().len() - seen.len(),
            span: syntax.span,
        });
    }
    Ok(PatternCoverage {
        declaration,
        arms,
        nodes: limits.max_nodes - nodes,
        edges: limits.max_edges - edges,
        work: limits.max_work - work,
    })
}
