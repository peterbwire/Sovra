//! Bounded opt-in projection of compiler-owned standalone evidence.

use std::{collections::HashMap, fmt::Write as _};

use super::check_report::push_string;
use super::checked::{
    CheckError, CheckedUnit, DeclarationKind, EvidenceError, EvidenceLimits, FlowEvidenceLimits,
    NodeId, SemanticEvidenceLimits, UseKind, UseTarget,
};
use super::diagnostics::Diagnostics;
use super::lexer::{Lexer, TokenKind};
mod locals;

const MAX_BYTES: usize = 512 * 1024;
const MAX_VISITS: usize = 8192;

/// Preserve ordinary check outcomes; optional collection failure exposes no facts.
pub(crate) fn check(target: &str, source: &str) -> Result<String, Diagnostics> {
    let nodes = super::resources::DEFAULT_AST_NODES;
    check_with_limits(
        target,
        source,
        SemanticEvidenceLimits {
            max_facts: nodes,
            max_types: MAX_VISITS,
            max_name_bytes: MAX_BYTES,
        },
        FlowEvidenceLimits {
            max_declarations: nodes,
            max_uses: nodes,
            max_coercions: nodes,
            max_name_bytes: MAX_BYTES,
        },
    )
}

fn check_with_limits(
    target: &str,
    source: &str,
    types: SemanticEvidenceLimits,
    flow: FlowEvidenceLimits,
) -> Result<String, Diagnostics> {
    let result = CheckedUnit::from_source_with_flow_evidence(
        source,
        EvidenceLimits {
            max_nodes: super::resources::DEFAULT_AST_NODES,
            max_source_bytes: super::resources::FrontendLimits::DEFAULT.max_source_bytes,
        },
        types,
        flow,
    );
    match result {
        Ok(unit) => Ok(render(target, &unit)),
        Err(EvidenceError::Check(CheckError::Diagnostics(errors))) => Err(errors),
        Err(_) => {
            // Preserve ordinary parse/semantic/admission failures, never partial facts.
            let parsed = super::parser::Parser::new().parse_source(source)?;
            super::semantic::SemanticAnalyzer::new().analyze(&parsed)?;
            let mut output = base(target);
            output.pop();
            output.push_str(",\"symbols\":{\"declarations\":[],\"references\":[],\"locals\":[],\"local_references\":[],\"truncated\":true,\"unavailable\":true}}");
            Ok(output)
        }
    }
}

fn base(target: &str) -> String {
    super::check_report::render(
        target,
        Some(super::check_report::CheckKind::Source),
        &Diagnostics::new(),
    )
}

