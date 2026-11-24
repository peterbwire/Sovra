//! Partial application syntax: retained type, callable and service nodes.
//!
//! This API never establishes full application parsing, type checking or execution.

mod auth;
mod structured;
pub use structured::{parse_structured, StructuredApplicationSyntax};
mod render;
pub use render::{
    parse_render_declarations, ApplicationRenderSyntax, RenderDeclarationKind,
    RenderDeclarationSyntax, RenderParameterSyntax, MAX_RENDER_DECLARATIONS, MAX_RENDER_PARAMETERS,
};
mod data;
pub use data::{
    parse_data_declarations, ApplicationDataSyntax, DataDeclarationKind, DataDeclarationSyntax,
    MAX_DATA_DECLARATIONS,
};
mod auth_combined;
pub use auth_combined::{parse_with_auth, AuthCombinedSyntax};
mod callables;
pub use auth::{
    parse_auth_blocks, ApplicationAuthSyntax, AuthBlockSyntax, AuthPolicyHeaderSyntax,
    PolicyConditionSyntax, MAX_AUTH_BLOCKS, MAX_AUTH_ITEMS, MAX_AUTH_MEMBERS,
};
mod combined;
pub use combined::{parse_combined, CombinedApplicationSyntax};
mod entries;
pub use entries::{
    parse_entry_blocks, AppEntrySyntax, AppMemberKind, AppMemberSyntax, ApplicationEntrySyntax,
    MAX_APP_BLOCKS, MAX_APP_MEMBERS,
};
mod imports;
pub use imports::{parse_file, ApplicationSyntax, ImportSyntax, MAX_IMPORTS};
mod services;
pub use callables::{
    parse_callables, ApplicationCallableSyntax, CallableKind, CallableSyntax, MAX_CALLABLES,
    MAX_CALLABLE_TOKENS,
};
pub use services::{
    parse_services, ApplicationServiceSyntax, ServiceOperationSyntax, ServiceSignatureSyntax,
    ServiceSyntax, MAX_SERVICES, MAX_SERVICE_OPERATIONS,
};

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::compiler::ast::{Program, StructDeclaration, TypeDeclaration};
use crate::compiler::diagnostics::Span;
use crate::compiler::lexer::{Lexer, Token, TokenKind};

/// Source ceiling for optional retained application type syntax.
pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
/// Combined alias/record declaration ceiling for retained syntax.
pub const MAX_DECLARATIONS: usize = 4096;
/// Cumulative record field ceiling for retained syntax.
pub const MAX_FIELDS: usize = 8192;

/// Source-owned alias/record syntax plus explicit gaps in parsing coverage.
///
/// Nodes use the executable parser's grammar and original UTF-8 byte spans.
/// Unparsed ranges are token envelopes, not syntax or semantic assertions.
/// Even an empty gap list establishes only type-declaration parsing, never a
/// successful project check. Canonical module/type identities remain separate.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationTypeSyntax {
    /// Original UTF-8 source length, including comments and trailing whitespace.
    pub source_bytes: usize,
    /// Parsed alias declarations, in alias declaration order.
    pub aliases: Vec<TypeDeclaration>,
    /// Parsed record declarations, in record declaration order.
    pub records: Vec<StructDeclaration>,
    /// Nontrivia token runs outside the recognized type declarations.
    pub unparsed_ranges: Vec<Span>,
}

/// Parse supported type declarations without treating other declarations as checked.
///
/// Unknown function/service/app forms remain explicit unparsed ranges. Malformed
/// recognized types and lexical/structural-depth failures reject the request.
/// Unresolved names and other semantic errors do not invalidate syntax nodes.
/// No imports, filesystem reads, source writes or execution occur.
pub fn parse(source: &str) -> Result<Arc<ApplicationTypeSyntax>, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err("application type syntax exceeds 1 MiB source limit".into());
    }
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "application type syntax contains lexical errors".to_owned())?;
    super::check_nesting(&tokens)?;
    let parsed = super::super::application_types::Aliases::collect_in_module(&tokens, "<syntax>")?;
    parsed
        .syntax
        .ok_or_else(|| "application type syntax metadata limit exceeded".into())
}

