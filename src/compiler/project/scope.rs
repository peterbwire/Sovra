//! Scope resolution foundation for application service receivers.
//!
//! Callers must supply scopes and bindings from structured syntax. This module
//! does not scan source text or establish that an application body was parsed.

use crate::compiler::semantic::Type;
use std::collections::BTreeSet;

/// Identity of a service declaration, qualified by its source module.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServiceIdentity {
    /// Stable module identity chosen by the import resolver.
    pub module: String,
    /// Declared service name.
    pub name: String,
}

/// Result of resolving the receiver of an application member call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Receiver {
    /// A parameter, local, or closure binding shadows service candidates.
    Local,
    /// Exactly one visible service declaration.
    Service(ServiceIdentity),
    /// Multiple distinct service declarations share the visible name.
    Ambiguous(Vec<ServiceIdentity>),
    /// No visible binding or service; no service-call rule may be inferred.
    Unresolved,
}

/// A lexical scope for one source module's application syntax.
///
/// Child scopes borrow their enclosing scope. Bindings are visible from a byte
/// offset supplied by the parser (after a local initializer, or at body entry
/// for parameters). Service identities are available throughout their scope.
#[derive(Debug, Default)]
pub struct Scope<'a> {
    parent: Option<&'a Scope<'a>>,
    bindings: Vec<(String, usize, Option<Type>)>,
    fields: std::collections::HashMap<String, std::collections::HashMap<String, Type>>,
    services: BTreeSet<ServiceIdentity>,
    operations: Vec<super::service_types::TypedServiceOperation>,
    functions: std::collections::BTreeMap<String, Option<super::application::FunctionSignature>>,
}