fn render(target: &str, unit: &CheckedUnit) -> String {
    let source = unit.source().expect("source-backed IDE snapshot");
    let tokens = Lexer::new()
        .tokenize(source)
        .expect("checked original source tokenizes");
    let flow = unit.semantic_evidence().unwrap().flow().unwrap();
    let mut output = base(target);
    output.pop();
    output.push_str(",\"symbols\":{\"declarations\":[");
    let mut emitted: HashMap<NodeId, usize> = HashMap::new();
    let mut truncated = false;
    for (visited, declaration) in flow
        .declarations()
        .iter()
        .filter(|d| d.kind == DeclarationKind::Function)
        .enumerate()
    {
        if visited >= MAX_VISITS {
            truncated = true;
            break;
        }
        let node = &unit.nodes()[declaration.node.ordinal()];
        let Ok(index) = tokens.binary_search_by_key(&node.span.start, |token| token.span.start)
        else {
            truncated = true;
            continue;
        };
        let Some(name_token) = tokens.get(index + 1) else {
            truncated = true;
            continue;
        };
        if !matches!(&name_token.kind, TokenKind::Identifier(name) if name == declaration.name.as_ref())
        {
            truncated = true;
            continue;
        }
        let module = node.parent.and_then(|parent| {
            let span = unit.nodes()[parent.ordinal()].span;
            let index = tokens
                .binary_search_by_key(&span.start, |token| token.span.start)
                .ok()?;
            match &tokens.get(index + 1)?.kind {
                TokenKind::Identifier(name) => Some(name.as_str()),
                _ => None,
            }
        });
        if declaration.name.len() + module.map_or(0, str::len) + 2 > 4096 {
            truncated = true;
            continue;
        }
        let name = module.map_or_else(
            || declaration.name.to_string(),
            |module| format!("{module}::{}", declaration.name),
        );
        let end = tokens[index..]
            .iter()
            .find(|t| t.kind == TokenKind::Punctuation('{'))
            .unwrap()
            .span
            .start;
        let shortened = end - node.span.start > 4096;
        let mut signature_end = if shortened {
            node.span.start + 4092
        } else {
            end
        };
        while !source.is_char_boundary(signature_end) {
            signature_end -= 1;
        }
        let signature = source[node.span.start..signature_end].trim_end();
        let checkpoint = output.len();
        if !emitted.is_empty() {
            output.push(',');
        }
        output.push_str("{\"name\":");
        push_string(&mut output, &name);
        output.push_str(",\"signature\":");
        if shortened {
            truncated = true;
            push_string(&mut output, &format!("{signature} …"));
        } else {
            push_string(&mut output, signature);
        }
        let _ = write!(
            output,
            ",\"start\":{},\"end\":{},\"name_start\":{},\"name_end\":{}}}",
            node.span.start, node.span.end, name_token.span.start, name_token.span.end
        );
        if output.len() > MAX_BYTES - 256 {
            output.truncate(checkpoint);
            truncated = true;
            break;
        }
        emitted.insert(declaration.node, emitted.len());
    }
    output.push_str("],\"references\":[");
    let mut count = 0;
    for (visited, usage) in flow
        .uses()
        .iter()
        .filter(|u| u.kind == UseKind::Call)
        .enumerate()
    {
        if visited >= MAX_VISITS {
            truncated = true;
            break;
        }
        let (definition, builtin) = match usage.target {
            UseTarget::Declaration(id) => match emitted.get(&id) {
                Some(index) => (Some(*index), None),
                _ => {
                    truncated = true;
                    continue;
                }
            },
            UseTarget::Builtin(name) => (None, super::stdlib::lookup(name)),
        };
        let span = unit.nodes()[usage.node.ordinal()].span;
        let checkpoint = output.len();
        if count > 0 {
            output.push(',');
        }
        let _ = write!(
            output,
            "{{\"start\":{},\"end\":{},\"definition\":",
            span.start, span.end
        );
        if let Some(definition) = definition {
            let _ = write!(output, "{definition}");
        } else {
            output.push_str("null");
        }
        if let Some(function) = builtin {
            output.push_str(",\"signature\":");
            push_string(
                &mut output,
                &format!(
                    "fn {}({}) -> {}",
                    function.name,
                    function.parameters.join(", "),
                    function.return_type
                ),
            );
        }
        output.push('}');
        if output.len() > MAX_BYTES - 256 {
            output.truncate(checkpoint);
            truncated = true;
            break;
        }
        count += 1;
    }
    output.push(']');
    let local = locals::collect(unit, &tokens, MAX_BYTES.saturating_sub(output.len() + 128));
    truncated |= local.truncated;
    output.push_str(",\"locals\":[");
    output.push_str(&local.declarations);
    output.push_str("],\"local_references\":[");
    output.push_str(&local.references);
    let _ = write!(output, "],\"truncated\":{truncated}}}}}");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_exhaustion_preserves_source_success_but_never_partial_facts() {
        let zero = SemanticEvidenceLimits {
            max_facts: 0,
            max_types: 0,
            max_name_bytes: 0,
        };
        let flow = FlowEvidenceLimits {
            max_declarations: 0,
            max_uses: 0,
            max_coercions: 0,
            max_name_bytes: 0,
        };
        let report = check_with_limits(
            "source.svr",
            "fn main() { let x = 1; print(x); }",
            zero,
            flow,
        )
        .unwrap();
        assert!(report.contains("\"success\":true"));
        assert!(report.contains("\"unavailable\":true"));
        assert!(report.contains("\"locals\":[]"));
        assert!(
            check_with_limits("source.svr", "fn main() { print(missing); }", zero, flow)
                .unwrap_err()
                .items
                .iter()
                .any(|d| d.code == "E3001")
        );
    }
    #[test]
    fn sliced_arrays_retain_dependent_inferred_ide_types() {
        let report = check("slice.svr", "struct Point{x:Int} fn main(){let floats=std::array_slice([3.0],0,1);let points=std::array_slice([Point{x:7}],0,1);print(floats);print(points);}").unwrap();
        assert!(report.contains("Array<Float>"), "{report}");
        assert!(report.contains("Array<Point>"), "{report}");
        assert!(!report.contains("inferred type unavailable"), "{report}");
    }
    #[test]
    fn grown_arrays_retain_first_directed_ide_types() {
        let report = check("growth.svr", "struct Point{x:Int} fn main(){let floats=std::array_append([1.0],2);let rows=std::array_concat([[1.0]],[[2]]);let points=std::array_append([Point{x:1}],Point{x:2});print(floats);print(rows);print(points);}").unwrap();
        for ty in ["Array<Float>", "Array<Array<Float>>", "Array<Point>"] {
            assert!(report.contains(ty), "{report}");
        }
        assert!(!report.contains("inferred type unavailable"), "{report}");
    }
}
