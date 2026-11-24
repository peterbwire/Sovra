//! Same-source policy model-name references, never typed policy authorization.
use super::syntax::{self, ApplicationAuthSyntax, ApplicationDataSyntax, DataDeclarationKind};
use crate::compiler::{
    diagnostics::{Diagnostics, Span},
    lexer::{Lexer, TokenKind},
};
use std::collections::BTreeMap;

/// A policy header model reference linked to one explicit local model declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPolicyModelReference {
    /// Source auth container index.
    pub auth_index: usize,
    /// Policy index in that auth container.
    pub policy_index: usize,
    /// Model-name index in that policy header, preserving duplicates.
    pub reference_index: usize,
    /// Index into retained model/enum declarations; always a Model here.
    pub declaration_index: usize,
    /// Original policy header envelope, excluding any condition.
    pub header_span: Span,
    /// Original model declaration envelope.
    pub declaration_span: Span,
}

/// Limited declaration-name checks, separate from type/body/permission validation.
#[derive(Debug, Clone)]
pub struct PolicyModelCheck {
    /// Auth headers with every untyped condition/context gap preserved.
    pub auth: ApplicationAuthSyntax,
    /// Data headers with all nonempty bodies explicitly unparsed.
    pub data: ApplicationDataSyntax,
    /// Local model declaration references, in source/list order.
    pub resolved: Vec<LocalPolicyModelReference>,
    /// Existing E4081 missing-model diagnostics at original policy headers.
    pub diagnostics: Diagnostics,
}

/// Resolve unqualified policy model names against explicit same-source models.
/// Imports/qualified names and ambiguous data names make context unavailable.
/// Enums/records are not models, and declarations generate no query/model APIs.
/// Guard/provider/role/action/type semantics and runtime authorization stay outside
/// scope. Existing syntax and diagnostic quotas reject exhausted requests entirely.
pub fn check_local_policy_models(source: &str) -> Result<PolicyModelCheck, String> {
    let auth = syntax::parse_auth_blocks(source)?;
    let data = syntax::parse_data_declarations(source)?;
    let tokens = Lexer::new()
        .tokenize(source)
        .map_err(|_| "policy model lexical errors")?;
    let mut depth = 0usize;
    for token in &tokens {
        if depth == 0 && token.kind == TokenKind::Keyword("use") {
            return Err("policy model imports require module context; context unavailable".into());
        }
        match token.kind {
            TokenKind::Punctuation('{') => depth += 1,
            TokenKind::Punctuation('}') => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let mut names = BTreeMap::new();
    for (index, declaration) in data.declarations.iter().enumerate() {
        if names.insert(declaration.name.as_str(), index).is_some() {
            return Err("ambiguous data declaration names; context unavailable".into());
        }
    }
    let mut resolved = Vec::new();
    let mut diagnostics = Diagnostics::new();
    let mut bytes = 0usize;
    for (auth_index, block) in auth.blocks.iter().enumerate() {
        for (policy_index, policy) in block.policies.iter().enumerate() {
            for (reference_index, model) in policy.header.models.iter().enumerate() {
                if model.contains('.') {
                    return Err(
                        "qualified policy model requires module context; context unavailable"
                            .into(),
                    );
                }
                let declaration = names
                    .get(model.as_str())
                    .copied()
                    .filter(|index| data.declarations[*index].kind == DataDeclarationKind::Model);
                let mut local = Diagnostics::new();
                super::super::validate_policy_model_target(
                    model,
                    declaration.is_some(),
                    &mut local,
                );
                for mut diagnostic in local.items {
                    if diagnostics.items.len() == super::MAX_ENTRY_PATH_DIAGNOSTICS {
                        return Err("policy model diagnostic count limit exceeded".into());
                    }
                    bytes = bytes
                        .checked_add(diagnostic.message.len())
                        .filter(|sum| *sum <= super::MAX_ENTRY_PATH_DIAGNOSTIC_BYTES)
                        .ok_or("policy model diagnostic byte limit exceeded")?;
                    diagnostic.span = policy.header_span;
                    diagnostics.push(diagnostic);
                }
                if let Some(declaration_index) = declaration {
                    resolved.push(LocalPolicyModelReference {
                        auth_index,
                        policy_index,
                        reference_index,
                        declaration_index,
                        header_span: policy.header_span,
                        declaration_span: data.declarations[declaration_index].span,
                    });
                }
            }
        }
    }
    Ok(PolicyModelCheck {
        auth,
        data,
        resolved,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicit_models_resolve_and_conditions_bodies_stay_unparsed() {
        let source = "model Item{value:Unknown}\nenum Role{user}\nauth session{\nallow user to read on [Item, Role, Missing] where false\n}";
        let checked = check_local_policy_models(source).unwrap();
        assert_eq!(checked.resolved.len(), 1);
        assert_eq!(
            checked
                .diagnostics
                .items
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            ["E4081", "E4081"]
        );
        assert_eq!(
            checked.data.declarations[checked.resolved[0].declaration_index].name,
            "Item"
        );
        assert!(!checked.auth.unparsed_ranges.is_empty());
        let record = check_local_policy_models(
            "struct Item{}\nauth session{\nallow user to read on Item\n}",
        )
        .unwrap();
        assert_eq!(record.diagnostics.items[0].code, "E4081");
        assert!(record.resolved.is_empty());

        assert!(checked.data.declarations[0].unparsed_body.is_some());
        for source in [
            "use app.models;",
            "model Item{}\nmodel Item{}",
            "model Item{}\nenum Item{}",
            "auth session{\nallow user to read on app.Item\n}",
        ] {
            assert!(check_local_policy_models(source)
                .unwrap_err()
                .contains("context unavailable"));
        }
    }
    #[test]
    fn policy_reference_diagnostic_limits_return_no_partial_result() {
        let exact = format!(
            "auth session{{\n{}}}",
            "allow user to read on Missing\n".repeat(super::super::MAX_ENTRY_PATH_DIAGNOSTICS)
        );
        assert_eq!(
            check_local_policy_models(&exact)
                .unwrap()
                .diagnostics
                .items
                .len(),
            super::super::MAX_ENTRY_PATH_DIAGNOSTICS
        );
        let over = format!(
            "auth session{{\n{}}}",
            "allow user to read on Missing\n".repeat(super::super::MAX_ENTRY_PATH_DIAGNOSTICS + 1)
        );
        assert!(check_local_policy_models(&over)
            .unwrap_err()
            .contains("count limit"));
        let models = vec!["x".repeat(1000); 1024].join(",");
        let source = format!("auth session{{\nallow user to read on [{models}]\n}}");
        assert!(source.len() <= syntax::MAX_SOURCE_BYTES);
        assert!(check_local_policy_models(&source)
            .unwrap_err()
            .contains("byte limit"));
    }
}
