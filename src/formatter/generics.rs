//! Syntax-only generic formatting: no linking, type checking, execution or grants.
use super::{equivalence, files, layout, Failure};
use crate::compiler::{
    ast::{Function, Program, Statement},
    diagnostics::Span,
    lexer::{Lexer, Token, TokenKind},
    parser::Parser,
    resources::FrontendLimits,
    v2::*,
};
use std::collections::BTreeSet;

#[derive(Debug, PartialEq)]
struct Body(Vec<Statement>);
impl Drop for Body {
    fn drop(&mut self) {
        crate::compiler::v2::generic_copy_bodies::drop_body(std::mem::take(&mut self.0));
    }
}
fn body(value: Vec<Statement>) -> Result<Body, Failure> {
    let mut program = Program {
        functions: vec![Function {
            name: String::new(),
            is_exported: false,
            parameters: vec![],
            return_type: None,
            return_type_span: None,
            body: value,
            span: Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            },
        }],
        modules: vec![],
        type_declarations: vec![],
        struct_declarations: vec![],
    };
    if crate::compiler::check_program_depth(&program).is_err() {
        program.drop_iterative();
        return Err("generic formatter structural depth exceeded"
            .to_owned()
            .into());
    }
    let result = Body(std::mem::take(&mut program.functions[0].body));
    Ok(result)
}
#[derive(Debug, PartialEq)]
struct Proof {
    bodies: Vec<Body>,
    // Token-index coordinates retain erased generic calls, matches and postfix sites.
    overlays: Vec<Vec<usize>>,
    before: BTreeSet<usize>,
    after: BTreeSet<usize>,
}
fn index(tokens: &[Token], offset: usize) -> usize {
    tokens.partition_point(|t| t.span.start < offset)
}
fn syntax(source: &str, tokens: &[Token]) -> Result<Proof, Failure> {
    crate::compiler::check_nesting(tokens, true)
        .map_err(|_| Failure::Message("generic formatter delimiter depth exceeded".into()))?;
    let mut at = 0;
    while tokens[at].kind == TokenKind::Keyword("use") {
        if !matches!(&tokens[at..], [_,a,s,b,e,..] if matches!(a.kind,TokenKind::Identifier(_)) && s.kind==TokenKind::Operator("::") && matches!(b.kind,TokenKind::Identifier(_)) && e.kind==TokenKind::Punctuation(';'))
        {
            return Err("generic formatter expects use dependency::module;"
                .to_owned()
                .into());
        }
        at += 5;
    }
    let end = tokens.len() - 1;
    let range = if tokens[at].kind == TokenKind::Keyword("mod") {
        if !matches!(&tokens[at..], [_,name,open,..] if matches!(name.kind,TokenKind::Identifier(_)) && open.kind==TokenKind::Punctuation('{'))
            || end < at + 4
            || tokens[end - 1].kind != TokenKind::Punctuation('}')
        {
            return Err("generic formatter requires one complete module wrapper"
                .to_owned()
                .into());
        }
        tokens[at + 2].span.end..tokens[end - 1].span.start
    } else {
        tokens[at].span.start..source.len()
    };
    let mut limits = GenericSourceSyntaxLimits::DEFAULT;
    let declarations = collect_package_fragment(source, range, &mut limits)
        .map_err(|e| Failure::Message(e.to_string()))?;
    let mut proof = Proof {
        bodies: vec![],
        overlays: vec![],
        before: BTreeSet::new(),
        after: BTreeSet::new(),
    };
    for declaration in declarations {
        proof.before.insert(index(tokens, declaration.span().start));
        let GenericSourceDeclarationKind::Function(function) = declaration.kind() else {
            continue;
        };
        let local = function.body_source();
        let local_tokens = Lexer::new().tokenize_propagation(local, FrontendLimits::DEFAULT)?;
        let (statements, used, calls, mut matches, sites) =
            Parser::new().parse_propagation_block_prefix(&local_tokens, 64, 4096, 128)?;
        let base = declaration.span().start + function.body_span().start;
        let mark = |statements: &[Statement], proof: &mut Proof| {
            let mut before = BTreeSet::new();
            let mut after = BTreeSet::new();
            for statement in statements {
                super::v2::core(statement, &local_tokens, &mut before, &mut after);
            }
            proof
                .before
                .extend(before.into_iter().map(|n| index(tokens, base + n)));
            proof
                .after
                .extend(after.into_iter().map(|n| index(tokens, base + n)));
        };
        // Own parsed trees immediately so every refusal uses iterative destruction.
        let mut statements = body(statements)?;
        mark(&statements.0, &mut proof);
        equivalence::statements(&mut statements.0);
        proof.bodies.push(statements);
        if used != local_tokens.len() - 1 {
            return Err("generic formatter requires a complete function body"
                .to_owned()
                .into());
        }
        let mut overlay = vec![calls.len(), matches.0.len(), sites.len()];
        for (start, span) in calls {
            overlay.extend([
                index(&local_tokens, start),
                index(&local_tokens, span.start),
                index(&local_tokens, span.end),
            ]);
        }
        for header in &mut matches.0 {
            overlay.extend([
                index(&local_tokens, header.span.start),
                index(&local_tokens, header.span.end),
                index(&local_tokens, header.scrutinee_span.start),
                index(&local_tokens, header.scrutinee_span.end),
                header.arms.len(),
            ]);
            for arm in &mut header.arms {
                overlay.extend([
                    index(&local_tokens, arm.span.start),
                    index(&local_tokens, arm.span.end),
                ]);
                let mut statements = body(std::mem::take(&mut arm.body))?;
                mark(&statements.0, &mut proof);
                equivalence::statements(&mut statements.0);
                proof.bodies.push(statements);
            }
        }
        for site in sites {
            overlay.extend([
                index(&local_tokens, site.operand.start),
                index(&local_tokens, site.operand.end),
                index(&local_tokens, site.question.start),
                index(&local_tokens, site.expression.start),
                index(&local_tokens, site.expression.end),
            ]);
        }
        proof.overlays.push(overlay);
    }
    Ok(proof)
}
fn tokens(source: &str) -> Result<Vec<Token>, Failure> {
    Ok(Lexer::new().tokenize_propagation(
        source,
        FrontendLimits {
            max_source_bytes: files::INPUT_LIMIT,
            ..FrontendLimits::DEFAULT
        },
    )?)
}
pub(super) fn format_source(source: &str) -> Result<String, Failure> {
    if source.len() > files::INPUT_LIMIT {
        return Err("formatter source exceeds 1 MiB".to_owned().into());
    }
    let original = tokens(source)?;
    let proof = syntax(source, &original)?;
    let render = |source: &str, tokens: &[Token], proof: &Proof| {
        let before = proof.before.iter().map(|&i| tokens[i].span.start).collect();
        let after = proof
            .after
            .iter()
            .filter_map(|&i| i.checked_sub(1).map(|j| tokens[j].span.end))
            .collect();
        layout::render_marked(source, tokens, &before, &after)
    };
    let output = render(source, &original, &proof)?;
    let formatted = tokens(&output)?;
    if original.len() != formatted.len()
        || original.iter().zip(&formatted).any(|(a, b)| {
            a.kind != b.kind || source[a.span.start..a.span.end] != output[b.span.start..b.span.end]
        })
    {
        return Err("generic formatter token spelling/order changed"
            .to_owned()
            .into());
    }
    if layout::comments(source, &original)? != layout::comments(&output, &formatted)? {
        return Err("generic formatter comment content/attachment changed"
            .to_owned()
            .into());
    }
    let formatted_proof = syntax(&output, &formatted)?;
    if proof != formatted_proof {
        return Err("generic formatter parsed structure changed"
            .to_owned()
            .into());
    }
    if render(&output, &formatted, &formatted_proof)? != output {
        return Err("generic formatter output is not idempotent"
            .to_owned()
            .into());
    }
    Ok(output)
}
