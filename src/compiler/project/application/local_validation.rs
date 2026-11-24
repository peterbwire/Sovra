//! Core semantics for self-contained ordinary functions beside partial application syntax.
use super::syntax::{
    self, ApplicationSyntax, CallableKind, CombinedApplicationSyntax, StructuredApplicationSyntax,
};
use crate::compiler::{ast::Program, diagnostics::Diagnostics, semantic::SemanticAnalyzer};

/// Local function semantics, separate from application binding/coverage validation.
#[derive(Debug, Clone)]
pub struct LocalFunctionCheck {
    /// Original combined syntax, including unknown application entry ranges.
    pub syntax: CombinedApplicationSyntax,
    /// Number of ordinary function bodies submitted to core semantic checking.
    pub checked_functions: usize,
    /// Core compiler diagnostics for local aliases, records and ordinary functions.
    /// Empty diagnostics establish only this limited semantic scope.
    pub diagnostics: Diagnostics,
}

/// Check self-contained ordinary functions with the executable semantic analyzer.
///
/// Reuses core annotation, name/type, call, field and return-path rules. Imports,
/// services and tasks require other context and explicitly make this request
/// unavailable; no imported or task/service bodies are silently treated as local
/// functions. Entry bindings and unknown entry members remain unvalidated.
/// Existing source/token/AST/diagnostic limits apply. No execution or writes occur.
pub fn check_local_functions(source: &str) -> Result<LocalFunctionCheck, String> {
    let syntax = syntax::parse_combined(source)?;
    let (checked_functions, diagnostics) = check_functions(&syntax.declarations)?;
    Ok(LocalFunctionCheck {
        syntax,
        checked_functions,
        diagnostics,
    })
}

/// Local ordinary function semantics beside explicitly opaque application headers.
#[derive(Debug, Clone)]
pub struct StructuredLocalFunctionCheck {
    /// Structured partial syntax; field/UI/enum/guard coverage gaps stay intact.
    pub syntax: StructuredApplicationSyntax,
    /// Number of ordinary function bodies submitted to core semantics.
    pub checked_functions: usize,
    /// Core executable diagnostics, never model/render/policy validity.
    pub diagnostics: Diagnostics,
}

/// Check self-contained ordinary functions beside structured application headers.
/// Imports/services/tasks and executable references to excluded header spellings
/// explicitly require later contextual checking. This conservative name guard also
/// refuses shadowed spellings rather than selecting unapproved namespace semantics.
/// Model/enum fields, UI bodies and policy guards remain unvalidated source gaps.
/// The existing local-function API and default project/CLI behavior are unchanged.
pub fn check_structured_local_functions(
    source: &str,
) -> Result<StructuredLocalFunctionCheck, String> {
    let syntax = syntax::parse_structured(source)?;
    let declarations = &syntax.base.declarations;
    let types = &declarations.base.base.types;
    let names: std::collections::HashSet<_> = syntax
        .data
        .declarations
        .iter()
        .map(|d| d.name.as_str())
        .chain(syntax.render.declarations.iter().map(|d| d.name.as_str()))
        .chain(syntax.auth.blocks.iter().map(|block| block.name.as_str()))
        .collect();
    if types
        .aliases
        .iter()
        .any(|alias| names.contains(alias.target.as_str()))
        || types
            .records
            .iter()
            .flat_map(|record| &record.fields)
            .any(|field| names.contains(field.type_name.as_str()))
    {
        return Err(
            "local executable types reference excluded declaration context; context unavailable"
                .into(),
        );
    }
    let mut spans: Vec<_> = declarations
        .base
        .base
        .callables
        .iter()
        .map(|callable| callable.span)
        .collect();
    spans.sort_by_key(|span| span.start);
    let tokens = crate::compiler::lexer::Lexer::new()
        .tokenize(source)
        .map_err(|_| "structured local function lexical errors")?;
    for token in tokens {
        if let crate::compiler::lexer::TokenKind::Identifier(name) = &token.kind {
            if names.contains(name.as_str()) {
                let index = spans.partition_point(|span| span.end <= token.span.start);
                if spans.get(index).is_some_and(|span| {
                    span.start <= token.span.start && token.span.end <= span.end
                }) {
                    return Err(
                        "local function references excluded declaration spelling; context unavailable".into(),
                    );
                }
            }
        }
    }
    let (checked_functions, diagnostics) = check_functions(declarations)?;
    Ok(StructuredLocalFunctionCheck {
        syntax,
        checked_functions,
        diagnostics,
    })
}

fn check_functions(declarations: &ApplicationSyntax) -> Result<(usize, Diagnostics), String> {
    if !declarations.imports.is_empty() {
        return Err(
            "local function validation requires resolved application imports; context unavailable"
                .into(),
        );
    }
    if !declarations.base.services.is_empty() {
        return Err(
            "local function validation requires service contract context; context unavailable"
                .into(),
        );
    }
    let callables = &declarations.base.base.callables;
    if callables
        .iter()
        .any(|callable| callable.kind != CallableKind::Function)
    {
        return Err(
            "local function validation excludes task/service callables; context unavailable".into(),
        );
    }
    let types = &declarations.base.base.types;
    let program = Program {
        functions: callables
            .iter()
            .map(|callable| callable.function.clone())
            .collect(),
        modules: Vec::new(),
        type_declarations: types.aliases.clone(),
        struct_declarations: types.records.clone(),
    };
    let checked_functions = program.functions.len();
    let diagnostics = match SemanticAnalyzer::new().analyze(&program) {
        Ok(_) => Diagnostics::new(),
        Err(diagnostics) => diagnostics,
    };
    Ok((checked_functions, diagnostics))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_bodies_use_core_rules_without_validating_entry_bindings() {
        let source = "app Demo{\nroute BAD \"relative\" -> missing\ntitle: \"future\"\n}\nfn main(){let value:Int=true;}";
        let checked = check_local_functions(source).unwrap();
        assert_eq!(checked.checked_functions, 1);
        assert!(!checked.diagnostics.is_empty());
        assert_eq!(checked.syntax.unparsed_ranges.len(), 1);
        let diagnostic = &checked.diagnostics.items[0];
        assert!(diagnostic.span.start >= source.find("fn main").unwrap());
        let checked = check_local_functions(
            "app Demo{}\nfn f(x:Int)->Float{return x;} fn main(){print(f(1));}",
        )
        .unwrap();
        assert_eq!(checked.checked_functions, 2);
        assert!(checked.diagnostics.is_empty());
    }
    #[test]
    fn missing_context_and_invalid_syntax_never_become_empty_successful_diagnostics() {
        for source in [
            "use app.library; fn main(){}",
            "service Api{fn f();} fn main(){}",
            "task job(){} fn main(){}",
            "fn main(){let x=;}",
        ] {
            assert!(check_local_functions(source).is_err(), "{source}");
        }
        let entry_only = check_local_functions("app Demo{}").unwrap();
        assert_eq!(entry_only.checked_functions, 0);
    }
}