impl<'a> Scope<'a> {
    /// Create a root scope with no implicit service imports.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a nested scope. Its bindings cannot leak back into the parent.
    pub fn child(&'a self) -> Scope<'a> {
        Scope {
            parent: Some(self),
            bindings: Vec::new(),
            fields: Default::default(),
            services: BTreeSet::new(),
            operations: Vec::new(),
            functions: std::collections::BTreeMap::new(),
        }
    }

    pub(super) fn add_fields(
        &mut self,
        fields: &std::collections::HashMap<String, std::collections::HashMap<String, Type>>,
    ) {
        self.fields.extend(fields.clone());
    }

    pub(super) fn field_type(&self, kind: &Type, name: &str) -> Option<Type> {
        let Type::Named(record) = kind else {
            return None;
        };
        if let Some(fields) = self.fields.get(record) {
            return fields.get(name).cloned();
        }
        self.parent.and_then(|parent| parent.field_type(kind, name))
    }

    pub(super) fn record_fields(
        &self,
        kind: &Type,
    ) -> Option<&std::collections::HashMap<String, Type>> {
        let Type::Named(record) = kind else {
            return None;
        };
        self.fields
            .get(record)
            .or_else(|| self.parent.and_then(|parent| parent.record_fields(kind)))
    }

    /// Record a parameter/local/closure binding and its visibility start offset.
    pub fn bind(&mut self, name: impl Into<String>, visible_from: usize) {
        self.bind_typed(name, visible_from, None);
    }

    pub(super) fn bind_typed(
        &mut self,
        name: impl Into<String>,
        visible_from: usize,
        kind: Option<Type>,
    ) {
        self.bindings.push((name.into(), visible_from, kind));
    }

    pub(super) fn binding_type(&self, name: &str, offset: usize) -> Option<Type> {
        if let Some((_, _, kind)) = self
            .bindings
            .iter()
            .rev()
            .find(|(binding, start, _)| binding == name && *start <= offset)
        {
            return kind.clone();
        }
        self.parent
            .and_then(|parent| parent.binding_type(name, offset))
    }

    /// Make an explicitly resolved service declaration visible in this scope.
    /// Repeated imports of the same declaration are idempotent.
    pub fn add_service(&mut self, service: ServiceIdentity) {
        self.services.insert(service);
    }

    pub(super) fn add_operations(
        &mut self,
        operations: &[super::service_types::TypedServiceOperation],
    ) {
        for operation in operations {
            for (name, fields) in operation.record_fields.iter() {
                self.fields
                    .entry(name.clone())
                    .or_insert_with(|| fields.clone());
            }
        }
        self.operations.extend_from_slice(operations);
    }

    pub(super) fn operation(
        &self,
        service: &ServiceIdentity,
        name: &str,
    ) -> Option<&super::service_types::TypedServiceOperation> {
        self.operations
            .iter()
            .find(|operation| operation.service == *service && operation.name == name)
            .or_else(|| {
                self.parent
                    .and_then(|parent| parent.operation(service, name))
            })
    }

    pub(super) fn add_functions(
        &mut self,
        functions: &std::collections::BTreeMap<
            String,
            Option<super::application::FunctionSignature>,
        >,
    ) {
        self.functions.clone_from(functions);
        for signature in functions.values().flatten() {
            for (identity, fields) in signature.record_fields.iter() {
                self.fields
                    .entry(identity.clone())
                    .or_insert_with(|| fields.clone());
            }
        }
    }

    pub(super) fn function(
        &self,
        name: &str,
        offset: usize,
    ) -> Option<&super::application::FunctionSignature> {
        if self.has_binding(name, offset) {
            return None;
        }
        self.find_function(name)
    }

    fn find_function(&self, name: &str) -> Option<&super::application::FunctionSignature> {
        if let Some(signature) = self.functions.get(name) {
            return signature.as_ref();
        }
        self.parent.and_then(|parent| parent.find_function(name))
    }

    /// Resolve a receiver at a source byte offset. Lexical bindings take
    /// precedence over all service candidates, including ancestor bindings.
    pub fn resolve(&self, name: &str, offset: usize) -> Receiver {
        if self.has_binding(name, offset) {
            return Receiver::Local;
        }
        let mut candidates = BTreeSet::new();
        self.collect_services(name, &mut candidates);
        match candidates.len() {
            0 => Receiver::Unresolved,
            1 => Receiver::Service(candidates.into_iter().next().expect("one candidate")),
            _ => Receiver::Ambiguous(candidates.into_iter().collect()),
        }
    }

    fn has_binding(&self, name: &str, offset: usize) -> bool {
        self.bindings
            .iter()
            .any(|(binding, start, _)| binding == name && *start <= offset)
            || self
                .parent
                .is_some_and(|parent| parent.has_binding(name, offset))
    }

    fn collect_services(&self, name: &str, candidates: &mut BTreeSet<ServiceIdentity>) {
        candidates.extend(
            self.services
                .iter()
                .filter(|service| service.name == name)
                .cloned(),
        );
        if let Some(parent) = self.parent {
            parent.collect_services(name, candidates);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn maps(module: &str) -> ServiceIdentity {
        ServiceIdentity {
            module: module.into(),
            name: "maps".into(),
        }
    }

    #[test]
    fn locals_shadow_only_after_their_visibility_boundary() {
        let mut module = Scope::new();
        module.add_service(maps("app.services"));
        let mut body = module.child();
        body.bind("maps", 50);
        assert_eq!(
            body.resolve("maps", 49),
            Receiver::Service(maps("app.services"))
        );
        assert_eq!(body.resolve("maps", 50), Receiver::Local);
        assert_eq!(body.child().resolve("maps", 60), Receiver::Local);
        assert_eq!(
            module.resolve("maps", 60),
            Receiver::Service(maps("app.services"))
        );
    }

    #[test]
    fn parameter_and_closure_bindings_do_not_leak_to_siblings() {
        let mut module = Scope::new();
        module.add_service(maps("app.services"));
        let mut closure = module.child();
        closure.bind("maps", 0);
        assert_eq!(closure.resolve("maps", 10), Receiver::Local);
        assert_eq!(
            module.child().resolve("maps", 10),
            Receiver::Service(maps("app.services"))
        );
        assert_eq!(module.resolve("unknown", 10), Receiver::Unresolved);
    }

    #[test]
    fn repeated_imports_are_idempotent_and_conflicts_are_explicit() {
        let mut module = Scope::new();
        module.add_service(maps("a"));
        module.add_service(maps("a"));
        assert_eq!(module.resolve("maps", 0), Receiver::Service(maps("a")));
        module.add_service(maps("b"));
        assert_eq!(
            module.resolve("maps", 0),
            Receiver::Ambiguous(vec![maps("a"), maps("b")])
        );
        let mut body = module.child();
        body.bind("maps", 0);
        assert_eq!(body.resolve("maps", 1), Receiver::Local);
    }
}
