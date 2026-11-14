//! Optional bounded root-function navigation from checked source-owned identities.
use super::*;
use crate::compiler::{
    check_report::push_string,
    lexer::{Lexer, TokenKind},
};
use std::fmt::Write;
mod bindings;
mod nominals;
/// Independent admission for optional IDE projection; exhaustion does not undo checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramSymbolLimits {
    /// Aggregate declarations and references across all selected symbol tables.
    pub max_items: usize,
    /// Complete serialized symbol envelope, including escaped source signatures.
    pub max_bytes: usize,
}
impl ProgramSymbolLimits {
    /// Matches the existing strict standalone language-server envelope ceilings.
    pub const DEFAULT: Self = Self {
        max_items: 8192,
        max_bytes: 512 * 1024,
    };
}
const UNAVAILABLE: &str = "{\"declarations\":[],\"references\":[],\"locals\":[],\"local_references\":[],\"truncated\":true,\"unavailable\":true}";
fn identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
impl CheckedProgramSource {
    /// Emit the existing standalone symbol envelope for checked root functions only.
    /// Any projection refusal returns an explicitly unavailable empty envelope;
    /// no unchecked syntax, constructor/standard target or partial table is exposed.
    /// Even a caller-selected zero byte cap can return the fixed refusal envelope.
    pub fn render_function_symbols(&self, limits: ProgramSymbolLimits) -> String {
        self.function_symbols(limits, false)
            .unwrap_or_else(|| UNAVAILABLE.into())
    }
    /// Project root functions and scope-owned parameter/local/match-payload facts.
    /// All tables share the same optional quotas and unavailable-on-refusal policy.
    pub fn render_symbols(&self, limits: ProgramSymbolLimits) -> String {
        self.function_symbols(limits, true)
            .unwrap_or_else(|| UNAVAILABLE.into())
    }
    fn function_symbols(
        &self,
        limits: ProgramSymbolLimits,
        include_bindings: bool,
    ) -> Option<String> {
        let count = self
            .functions()
            .len()
            .checked_add(self.call_links().len())?;
        let count = if include_bindings {
            self.functions().iter().try_fold(count, |count, function| {
                count
                    .checked_add(function.bindings().declarations().len())?
                    .checked_add(function.bindings().uses().len())
            })?
        } else {
            count
        };
        let count = if include_bindings {
            count.checked_add(nominals::count(self)?)?
        } else {
            count
        };
        if count > limits.max_items {
            return None;
        }
        let source = self.source();
        let tokens = Lexer::new()
            .tokenize_with_limits(source, crate::compiler::resources::FrontendLimits::DEFAULT)
            .ok()?;
        let mut output = String::from("{\"declarations\":[");
        let mut names = Vec::new();
        for (index, checked) in self.functions().iter().enumerate() {
            let f = checked.typed().callable().function().syntax();
            let canonical = self.function_name(self.function_id_at(index)?)?;
            if canonical.len() > 4096 || !canonical.split("::").all(identifier) {
                return None;
            }
            let mut head = tokens
                .binary_search_by_key(&f.span.start, |token| token.span.start)
                .ok()?;
            if tokens[head].kind == TokenKind::Keyword("export") {
                head += 1;
            }
            if tokens.get(head)?.kind != TokenKind::Keyword("fn") {
                return None;
            }
            let name = tokens.get(head + 1)?;
            if !matches!(&name.kind, TokenKind::Identifier(value) if value == &f.name) {
                return None;
            }
            let end = tokens[head..]
                .iter()
                .find(|token| token.kind == TokenKind::Punctuation('{'))?
                .span
                .start;
            if end > f.span.end || end.checked_sub(f.span.start)? > 4096 {
                return None;
            }
            let signature = source.get(f.span.start..end)?.trim_end();
            if signature.is_empty() {
                return None;
            }
            if index != 0 {
                output.push(',');
            }
            output.push_str("{\"name\":");
            push_string(&mut output, canonical);
            output.push_str(",\"signature\":");
            push_string(&mut output, signature);
            write!(
                output,
                ",\"start\":{},\"end\":{},\"name_start\":{},\"name_end\":{}}}",
                f.span.start, f.span.end, name.span.start, name.span.end
            )
            .ok()?;
            if output.len() > limits.max_bytes {
                return None;
            }
            names.push(canonical);
        }
        let nominals = if include_bindings {
            Some(nominals::project(self, &tokens, limits.max_bytes)?)
        } else {
            None
        };
        if let Some(nominals) = &nominals {
            if !nominals.declarations.is_empty() {
                output.push(',');
                output.push_str(&nominals.declarations);
            }
        }
        if output.len() > limits.max_bytes {
            return None;
        }
        output.push_str("],\"references\":[");
        for (index, link) in self.call_links().iter().enumerate() {
            // Resolve by retained ID first; spelling only validates the source range.
            self.function(link.caller)?;
            self.function(link.callee)?;
            let name = self.function_name(link.callee)?;
            let definition = link.callee.ordinal();
            if names.get(definition).copied() != Some(name)
                || source.get(link.span.start..link.span.end)? != name
            {
                return None;
            }
            if index != 0 {
                output.push(',');
            }
            write!(
                output,
                "{{\"start\":{},\"end\":{},\"definition\":{}}}",
                link.span.start, link.span.end, definition
            )
            .ok()?;
            if output.len() > limits.max_bytes {
                return None;
            }
        }
        if let Some(nominals) = &nominals {
            if !nominals.references.is_empty() {
                if !self.call_links().is_empty() {
                    output.push(',');
                }
                output.push_str(&nominals.references);
            }
        }
        if output.len() > limits.max_bytes {
            return None;
        }
        output.push(']');
        if include_bindings {
            bindings::append(self, &tokens, &mut output, limits.max_bytes)?;
        } else {
            output.push_str(",\"locals\":[],\"local_references\":[]");
        }
        output.push_str(",\"truncated\":false}");
        (output.len() <= limits.max_bytes).then_some(output)
    }
}
