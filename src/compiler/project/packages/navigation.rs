//! Optional bounded source-owned navigation, projected from checked graph identities.
use crate::compiler::{
    check_report::push_string,
    checked::{CheckedUnit, DeclarationKind, NodeId, UseKind, UseTarget},
    lexer::{Lexer, TokenKind},
};
use std::{collections::HashMap, fmt::Write as _, path::Path, sync::Arc};

const MAX_FILES: usize = 32;
const MAX_SOURCE: usize = 4 * 1024 * 1024;
const MAX_BYTES: usize = 512 * 1024;
const MAX_RECORDS: usize = 8192;
/// Bounded version-one navigation JSON. Exhaustion publishes no partial tables.
#[derive(Debug)]
pub struct PackageNavigation {
    json: String,
}
impl PackageNavigation {
    /// Valid JSON with source fingerprints and graph-local declaration indices.
    pub fn as_json(&self) -> &str {
        &self.json
    }
}
/// Checked compilation and optional navigation from the exact same source reads.
#[derive(Debug)]
pub struct NavigablePackage {
    /// Ordinary linked checked output and remaining mandatory metadata allowances.
    pub compiled: super::CheckedPackage,
    /// Complete bounded projection, or an explicitly unavailable envelope.
    pub navigation: PackageNavigation,
}
struct File {
    owner: usize,
    path: String,
    source: Arc<str>,
}
struct Declaration {
    symbol: String,
    file: usize,
    name: String,
    signature: String,
    start: usize,
    end: usize,
    name_start: usize,
    name_end: usize,
}
struct Reference {
    file: usize,
    start: usize,
    end: usize,
    target: Option<String>,
    builtin: Option<String>,
}
#[derive(Default)]
pub(super) struct Recorder {
    files: Vec<File>,
    declarations: Vec<Declaration>,
    references: Vec<Reference>,
    source_bytes: usize,
    payload: usize,
    unavailable: bool,
}
impl Recorder {
    fn disable(&mut self) {
        self.unavailable = true;
        self.files.clear();
        self.declarations.clear();
        self.references.clear();
    }
    fn charge(&mut self, bytes: usize) -> bool {
        if self.unavailable {
            return false;
        }
        if let Some(total) = self
            .payload
            .checked_add(bytes)
            .filter(|total| *total <= MAX_BYTES)
        {
            self.payload = total;
            true
        } else {
            self.disable();
            false
        }
    }
    pub fn source(&mut self, owner: usize, path: &Path, source: &str) {
        if self.unavailable {
            return;
        }
        let Some(path) = path.to_str() else {
            self.disable();
            return;
        };
        if self.files.len() >= MAX_FILES
            || path.len() > 4096
            || source.len() > 512 * 1024
            || source.len() > MAX_SOURCE - self.source_bytes
        {
            self.disable();
            return;
        }
        if !self.charge(path.len() + 64) {
            return;
        }
        self.source_bytes += source.len();
        self.files.push(File {
            owner,
            path: path.to_owned(),
            source: Arc::from(source),
        });
    }
    pub fn unit(
        &mut self,
        owner: usize,
        unit: &CheckedUnit,
        targets: &std::collections::BTreeMap<String, String>,
    ) {
        if self.unavailable {
            return;
        }
        let Some(file) = self.files.iter().position(|file| file.owner == owner) else {
            self.disable();
            return;
        };
        let source = self.files[file].source.clone();
        let Ok(tokens) = Lexer::new().tokenize(&source) else {
            self.disable();
            return;
        };
        let evidence = unit.semantic_evidence().unwrap();
        let flow = evidence.flow().unwrap();
        let mut local = HashMap::<NodeId, String>::new();
        for declaration in flow
            .declarations()
            .iter()
            .filter(|d| d.kind == DeclarationKind::Function)
        {
            let node = &unit.nodes()[declaration.node.ordinal()];
            let Ok(index) = tokens.binary_search_by_key(&node.span.start, |token| token.span.start)
            else {
                self.disable();
                return;
            };
            let Some(name_token) = tokens.get(index + 1) else {
                self.disable();
                return;
            };
            if tokens[index].kind != TokenKind::Keyword("fn")
                || !matches!(&name_token.kind, TokenKind::Identifier(name) if name == declaration.name.as_ref())
            {
                self.disable();
                return;
            }
            let mut name = declaration.name.to_string();
            if let Some(parent) = node.parent {
                let span = unit.nodes()[parent.ordinal()].span;
                let Ok(module) = tokens.binary_search_by_key(&span.start, |token| token.span.start)
                else {
                    self.disable();
                    return;
                };
                let Some(token) = tokens.get(module + 1) else {
                    self.disable();
                    return;
                };
                let TokenKind::Identifier(prefix) = &token.kind else {
                    self.disable();
                    return;
                };
                name = format!("{prefix}::{name}");
            }
            let Some(symbol) = targets.get(&name) else {
                self.disable();
                return;
            };
            let Some(body) = tokens[index..]
                .iter()
                .find(|token| token.kind == TokenKind::Punctuation('{'))
            else {
                self.disable();
                return;
            };
            let Some(signature) = source.get(node.span.start..body.span.start) else {
                self.disable();
                return;
            };
            let signature = signature.trim_end();
            if self.declarations.len() >= MAX_RECORDS
                || name.len() > 4096
                || signature.len() > 4096
                || node.span.end > source.len()
                || !self.charge(name.len() + signature.len() + symbol.len())
            {
                self.disable();
                return;
            }
            local.insert(declaration.node, symbol.clone());
            self.declarations.push(Declaration {
                symbol: symbol.clone(),
                file,
                name,
                signature: signature.to_owned(),
                start: node.span.start,
                end: node.span.end,
                name_start: name_token.span.start,
                name_end: name_token.span.end,
            });
        }
        for usage in flow.uses().iter().filter(|u| u.kind == UseKind::Call) {
            let (target, builtin) = match usage.target {
                UseTarget::Declaration(node) => (local.get(&node).cloned(), None),
                UseTarget::Builtin(name) => (
                    None,
                    crate::compiler::stdlib::lookup(name).map(|f| {
                        format!(
                            "fn {}({}) -> {}",
                            f.name,
                            f.parameters.join(", "),
                            f.return_type
                        )
                    }),
                ),
            };
            if target.is_none() && builtin.is_none() {
                self.disable();
                return;
            }
            self.reference(
                file,
                unit.nodes()[usage.node.ordinal()].span,
                target,
                builtin,
            );
            if self.unavailable {
                return;
            }
        }
        let package = evidence.package().unwrap();
        for call in package.calls() {
            let target = package.imports()[call.import.ordinal()].symbol.to_string();
            self.reference(
                file,
                unit.nodes()[call.node.ordinal()].span,
                Some(target),
                None,
            );
            if self.unavailable {
                return;
            }
        }
    }
    fn reference(
        &mut self,
        file: usize,
        span: crate::compiler::diagnostics::Span,
        target: Option<String>,
        builtin: Option<String>,
    ) {
        if self.references.len() >= MAX_RECORDS
            || span.end > self.files[file].source.len()
            || !self.charge(
                target.as_ref().map_or(0, String::len) + builtin.as_ref().map_or(0, String::len),
            )
        {
            self.disable();
            return;
        }
        self.references.push(Reference {
            file,
            start: span.start,
            end: span.end,
            target,
            builtin,
        });
    }
    pub fn finish(self) -> PackageNavigation {
        fn unavailable() -> PackageNavigation {
            PackageNavigation {
                json: "{\"schema_version\":1,\"unavailable\":true}".to_owned(),
            }
        }
        if self.unavailable {
            return unavailable();
        }
        let indices: HashMap<_, _> = self
            .declarations
            .iter()
            .enumerate()
            .map(|(i, d)| (d.symbol.as_str(), i))
            .collect();
        let mut json = String::from("{\"schema_version\":1,\"unavailable\":false,\"files\":[");
        for (i, file) in self.files.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str("{\"path\":");
            push_string(&mut json, &file.path);
            json.push_str(",\"sha256\":");
            push_string(
                &mut json,
                &crate::native_build::sha256::digest(file.source.as_bytes()),
            );
            let _ = write!(json, ",\"bytes\":{}}}", file.source.len());
        }
        json.push_str("],\"declarations\":[");
        for (i, d) in self.declarations.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let _ = write!(json, "{{\"file\":{},\"name\":", d.file);
            push_string(&mut json, &d.name);
            json.push_str(",\"signature\":");
            push_string(&mut json, &d.signature);
            let _ = write!(
                json,
                ",\"start\":{},\"end\":{},\"name_start\":{},\"name_end\":{}}}",
                d.start, d.end, d.name_start, d.name_end
            );
        }
        json.push_str("],\"references\":[");
        for (i, r) in self.references.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let _ = write!(
                json,
                "{{\"file\":{},\"start\":{},\"end\":{},\"definition\":",
                r.file, r.start, r.end
            );
            if let Some(target) = &r.target {
                let Some(index) = indices.get(target.as_str()) else {
                    return unavailable();
                };
                let _ = write!(json, "{index}");
            } else {
                json.push_str("null,\"signature\":");
                push_string(&mut json, r.builtin.as_deref().unwrap());
            }
            json.push('}');
        }
        json.push_str("]}");
        if json.len() > MAX_BYTES {
            unavailable()
        } else {
            PackageNavigation { json }
        }
    }
}