pub(in crate::compiler::project) fn retain(
    program: Program,
    tokens: &[Token],
    ends: &BTreeMap<usize, usize>,
) -> Option<Arc<ApplicationTypeSyntax>> {
    let source_bytes = tokens.last()?.span.end;
    let declarations = program
        .type_declarations
        .len()
        .checked_add(program.struct_declarations.len())?;
    let fields = program
        .struct_declarations
        .iter()
        .try_fold(0usize, |used, record| used.checked_add(record.fields.len()))?;
    if source_bytes > MAX_SOURCE_BYTES || declarations > MAX_DECLARATIONS || fields > MAX_FIELDS {
        return None;
    }
    let mut unparsed_ranges = Vec::new();
    let mut position = 0;
    while position < tokens.len() && tokens[position].kind != TokenKind::Eof {
        if let Some(end) = ends.get(&position) {
            position = *end;
            continue;
        }
        let start = tokens[position].span;
        position += 1;
        while position < tokens.len()
            && tokens[position].kind != TokenKind::Eof
            && !ends.contains_key(&position)
        {
            position += 1;
        }
        unparsed_ranges.push(Span {
            end: tokens[position - 1].span.end,
            ..start
        });
    }
    Some(Arc::new(ApplicationTypeSyntax {
        source_bytes,
        aliases: program.type_declarations,
        records: program.struct_declarations,
        unparsed_ranges,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_original_nodes_and_explicit_unparsed_ranges() {
        let source = "// λ header\r\nuse app.library;\r\ntype Amount = Int;\r\nfn before(){}\r\nexport struct Point { x: Amount, y: Float }\r\napp Demo {}";
        let syntax = parse(source).unwrap();
        assert_eq!(syntax.source_bytes, source.len());
        assert_eq!(syntax.aliases.len(), 1);
        assert_eq!(syntax.records.len(), 1);
        assert_eq!(syntax.aliases[0].name, "Amount");
        assert_eq!(syntax.records[0].fields.len(), 2);
        assert!(syntax.records[0].is_exported);
        assert!(
            source[syntax.aliases[0].span.start..syntax.aliases[0].span.end]
                .starts_with("type Amount")
        );
        assert_eq!(
            syntax
                .unparsed_ranges
                .iter()
                .map(|span| &source[span.start..span.end])
                .collect::<Vec<_>>(),
            ["use app.library;", "fn before(){}", "app Demo {}"]
        );
        assert!(syntax
            .unparsed_ranges
            .windows(2)
            .all(|pair| pair[0].end <= pair[1].start));
    }
    #[test]
    fn syntax_never_asserts_semantic_validity_or_nested_declaration_coverage() {
        let syntax = parse(
            "type Missing = Undefined; struct Bad { x: Undefined } fn f(){struct Nested{x:Int}} ",
        )
        .unwrap();
        assert_eq!(syntax.aliases.len(), 1);
        assert_eq!(syntax.records.len(), 1);
        assert_eq!(syntax.unparsed_ranges.len(), 1);
        assert!(parse("type Broken = ;").is_err());
        assert!(parse("struct Broken { field }").is_err());
        assert!(parse("fn bad(){\"unterminated}").is_err());
        assert!(parse(&" ".repeat(MAX_SOURCE_BYTES + 1)).is_err());
        assert!(parse("// comments only\n")
            .unwrap()
            .unparsed_ranges
            .is_empty());
    }
    #[test]
    fn exact_declaration_and_field_limits_are_optional_not_checker_fallbacks() {
        let aliases = (0..MAX_DECLARATIONS)
            .map(|n| format!("type T{n} = Int;\n"))
            .collect::<String>();
        assert_eq!(parse(&aliases).unwrap().aliases.len(), MAX_DECLARATIONS);
        assert!(parse(&(aliases + "type Extra = Int;")).is_err());
        let fields = (0..MAX_FIELDS)
            .map(|n| format!("f{n}: Int,"))
            .collect::<String>();
        assert_eq!(
            parse(&format!("struct Large {{{fields}}}"))
                .unwrap()
                .records[0]
                .fields
                .len(),
            MAX_FIELDS
        );
        assert!(parse(&format!("struct Large {{{fields} extra: Int}}")).is_err());
    }
}
