//! Name resolution and basic type checking for M3.

use std::collections::{HashMap, HashSet};

use crate::compiler::ast::{Expression, ExpressionKind, Function, Program, Statement};
use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};
use crate::compiler::stdlib;

use super::checked::{CoercionKind, DeclarationKind, FactKind, NodeKind, Recorder, UseKind};

mod interfaces;
mod type_scopes;
pub(crate) use interfaces::{
    exported_record_fields, normalize_imported_signatures, InterfaceTypes,
};

impl Diagnostics {
    fn push_semantic(&mut self, diagnostic: Diagnostic) {
        self.push_capped_error(
            diagnostic,
            super::resources::FrontendLimits::DEFAULT.max_diagnostics,
            "E3019",
            "semantic",
        );
    }
}

/// The types understood by the initial semantic checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// No value.
    Unit,
    /// Boolean value.
    Bool,
    /// Integer value.
    Int,
    /// Floating-point value.
    Float,
    /// UTF-8 string value.
    String,
    /// A homogeneous array value.
    Array(Box<Type>),
    /// A named type resolved from a declared user type or future type registry.
    Named(String),
    /// An unresolved or invalid type.
    Unknown,
}

/// Decompose the bounded structural array spelling without recursive parsing.
pub(crate) fn array_parts(mut name: &str) -> Option<(usize, &str)> {
    let mut depth = 0;
    while let Some(inner) = name.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        depth += 1;
        if depth >= super::limits::MAX_STRUCTURAL_DEPTH {
            return None;
        }
        name = inner;
    }
    if name.is_empty() || name.contains(['[', ']']) {
        return None;
    }
    Some((depth, name))
}

fn array_wrap(mut kind: Type, count: usize) -> Type {
    if kind == Type::Unknown {
        return kind;
    }
    let mut existing = 0;
    let mut item = &kind;
    while let Type::Array(inner) = item {
        existing += 1;
        item = inner;
    }
    if existing + count >= super::limits::MAX_STRUCTURAL_DEPTH {
        return Type::Unknown;
    }
    for _ in 0..count {
        kind = Type::Array(Box::new(kind));
    }
    kind
}

impl Type {
    /// Resolve a type name against the built-in scalar types and a caller-supplied
    /// registry of declared named types. This keeps the checker forward-compatible
    /// with user-defined types without changing the existing source-language checks.
    pub fn from_name_with_known(
        name: &str,
        known_named_types: Option<&HashMap<String, Type>>,
    ) -> Self {
        let Some((arrays, name)) = array_parts(name) else {
            return Self::Unknown;
        };
        let kind = match name {
            "Unit" => Self::Unit,
            "Bool" => Self::Bool,
            "Int" => Self::Int,
            "Float" => Self::Float,
            "String" => Self::String,
            "std::InputLine" | "std::TextRead" | "std::TextWrite" | "std::SplitOnce"
            | "std::ParsedInt" => Self::Named(name.to_owned()),
            _ => known_named_types
                .and_then(|known| known.get(name).cloned())
                .unwrap_or(Self::Unknown),
        };
        array_wrap(kind, arrays)
    }

    /// Canonicalize a nominal name to its underlying primitive or alias target when
    /// that mapping is known to the semantic analyzer. Structs and aliases are tracked
    /// as named declarations, so we stop when the mapping loops back to the same symbol
    /// instead of recursing forever.
    pub fn canonicalize(&self, known_named_types: &HashMap<String, Type>) -> Self {
        match self {
            Self::Named(name) => {
                let mut current = name.clone();
                let mut seen = HashSet::new();
                loop {
                    if !seen.insert(current.clone()) {
                        return Self::Named(current);
                    }
                    match known_named_types.get(&current) {
                        Some(Self::Named(next)) if next == &current => {
                            return Self::Named(current);
                        }
                        Some(Self::Named(next)) => {
                            current = next.clone();
                        }
                        Some(other) => return other.clone(),
                        None => return Self::Named(current),
                    }
                }
            }
            _ => self.clone(),
        }
    }
}

/// Result of successful semantic analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    /// The validated program, with module-local type references qualified.
    pub program: Program,
}

/// Semantic analyzer.
#[derive(Debug, Default)]
pub struct SemanticAnalyzer;

impl SemanticAnalyzer {
    /// Construct an analyzer.
    pub const fn new() -> Self {
        Self
    }

    /// Resolve names and validate the M2 AST.
    /// Caller-built trees exceeding depth 128 return E3018 before recursive work
    /// or cloning. This borrowed check does not change the caller's AST drop behavior.
    /// The approved source-unit AST node ceiling is enforced before semantic cloning.
    pub fn analyze(&self, program: &Program) -> Result<TypedProgram, Diagnostics> {
        self.analyze_with_node_limit(program, super::resources::DEFAULT_AST_NODES)
    }

    /// Check a borrowed AST-node allowance before normal semantic cloning.
    /// This bounds admitted syntax size, not all repeated semantic operations.
    pub fn analyze_with_node_limit(
        &self,
        program: &Program,
        max_nodes: usize,
    ) -> Result<TypedProgram, Diagnostics> {
        super::resources::check_ast_budget(program, max_nodes)?;
        self.analyze_with_imports(program, &[])
    }

    pub(crate) fn analyze_with_imports<'a>(
        &self,
        program: &'a Program,
        imports: &[(String, &'a Function)],
    ) -> Result<TypedProgram, Diagnostics> {
        self.analyze_recorded_imports(program, imports, &mut Recorder::default())
    }

    pub(crate) fn analyze_recorded(
        &self,
        program: &Program,
        evidence: &mut Recorder,
    ) -> Result<TypedProgram, Diagnostics> {
        self.analyze_recorded_imports(program, &[], evidence)
    }

    pub(crate) fn analyze_recorded_imports<'a>(
        &self,
        program: &'a Program,
        imports: &[(String, &'a Function)],
        evidence: &mut Recorder,
    ) -> Result<TypedProgram, Diagnostics> {
        let mut diagnostics = Diagnostics::new();
        if let Err(span) = super::limits::check_program_depth(program) {
            diagnostics.push_semantic(diagnostic("E3018", super::limits::depth_message(), span));
            return Err(diagnostics);
        }
        let mut resolved_program = type_scopes::resolve(program)?;
        let program = &resolved_program;
        evidence.begin(program);
        let named_types = collect_named_types(program, &mut diagnostics);
        if diagnostics.limit_reached("E3019") {
            return Err(diagnostics);
        }
        let mut struct_fields = collect_struct_fields(program, &named_types, &mut diagnostics);
        if diagnostics.limit_reached("E3019") {
            return Err(diagnostics);
        }
        struct_fields.insert(
            crate::compiler::stdlib::INPUT_LINE_TYPE.to_owned(),
            HashMap::from([
                ("eof".to_owned(), Type::Bool),
                ("text".to_owned(), Type::String),
            ]),
        );
        struct_fields.insert(
            stdlib::TEXT_READ_TYPE.to_owned(),
            HashMap::from([
                ("ok".to_owned(), Type::Bool),
                ("text".to_owned(), Type::String),
                ("error".to_owned(), Type::String),
            ]),
        );
        struct_fields.insert(
            stdlib::TEXT_WRITE_TYPE.to_owned(),
            HashMap::from([
                ("ok".to_owned(), Type::Bool),
                ("error".to_owned(), Type::String),
            ]),
        );
        struct_fields.insert(
            stdlib::SPLIT_ONCE_TYPE.to_owned(),
            HashMap::from([
                ("found".to_owned(), Type::Bool),
                ("before".to_owned(), Type::String),
                ("after".to_owned(), Type::String),
            ]),
        );
        struct_fields.insert(
            stdlib::PARSED_INT_TYPE.to_owned(),
            HashMap::from([
                ("ok".to_owned(), Type::Bool),
                ("value".to_owned(), Type::Int),
            ]),
        );
        let mut declared_functions = HashMap::new();
        for function in &program.functions {
            if diagnostics.limit_reached("E3019") {
                return Err(diagnostics);
            }
            check_builtin_collision(&function.name, function.span, &mut diagnostics);
            if declared_functions
                .insert(function.name.clone(), function.span)
                .is_some()
            {
                diagnostics.push_semantic(diagnostic(
                    "E3008",
                    format_args!("duplicate function `{}`", function.name),
                    function.span,
                ));
            }
        }
        let mut module_names = HashMap::new();
        if diagnostics.limit_reached("E3019") {
            return Err(diagnostics);
        }
        for module in &program.modules {
            let mut seen = HashMap::new();
            for function in &module.functions {
                if diagnostics.limit_reached("E3019") {
                    return Err(diagnostics);
                }
                check_builtin_collision(
                    &format!("{}::{}", module.name, function.name),
                    function.span,
                    &mut diagnostics,
                );
                if seen.insert(function.name.clone(), function.span).is_some() {
                    diagnostics.push_semantic(diagnostic(
                        "E3008",
                        format_args!(
                            "duplicate function `{}` in module `{}`",
                            function.name, module.name
                        ),
                        function.span,
                    ));
                }
            }
            if module_names
                .insert(module.name.clone(), module.span)
                .is_some()
            {
                diagnostics.push_semantic(diagnostic(
                    "E3008",
                    format_args!("duplicate module `{}`", module.name),
                    module.span,
                ));
            }
        }
        if diagnostics.limit_reached("E3019") {
            return Err(diagnostics);
        }
        let mut functions: HashMap<String, &Function> = program
            .functions
            .iter()
            .map(|function| (function.name.clone(), function))
            .collect();
        for (name, function) in imports {
            if functions.insert(name.clone(), function).is_some() {
                diagnostics.push_semantic(diagnostic(
                    "E3008",
                    format_args!("duplicate imported function `{name}`"),
                    function.span,
                ));
            }
        }
        for module in &program.modules {
            for function in &module.functions {
                if function.is_exported {
                    functions.insert(format!("{}::{}", module.name, function.name), function);
                }
            }
        }
        for function in &program.functions {
            if diagnostics.limit_reached("E3019") {
                return Err(diagnostics);
            }
            if function.name == "main" && !function.parameters.is_empty() {
                diagnostics.push_semantic(diagnostic(
                    "E3009",
                    "entry function `main` cannot declare parameters",
                    function.span,
                ));
            }
            if function.name == "main"
                && function.return_type.as_deref().is_some_and(|return_type| {
                    !matches!(
                        Type::from_name_with_known(return_type, Some(&named_types))
                            .canonicalize(&named_types),
                        Type::Unit | Type::Unknown
                    )
                })
            {
                diagnostics.push_semantic(diagnostic(
                    "E3010",
                    "entry function `main` must return Unit",
                    function.span,
                ));
            }
            check_function_body(
                function,
                &functions,
                &named_types,
                &struct_fields,
                &mut diagnostics,
                evidence,
            );
        }
        for module in &program.modules {
            if diagnostics.limit_reached("E3019") {
                return Err(diagnostics);
            }
            let mut visible_functions = functions.clone();
            for function in &module.functions {
                visible_functions.insert(format!("{}::{}", module.name, function.name), function);
            }
            for function in &module.functions {
                check_function_body(
                    function,
                    &visible_functions,
                    &named_types,
                    &struct_fields,
                    &mut diagnostics,
                    evidence,
                );
            }
        }
        if diagnostics.is_empty() {
            type_scopes::canonicalize_alias_targets(&mut resolved_program, &named_types);
            Ok(TypedProgram {
                program: resolved_program,
            })
        } else {
            Err(diagnostics)
        }
    }
}

/// Reuse source-local type qualification for owned multi-file interfaces.
pub(crate) fn resolve_source_type_scopes(program: &Program) -> Result<Program, Diagnostics> {
    type_scopes::resolve(program)
}

/// Resolve declared type interfaces without checking function bodies.
/// Shared with application scalar alias inspection to preserve type rules.
pub(crate) fn collect_named_types(
    program: &Program,
    diagnostics: &mut Diagnostics,
) -> HashMap<String, Type> {
    collect_named_types_with_imports(program, &HashMap::new(), diagnostics)
}

/// Resolve local declarations against already resolved imported type identities.
pub(crate) fn collect_named_types_with_imports(
    program: &Program,
    imported: &HashMap<String, Type>,
    diagnostics: &mut Diagnostics,
) -> HashMap<String, Type> {
    let mut aliases = HashMap::new();
    let mut structured_names = HashSet::new();
    let mut type_declarations = HashMap::new();

    fn insert_alias(
        name: &str,
        target: &str,
        span: Span,
        aliases: &mut HashMap<String, String>,
        diagnostics: &mut Diagnostics,
    ) {
        if matches!(
            name,
            "Unit"
                | "Bool"
                | "Int"
                | "Float"
                | "String"
                | "std::InputLine"
                | "std::TextRead"
                | "std::TextWrite"
                | "std::SplitOnce"
                | "std::ParsedInt"
        ) || aliases.contains_key(name)
        {
            diagnostics.push_semantic(diagnostic(
                "E3008",
                format_args!("duplicate type `{name}`"),
                span,
            ));
            return;
        }
        aliases.insert(name.to_owned(), target.to_owned());
    }

    fn insert_struct(
        name: &str,
        declaration: &crate::compiler::ast::StructDeclaration,
        structured_names: &mut HashSet<String>,
        diagnostics: &mut Diagnostics,
    ) {
        if matches!(
            name,
            "Unit"
                | "Bool"
                | "Int"
                | "Float"
                | "String"
                | "std::InputLine"
                | "std::TextRead"
                | "std::TextWrite"
                | "std::SplitOnce"
                | "std::ParsedInt"
        ) || structured_names.contains(name)
        {
            diagnostics.push_semantic(diagnostic(
                "E3008",
                format_args!("duplicate type `{name}`"),
                declaration.span,
            ));
            return;
        }
        structured_names.insert(name.to_owned());
    }

    for declaration in &program.type_declarations {
        if diagnostics.limit_reached("E3019") {
            return HashMap::new();
        }
        insert_alias(
            &declaration.name,
            &declaration.target,
            declaration.span,
            &mut aliases,
            diagnostics,
        );
        type_declarations.insert(declaration.name.clone(), declaration);
    }
    for module in &program.modules {
        for declaration in &module.type_declarations {
            if diagnostics.limit_reached("E3019") {
                return HashMap::new();
            }
            let qualified = format!("{}::{}", module.name, declaration.name);
            insert_alias(
                &qualified,
                &declaration.target,
                declaration.span,
                &mut aliases,
                diagnostics,
            );
            type_declarations.insert(qualified.clone(), declaration);
        }
    }

    for declaration in &program.struct_declarations {
        if diagnostics.limit_reached("E3019") {
            return HashMap::new();
        }
        insert_struct(
            &declaration.name,
            declaration,
            &mut structured_names,
            diagnostics,
        );
    }
    for module in &program.modules {
        for declaration in &module.struct_declarations {
            if diagnostics.limit_reached("E3019") {
                return HashMap::new();
            }
            insert_struct(
                &format!("{}::{}", module.name, declaration.name),
                declaration,
                &mut structured_names,
                diagnostics,
            );
        }
    }
    let mut ordered_aliases: Vec<_> = aliases.keys().collect();
    ordered_aliases.sort();
    for alias_name in &ordered_aliases {
        if diagnostics.limit_reached("E3019") {
            return HashMap::new();
        }
        if structured_names.contains(*alias_name) {
            diagnostics.push_semantic(diagnostic(
                "E3008",
                format_args!("duplicate type `{alias_name}`"),
                type_declarations
                    .get(*alias_name)
                    .map(|declaration| declaration.span)
                    .unwrap_or(Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        column: 0,
                    }),
            ));
        }
    }

    let mut resolved = imported.clone();
    resolved.insert(
        crate::compiler::stdlib::INPUT_LINE_TYPE.to_owned(),
        Type::Named(crate::compiler::stdlib::INPUT_LINE_TYPE.to_owned()),
    );
    for name in [
        stdlib::TEXT_READ_TYPE,
        stdlib::TEXT_WRITE_TYPE,
        stdlib::SPLIT_ONCE_TYPE,
        stdlib::PARSED_INT_TYPE,
    ] {
        resolved.insert(name.to_owned(), Type::Named(name.to_owned()));
    }
    for name in structured_names.iter() {
        resolved.insert(name.clone(), Type::Named(name.clone()));
    }

    for name in ordered_aliases {
        if diagnostics.limit_reached("E3019") {
            return resolved;
        }
        resolve_type_alias(
            name,
            &aliases,
            &type_declarations,
            &mut resolved,
            diagnostics,
        );
    }
    resolved
}

fn resolve_type_alias(
    name: &str,
    aliases: &HashMap<String, String>,
    declarations: &HashMap<String, &crate::compiler::ast::TypeDeclaration>,
    resolved: &mut HashMap<String, Type>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let mut current = name;
    let mut path = Vec::new();
    let mut visiting = HashSet::new();
    let result = loop {
        if let Some(result) = resolved.get(current) {
            break result.clone();
        }
        let declaration = declarations[current];
        if !visiting.insert(current) {
            diagnostics.push_semantic(diagnostic(
                "E3017",
                format_args!(
                    "type `{}` creates a recursive alias cycle",
                    declaration.name
                ),
                declaration.span,
            ));
            break Type::Unknown;
        }
        let raw_target = aliases[current].as_str();
        let Some((arrays, target)) = array_parts(raw_target) else {
            diagnostics.push_semantic(diagnostic(
                "E3018",
                "array type depth or spelling is invalid",
                declaration.span,
            ));
            break Type::Unknown;
        };
        path.push((current, arrays));
        if target == current {
            diagnostics.push_semantic(diagnostic(
                "E3017",
                format_args!("type `{current}` cannot alias itself"),
                declaration.span,
            ));
            break Type::Unknown;
        }
        let primitive = type_from_name(target);
        if primitive != Type::Unknown {
            break primitive;
        }
        if aliases.contains_key(target) || resolved.contains_key(target) {
            current = target;
        } else {
            diagnostics.push_semantic(diagnostic("E3017", format_args!("unknown type `{target}`; expected Unit, Bool, Int, Float, String or a declared alias"), declaration.span));
            break Type::Unknown;
        }
    };
    let mut result = result;
    for (alias, arrays) in path.into_iter().rev() {
        let was_known = result != Type::Unknown;
        result = array_wrap(result, arrays);
        if was_known && result == Type::Unknown {
            diagnostics.push_semantic(diagnostic(
                "E3018",
                "expanded array type depth exceeds 128",
                declarations[alias].span,
            ));
        }
        resolved.insert(alias.to_owned(), result.clone());
    }
    result
}
/// Validate record field interfaces without checking executable bodies.
pub(crate) fn collect_struct_fields(
    program: &Program,
    named_types: &HashMap<String, Type>,
    diagnostics: &mut Diagnostics,
) -> HashMap<String, HashMap<String, Type>> {
    let mut structs = HashMap::new();
    let mut seen = HashSet::new();
    let mut insert_struct = |name: &str, declaration: &crate::compiler::ast::StructDeclaration| {
        if diagnostics.limit_reached("E3019") {
            return;
        }
        if !seen.insert(name.to_owned()) {
            diagnostics.push_semantic(diagnostic(
                "E3008",
                format_args!("duplicate type `{name}`"),
                declaration.span,
            ));
            return;
        }
        let mut fields = HashMap::new();
        let mut seen_fields = HashSet::new();
        for field in &declaration.fields {
            if diagnostics.limit_reached("E3019") {
                return;
            }
            if !seen_fields.insert(field.name.clone()) {
                diagnostics.push_semantic(diagnostic(
                    "E3008",
                    format_args!("duplicate field `{}` in struct `{name}`", field.name),
                    field.span,
                ));
                continue;
            }
            let field_type = Type::from_name_with_known(&field.type_name, Some(named_types));
            if field_type == Type::Unknown {
                diagnostics.push_semantic(diagnostic(
                    "E3017",
                    format_args!(
                        "unknown type `{}`; expected Unit, Bool, Int, Float, String or a declared alias",
                        field.type_name
                    ),
                    field.span,
                ));
            }
            fields.insert(field.name.clone(), field_type);
        }
        structs.insert(name.to_owned(), fields);
    };

    for declaration in &program.struct_declarations {
        insert_struct(&declaration.name, declaration);
    }
    for module in &program.modules {
        for declaration in &module.struct_declarations {
            insert_struct(
                &format!("{}::{}", module.name, declaration.name),
                declaration,
            );
        }
    }
    structs
}

// Import signatures must never resolve type names in the consumer's environment.
fn check_builtin_collision(name: &str, span: Span, diagnostics: &mut Diagnostics) {
    if stdlib::lookup(name).is_some() {
        diagnostics.push_semantic(diagnostic(
            "E3016",
            format_args!("function `{name}` conflicts with a builtin; choose a different name"),
            span,
        ));
    }
}

fn check_function_body(
    function: &Function,
    functions: &HashMap<String, &Function>,
    named_types: &HashMap<String, Type>,
    struct_fields: &HashMap<String, HashMap<String, Type>>,
    diagnostics: &mut Diagnostics,
    evidence: &mut Recorder,
) {
    if diagnostics.limit_reached("E3019") {
        return;
    }
    evidence.start_function();
    let mut scope = LocalScope::default();
    for parameter in &function.parameters {
        if diagnostics.limit_reached("E3019") {
            return;
        }
        if scope.types.contains_key(&parameter.name) {
            diagnostics.push_semantic(diagnostic(
                "E3011",
                format_args!("duplicate parameter `{}`", parameter.name),
                parameter.span,
            ));
        }
        let parameter_type = match parameter.type_name.as_deref() {
            Some(type_name) => check_annotation(
                type_name,
                parameter.type_span.unwrap_or(parameter.span),
                named_types,
                diagnostics,
            ),
            None => {
                diagnostics.push_semantic(diagnostic(
                    "E3014",
                    format_args!(
                        "parameter `{name}` requires an explicit type annotation; write `{name}: Type`",
                        name = parameter.name
                    ),
                    parameter.span,
                ));
                Type::Unknown
            }
        };
        evidence.record(
            parameter,
            NodeKind::Parameter,
            FactKind::Parameter,
            &parameter_type,
            named_types,
        );
        evidence.declare(
            parameter,
            NodeKind::Parameter,
            &parameter.name,
            DeclarationKind::Parameter,
        );
        scope.types.insert(parameter.name.clone(), parameter_type);
    }
    let expected_return = function
        .return_type
        .as_deref()
        .map(|name| {
            check_annotation(
                name,
                function.return_type_span.unwrap_or(function.span),
                named_types,
                diagnostics,
            )
        })
        .unwrap_or(Type::Unit);
    evidence.function_result(function, &expected_return, named_types);
    if !matches!(expected_return, Type::Unit | Type::Unknown)
        && !block_guarantees_return(&function.body)
    {
        diagnostics.push_semantic(diagnostic(
            "E3013",
            format_args!(
                "function `{}` must return {expected_return:?}",
                function.name
            ),
            function.span,
        ));
    }
    for statement in &function.body {
        if diagnostics.limit_reached("E3019") {
            break;
        }
        check_statement(
            statement,
            &mut scope,
            functions,
            named_types,
            struct_fields,
            &expected_return,
            diagnostics,
            evidence,
        );
    }
}

fn block_guarantees_return(statements: &[Statement]) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Return { .. } => true,
        Statement::If {
            then_block,
            else_block: Some(else_block),
            ..
        } => block_guarantees_return(then_block) && block_guarantees_return(else_block),
        Statement::Let { .. }
        | Statement::Assign { .. }
        | Statement::If {
            else_block: None, ..
        }
        | Statement::While { .. }
        | Statement::Expression(_) => false,
    })
}

#[derive(Clone, Default)]
struct LocalScope {
    types: HashMap<String, Type>,
    mutable_names: HashSet<String>,
}

#[allow(clippy::too_many_arguments)] // Keep the existing checker signature; recorder is optional.
fn check_statement(
    statement: &Statement,
    scope: &mut LocalScope,
    functions: &HashMap<String, &Function>,
    named_types: &HashMap<String, Type>,
    struct_fields: &HashMap<String, HashMap<String, Type>>,
    expected_return: &Type,
    diagnostics: &mut Diagnostics,
    evidence: &mut Recorder,
) {
    if diagnostics.limit_reached("E3019") {
        return;
    }
    match statement {
        Statement::Let {
            name,
            is_mutable,
            type_name,
            type_span,
            value,
            span,
        } => {
            let value_type = check_expression(
                value,
                &scope.types,
                functions,
                named_types,
                struct_fields,
                diagnostics,
                evidence,
            );
            let declared_type = type_name
                .as_deref()
                .map(|name| {
                    check_annotation(name, type_span.unwrap_or(*span), named_types, diagnostics)
                })
                .unwrap_or(value_type.clone());
            if !types_compatible(&declared_type, &value_type, named_types) {
                diagnostics.push_semantic(diagnostic(
                    "E3002",
                    format_args!(
                        "binding type mismatch for `{name}`: expected {declared_type:?}, found {value_type:?}"
                    ),
                    *span,
                ));
            }
            evidence.record(
                statement,
                NodeKind::Statement,
                FactKind::Local {
                    mutable: *is_mutable,
                },
                &declared_type,
                named_types,
            );
            evidence.coerce(
                statement,
                NodeKind::Statement,
                value,
                CoercionKind::Initialization,
                &value_type,
                &declared_type,
                named_types,
            );
            evidence.declare(
                statement,
                NodeKind::Statement,
                name,
                DeclarationKind::Local {
                    mutable: *is_mutable,
                },
            );
            scope.types.insert(name.clone(), declared_type);
            if *is_mutable {
                scope.mutable_names.insert(name.clone());
            } else {
                scope.mutable_names.remove(name);
            }
        }
        Statement::Assign {
            target,
            value,
            span,
        } => match &target.kind {
            ExpressionKind::Identifier(name) => {
                let Some(current_type) = scope.types.get(name).cloned() else {
                    diagnostics.push_semantic(diagnostic(
                        "E3001",
                        format_args!("undefined variable `{name}`"),
                        target.span,
                    ));
                    check_expression(
                        value,
                        &scope.types,
                        functions,
                        named_types,
                        struct_fields,
                        diagnostics,
                        evidence,
                    );
                    return;
                };
                if !scope.mutable_names.contains(name) {
                    diagnostics.push_semantic(diagnostic(
                        "E3011",
                        format_args!("variable `{name}` is not mutable"),
                        target.span,
                    ));
                }
                let value_type = check_expression(
                    value,
                    &scope.types,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                    evidence,
                );
                if !types_compatible(&current_type, &value_type, named_types) {
                    diagnostics.push_semantic(diagnostic(
                            "E3002",
                            format_args!(
                                "assignment type mismatch for `{name}`: expected {current_type:?}, found {value_type:?}"
                            ),
                            *span,
                        ));
                }
                evidence.variable(target, name, UseKind::Write);
                evidence.coerce(
                    statement,
                    NodeKind::Statement,
                    value,
                    CoercionKind::Assignment,
                    &value_type,
                    &current_type,
                    named_types,
                );
                scope.types.insert(name.clone(), current_type);
            }
            ExpressionKind::Index {
                target: base,
                index,
            } => {
                if let ExpressionKind::Identifier(name) = &base.kind {
                    if scope.types.contains_key(name) && !scope.mutable_names.contains(name) {
                        diagnostics.push_semantic(diagnostic(
                            "E3011",
                            format_args!("variable `{name}` is not mutable"),
                            base.span,
                        ));
                    }
                } else {
                    diagnostics.push_semantic(diagnostic(
                        "E3003",
                        "indexed assignment must target a mutable array binding directly",
                        base.span,
                    ));
                }
                let target_type = check_expression(
                    base,
                    &scope.types,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                    evidence,
                );
                let index_type = check_expression(
                    index,
                    &scope.types,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                    evidence,
                );
                if !types_compatible(&Type::Int, &index_type, named_types) {
                    diagnostics.push_semantic(diagnostic(
                        "E3005",
                        format_args!("array index must be Int, found {index_type:?}"),
                        index.span,
                    ));
                }
                let array_type = target_type.canonicalize(named_types);
                let element_type = match &array_type {
                    Type::Array(element_type) => element_type.as_ref(),
                    _ => {
                        diagnostics.push_semantic(diagnostic(
                            "E3007",
                            format_args!("indexing requires an array, found {array_type:?}"),
                            base.span,
                        ));
                        check_expression(
                            value,
                            &scope.types,
                            functions,
                            named_types,
                            struct_fields,
                            diagnostics,
                            evidence,
                        );
                        return;
                    }
                };
                let value_type = check_expression(
                    value,
                    &scope.types,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                    evidence,
                );
                if !types_compatible(element_type, &value_type, named_types) {
                    diagnostics.push_semantic(diagnostic(
                            "E3007",
                            format_args!(
                                "assignment type mismatch for index target: expected {element_type:?}, found {value_type:?}"
                            ),
                            value.span,
                        ));
                }
                if let ExpressionKind::Identifier(name) = &base.kind {
                    if evidence.has_flow() && scope.types.contains_key(name) {
                        evidence.variable(target, name, UseKind::IndexedWrite);
                    }
                }
                evidence.coerce(
                    statement,
                    NodeKind::Statement,
                    value,
                    CoercionKind::IndexedAssignment,
                    &value_type,
                    element_type,
                    named_types,
                );
            }
            _ => {
                diagnostics.push_semantic(diagnostic(
                    "E3003",
                    "assignment target must be a mutable variable or array index",
                    *span,
                ));
                check_expression(
                    value,
                    &scope.types,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                    evidence,
                );
            }
        },
        Statement::Return { value, span } => {
            let actual = value
                .as_ref()
                .map(|expression| {
                    check_expression(
                        expression,
                        &scope.types,
                        functions,
                        named_types,
                        struct_fields,
                        diagnostics,
                        evidence,
                    )
                })
                .unwrap_or(Type::Unit);
            if !types_compatible(expected_return, &actual, named_types) {
                diagnostics.push_semantic(diagnostic(
                    "E3002",
                    format_args!(
                        "return type mismatch: expected {expected_return:?}, found {actual:?}"
                    ),
                    *span,
                ));
            }
            if let Some(value) = value {
                evidence.coerce(
                    statement,
                    NodeKind::Statement,
                    value,
                    CoercionKind::Return,
                    &actual,
                    expected_return,
                    named_types,
                );
            }
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            span,
        } => {
            let condition_type = check_expression(
                condition,
                &scope.types,
                functions,
                named_types,
                struct_fields,
                diagnostics,
                evidence,
            );
            if !types_compatible(&Type::Bool, &condition_type, named_types) {
                diagnostics.push_semantic(diagnostic(
                    "E3005",
                    format_args!("if condition must be Bool, found {condition_type:?}"),
                    *span,
                ));
            }
            evidence.enter_scope();
            let mut then_scope = scope.clone();
            for statement in then_block {
                if diagnostics.limit_reached("E3019") {
                    break;
                }
                check_statement(
                    statement,
                    &mut then_scope,
                    functions,
                    named_types,
                    struct_fields,
                    expected_return,
                    diagnostics,
                    evidence,
                );
            }
            evidence.exit_scope();
            if let Some(else_block) = else_block {
                evidence.enter_scope();
                let mut else_scope = scope.clone();
                for statement in else_block {
                    if diagnostics.limit_reached("E3019") {
                        break;
                    }
                    check_statement(
                        statement,
                        &mut else_scope,
                        functions,
                        named_types,
                        struct_fields,
                        expected_return,
                        diagnostics,
                        evidence,
                    );
                }
                evidence.exit_scope();
            }
        }
        Statement::While {
            condition,
            body,
            span,
        } => {
            let condition_type = check_expression(
                condition,
                &scope.types,
                functions,
                named_types,
                struct_fields,
                diagnostics,
                evidence,
            );
            if !types_compatible(&Type::Bool, &condition_type, named_types) {
                diagnostics.push_semantic(diagnostic(
                    "E3005",
                    format_args!("while condition must be Bool, found {condition_type:?}"),
                    *span,
                ));
            }
            evidence.enter_scope();
            let mut loop_scope = scope.clone();
            for statement in body {
                if diagnostics.limit_reached("E3019") {
                    break;
                }
                check_statement(
                    statement,
                    &mut loop_scope,
                    functions,
                    named_types,
                    struct_fields,
                    expected_return,
                    diagnostics,
                    evidence,
                );
            }
            evidence.exit_scope();
        }
        Statement::Expression(expression) => {
            check_expression(
                expression,
                &scope.types,
                functions,
                named_types,
                struct_fields,
                diagnostics,
                evidence,
            );
        }
    }
}

fn check_expression(
    expression: &Expression,
    scope: &HashMap<String, Type>,
    functions: &HashMap<String, &Function>,
    named_types: &HashMap<String, Type>,
    struct_fields: &HashMap<String, HashMap<String, Type>>,
    diagnostics: &mut Diagnostics,
    evidence: &mut Recorder,
) -> Type {
    let result = check_expression_inner(
        expression,
        scope,
        functions,
        named_types,
        struct_fields,
        diagnostics,
        evidence,
    );
    if let ExpressionKind::Identifier(name) = &expression.kind {
        if evidence.has_flow() && scope.contains_key(name) {
            evidence.variable(expression, name, UseKind::Read);
        }
    }
    evidence.record(
        expression,
        NodeKind::Expression,
        FactKind::Expression,
        &result,
        named_types,
    );
    result
}

fn check_expression_inner(
    expression: &Expression,
    scope: &HashMap<String, Type>,
    functions: &HashMap<String, &Function>,
    named_types: &HashMap<String, Type>,
    struct_fields: &HashMap<String, HashMap<String, Type>>,
    diagnostics: &mut Diagnostics,
    evidence: &mut Recorder,
) -> Type {
    if diagnostics.limit_reached("E3019") {
        return Type::Unknown;
    }
    let span = expression.span;
    match &expression.kind {
        ExpressionKind::String(_) => Type::String,
        ExpressionKind::Integer(value) => {
            if value.parse::<i64>().is_err() {
                diagnostics.push_semantic(diagnostic(
                    "E3012",
                    "integer literal is outside the signed 64-bit range",
                    span,
                ));
            }
            Type::Int
        }
        ExpressionKind::Float(_) => Type::Float,
        ExpressionKind::Boolean(_) => Type::Bool,
        ExpressionKind::Identifier(name) => scope.get(name).cloned().unwrap_or_else(|| {
            diagnostics.push_semantic(diagnostic(
                "E3001",
                format_args!("undefined variable `{name}`"),
                span,
            ));
            Type::Unknown
        }),
        ExpressionKind::QualifiedName { path } => {
            let qualified = path.join("::");
            if stdlib::lookup(&qualified).is_some() || functions.contains_key(&qualified) {
                diagnostics.push_semantic(diagnostic(
                    "E3015",
                    format_args!("function `{qualified}` cannot be used as a value; call it with parentheses and the required arguments"),
                    span,
                ));
                Type::Unknown
            } else {
                diagnostics.push_semantic(diagnostic(
                    "E3004",
                    format_args!("undefined function `{qualified}`"),
                    span,
                ));
                Type::Unknown
            }
        }
        ExpressionKind::StructLiteral { type_name, fields } => {
            let resolved =
                Type::from_name_with_known(type_name, Some(named_types)).canonicalize(named_types);
            let identity = match &resolved {
                Type::Named(name) => name.as_str(),
                _ => type_name.as_str(),
            };
            let type_fields = match struct_fields.get(identity) {
                Some(fields) => fields,
                None => {
                    diagnostics.push_semantic(diagnostic(
                        "E3004",
                        format_args!("undefined struct `{type_name}`"),
                        span,
                    ));
                    for (_, value) in fields {
                        check_expression(
                            value,
                            scope,
                            functions,
                            named_types,
                            struct_fields,
                            diagnostics,
                            evidence,
                        );
                    }
                    return Type::Unknown;
                }
            };
            let declared_fields: std::collections::BTreeSet<_> = type_fields.keys().collect();
            let mut seen_fields = HashSet::new();
            for (field_name, value) in fields {
                if diagnostics.limit_reached("E3019") {
                    return Type::Unknown;
                }
                if !seen_fields.insert(field_name.clone()) {
                    diagnostics.push_semantic(diagnostic(
                        "E3008",
                        format_args!(
                            "duplicate field `{field_name}` in struct literal `{type_name}`"
                        ),
                        value.span,
                    ));
                    check_expression(
                        value,
                        scope,
                        functions,
                        named_types,
                        struct_fields,
                        diagnostics,
                        evidence,
                    );
                    continue;
                }
                let field_type = type_fields.get(field_name).cloned().unwrap_or_else(|| {
                    diagnostics.push_semantic(diagnostic(
                        "E3001",
                        format_args!("struct `{type_name}` has no field `{field_name}`"),
                        span,
                    ));
                    Type::Unknown
                });
                let value_type = check_expression(
                    value,
                    scope,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                    evidence,
                );
                evidence.coerce(
                    expression,
                    NodeKind::Expression,
                    value,
                    CoercionKind::RecordField,
                    &value_type,
                    &field_type,
                    named_types,
                );
                if !types_compatible(&field_type, &value_type, named_types) {
                    diagnostics.push_semantic(diagnostic(
                        "E3007",
                        format_args!(
                            "field `{field_name}` on `{type_name}` expects {field_type:?}, found {value_type:?}"
                        ),
                        value.span,
                    ));
                }
            }
            for field_name in declared_fields {
                if diagnostics.limit_reached("E3019") {
                    return Type::Unknown;
                }
                if !seen_fields.contains(field_name) {
                    diagnostics.push_semantic(diagnostic(
                        "E3001",
                        format_args!("struct `{type_name}` is missing field `{field_name}`"),
                        span,
                    ));
                }
            }
            resolved
        }
        ExpressionKind::ArrayLiteral(items) => {
            let mut element_type = Type::Unknown;
            for item in items {
                if diagnostics.limit_reached("E3019") {
                    return Type::Unknown;
                }
                let item_type = check_expression(
                    item,
                    scope,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                    evidence,
                );
                if element_type == Type::Unknown {
                    element_type = item_type.clone();
                } else if element_type == Type::Int && item_type == Type::Float {
                    element_type = Type::Float;
                } else if !types_compatible(&element_type, &item_type, named_types) {
                    diagnostics.push_semantic(diagnostic(
                        "E3007",
                        format_args!("array elements must share a compatible type, found {element_type:?} and {item_type:?}"),
                        item.span,
                    ));
                } else {
                    element_type = refine_array_inference(&element_type, &item_type);
                }
            }
            evidence.array_items(expression, items, &element_type, named_types);
            Type::Array(Box::new(element_type))
        }
        ExpressionKind::FieldAccess { receiver, field } => {
            let receiver_type = check_expression(
                receiver,
                scope,
                functions,
                named_types,
                struct_fields,
                diagnostics,
                evidence,
            );
            let struct_type = match receiver_type.canonicalize(named_types) {
                Type::Named(name) => name,
                _ => {
                    diagnostics.push_semantic(diagnostic(
                        "E3007",
                        format_args!("field access `{field}` requires a named struct instance"),
                        span,
                    ));
                    return Type::Unknown;
                }
            };
            struct_fields
                .get(&struct_type)
                .and_then(|fields| fields.get(field).cloned())
                .unwrap_or_else(|| {
                    diagnostics.push_semantic(diagnostic(
                        "E3001",
                        format_args!("struct `{struct_type}` has no field `{field}`"),
                        span,
                    ));
                    Type::Unknown
                })
        }
        ExpressionKind::Index { target, index } => {
            let target_type = check_expression(
                target,
                scope,
                functions,
                named_types,
                struct_fields,
                diagnostics,
                evidence,
            );
            let index_type = check_expression(
                index,
                scope,
                functions,
                named_types,
                struct_fields,
                diagnostics,
                evidence,
            );
            if !types_compatible(&Type::Int, &index_type, named_types) {
                diagnostics.push_semantic(diagnostic(
                    "E3005",
                    format_args!("array index must be Int, found {index_type:?}"),
                    index.span,
                ));
            }
            match target_type.canonicalize(named_types) {
                Type::Array(element_type) => *element_type,
                _ => {
                    diagnostics.push_semantic(diagnostic(
                        "E3007",
                        format_args!("indexing requires an array, found {target_type:?}"),
                        span,
                    ));
                    Type::Unknown
                }
            }
        }
        ExpressionKind::Call { callee, arguments } => {
            let name = match &callee.kind {
                ExpressionKind::Identifier(name) => Some(name.clone()),
                ExpressionKind::QualifiedName { path } => Some(path.join("::")),
                _ => {
                    diagnostics.push_semantic(diagnostic(
                        "E3003",
                        "call target must be a function name",
                        callee.span,
                    ));
                    check_expression(
                        callee,
                        scope,
                        functions,
                        named_types,
                        struct_fields,
                        diagnostics,
                        evidence,
                    );
                    for argument in arguments {
                        check_expression(
                            argument,
                            scope,
                            functions,
                            named_types,
                            struct_fields,
                            diagnostics,
                            evidence,
                        );
                    }
                    return Type::Unknown;
                }
            };
            let Some(name) = name else {
                return Type::Unknown;
            };
            if let Some(function) = stdlib::lookup(&name) {
                evidence.builtin_call(callee, function.name);
                return check_std_call(StdCallContext {
                    function,
                    source_name: &name,
                    arguments,
                    scope,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                    span,
                    evidence,
                    owner: expression,
                });
            }
            if let Some(function) = functions.get(&name) {
                evidence.function_call(callee, function, &name);
                if arguments.len() != function.parameters.len() {
                    diagnostics.push_semantic(diagnostic(
                        "E3006",
                        format_args!(
                            "function `{name}` expects {} argument(s), found {}",
                            function.parameters.len(),
                            arguments.len()
                        ),
                        span,
                    ));
                }
                for (argument, parameter) in arguments.iter().zip(&function.parameters) {
                    if diagnostics.limit_reached("E3019") {
                        return Type::Unknown;
                    }
                    let argument_type = check_expression(
                        argument,
                        scope,
                        functions,
                        named_types,
                        struct_fields,
                        diagnostics,
                        evidence,
                    );
                    if let Some(parameter_type) = parameter.type_name.as_deref() {
                        let expected =
                            Type::from_name_with_known(parameter_type, Some(named_types));
                        evidence.coerce(
                            expression,
                            NodeKind::Expression,
                            argument,
                            CoercionKind::Argument,
                            &argument_type,
                            &expected,
                            named_types,
                        );
                        if !types_compatible(&expected, &argument_type, named_types) {
                            diagnostics.push_semantic(diagnostic(
                                "E3007",
                                format_args!(
                                    "argument type mismatch for `{name}`: expected {expected:?}, found {argument_type:?}"
                                ),
                                argument.span,
                            ));
                        }
                    }
                }
                for argument in arguments.iter().skip(function.parameters.len()) {
                    if diagnostics.limit_reached("E3019") {
                        return Type::Unknown;
                    }
                    check_expression(
                        argument,
                        scope,
                        functions,
                        named_types,
                        struct_fields,
                        diagnostics,
                        evidence,
                    );
                }
                function
                    .return_type
                    .as_deref()
                    .map(|return_type| Type::from_name_with_known(return_type, Some(named_types)))
                    .unwrap_or(Type::Unit)
            } else {
                for argument in arguments {
                    check_expression(
                        argument,
                        scope,
                        functions,
                        named_types,
                        struct_fields,
                        diagnostics,
                        evidence,
                    );
                }
                diagnostics.push_semantic(diagnostic(
                    "E3004",
                    format_args!("undefined function `{name}`"),
                    callee.span,
                ));
                Type::Unknown
            }
        }
        ExpressionKind::Binary {
            left,
            operator,
            right,
        } => {
            let left_type = check_expression(
                left,
                scope,
                functions,
                named_types,
                struct_fields,
                diagnostics,
                evidence,
            );
            let right_type = check_expression(
                right,
                scope,
                functions,
                named_types,
                struct_fields,
                diagnostics,
                evidence,
            );
            if matches!(operator.as_str(), "&&" | "||") {
                if !types_compatible(&Type::Bool, &left_type, named_types)
                    || !types_compatible(&Type::Bool, &right_type, named_types)
                {
                    diagnostics.push_semantic(diagnostic(
                        "E3005",
                        format_args!("operator `{operator}` requires Bool operands"),
                        span,
                    ));
                }
                return Type::Bool;
            }
            let compound_equality = matches!(operator.as_str(), "==" | "!=")
                && matches!(
                    left_type.canonicalize(named_types),
                    Type::Array(_) | Type::Named(_)
                )
                && (types_compatible(&left_type, &right_type, named_types)
                    || types_compatible(&right_type, &left_type, named_types));
            let comparable = compound_equality
                || numeric_or_string_comparison_compatible(&left_type, &right_type)
                    && match operator.as_str() {
                        "==" | "!=" => {
                            matches!(
                                &left_type,
                                Type::Bool | Type::Int | Type::Float | Type::String
                            ) || matches!(
                                &right_type,
                                Type::Bool | Type::Int | Type::Float | Type::String
                            )
                        }
                        "<" | "<=" | ">" | ">=" => matches!(
                            (&left_type, &right_type),
                            (Type::Int, Type::Int)
                                | (Type::Float, Type::Float)
                                | (Type::Int, Type::Float)
                                | (Type::Float, Type::Int)
                                | (Type::String, Type::String)
                        ),
                        _ => false,
                    };
            let arithmetic = numeric_or_string_arithmetic_compatible(&left_type, &right_type)
                && match operator.as_str() {
                    "+" => matches!(
                        (&left_type, &right_type),
                        (Type::Int, Type::Int)
                            | (Type::Float, Type::Float)
                            | (Type::Int, Type::Float)
                            | (Type::Float, Type::Int)
                            | (Type::String, Type::String)
                    ),
                    "-" | "*" | "/" => matches!(
                        (&left_type, &right_type),
                        (Type::Int, Type::Int)
                            | (Type::Float, Type::Float)
                            | (Type::Int, Type::Float)
                            | (Type::Float, Type::Int)
                    ),
                    _ => false,
                };
            if comparable || arithmetic {
                if right_type == Type::Float {
                    evidence.coerce(
                        expression,
                        NodeKind::Expression,
                        left,
                        CoercionKind::BinaryOperand,
                        &left_type,
                        &Type::Float,
                        named_types,
                    );
                }
                if left_type == Type::Float {
                    evidence.coerce(
                        expression,
                        NodeKind::Expression,
                        right,
                        CoercionKind::BinaryOperand,
                        &right_type,
                        &Type::Float,
                        named_types,
                    );
                }
            }
            if comparable {
                Type::Bool
            } else if arithmetic {
                arithmetic_result_type(&left_type, &right_type)
            } else {
                diagnostics.push_semantic(diagnostic(
                    "E3005",
                    format_args!("operator `{operator}` cannot be applied to these types"),
                    span,
                ));
                Type::Unknown
            }
        }
    }
}

struct StdCallContext<'a> {
    function: stdlib::StdFunction,
    source_name: &'a str,
    arguments: &'a [Expression],
    scope: &'a HashMap<String, Type>,
    functions: &'a HashMap<String, &'a Function>,
    named_types: &'a HashMap<String, Type>,
    struct_fields: &'a HashMap<String, HashMap<String, Type>>,
    diagnostics: &'a mut Diagnostics,
    span: Span,
    evidence: &'a mut Recorder,
    owner: &'a Expression,
}

fn check_std_call(context: StdCallContext<'_>) -> Type {
    let StdCallContext {
        function,
        source_name,
        arguments,
        scope,
        functions,
        named_types,
        struct_fields,
        diagnostics,
        span,
        evidence,
        owner,
    } = context;

    let growth = matches!(function.name, "std::array_append" | "std::array_concat");
    let mut result_type = if function.name == "std::array_slice" || growth {
        Type::Unknown
    } else {
        type_from_name(function.return_type)
    };
    if arguments.len() != function.parameters.len() {
        diagnostics.push_semantic(diagnostic(
            "E3006",
            format_args!(
                "function `{source_name}` expects {} argument(s), found {}",
                function.parameters.len(),
                arguments.len()
            ),
            span,
        ));
    }
    for (index, (argument, expected)) in arguments.iter().zip(function.parameters).enumerate() {
        if diagnostics.limit_reached("E3019") {
            return Type::Unknown;
        }
        let argument_type = check_expression(
            argument,
            scope,
            functions,
            named_types,
            struct_fields,
            diagnostics,
            evidence,
        );
        let dependent = if growth && index == 1 {
            Some(if function.name == "std::array_append" {
                match &result_type {
                    Type::Array(element) => (**element).clone(),
                    _ => Type::Unknown,
                }
            } else {
                result_type.clone()
            })
        } else {
            None
        };
        if *expected == "Array" && dependent.is_none() {
            let canonical = argument_type.canonicalize(named_types);
            if (function.name == "std::array_slice" || growth)
                && matches!(canonical, Type::Array(_))
            {
                let mut leaf = &canonical;
                while let Type::Array(element) = leaf {
                    leaf = element;
                }
                if growth && matches!(leaf, Type::Unknown) {
                    diagnostics.push_semantic(diagnostic(
                        "E3007",
                        format_args!(
                            "{} requires a known first array element type",
                            function.name
                        ),
                        argument.span,
                    ));
                } else {
                    result_type = canonical.clone();
                }
            }
            if !matches!(canonical, Type::Array(_) | Type::Unknown) {
                diagnostics.push_semantic(diagnostic(
                    "E3007",
                    format_args!("{} expects an array argument", function.name),
                    argument.span,
                ));
            }
            continue;
        }
        if stdlib::is_any_type(expected) {
            continue;
        }
        let expected =
            dependent.unwrap_or_else(|| Type::from_name_with_known(expected, Some(named_types)));
        evidence.coerce(
            owner,
            NodeKind::Expression,
            argument,
            CoercionKind::Argument,
            &argument_type,
            &expected,
            named_types,
        );
        if !types_compatible(&expected, &argument_type, named_types) {
            diagnostics.push_semantic(diagnostic(
                "E3007",
                format_args!(
                    "argument type mismatch for `{source_name}`: expected {expected:?}, found {argument_type:?}"
                ),
                argument.span,
            ));
        }
    }
    for argument in arguments.iter().skip(function.parameters.len()) {
        if diagnostics.limit_reached("E3019") {
            return Type::Unknown;
        }
        check_expression(
            argument,
            scope,
            functions,
            named_types,
            struct_fields,
            diagnostics,
            evidence,
        );
    }
    result_type
}

fn type_from_name(name: &str) -> Type {
    Type::from_name_with_known(name, None)
}

fn check_annotation(
    name: &str,
    span: Span,
    named_types: &HashMap<String, Type>,
    diagnostics: &mut Diagnostics,
) -> Type {
    let resolved = Type::from_name_with_known(name, Some(named_types));
    if resolved == Type::Unknown {
        diagnostics.push_semantic(diagnostic(
            "E3017",
            format_args!(
                "unknown type `{name}`; expected Unit, Bool, Int, Float, String or a declared alias"
            ),
            span,
        ));
    }
    resolved
}

fn arithmetic_result_type(left: &Type, right: &Type) -> Type {
    match (left, right) {
        (Type::String, Type::String) => Type::String,
        (Type::Float, _) | (_, Type::Float) => Type::Float,
        (Type::Int, Type::Int) => Type::Int,
        _ => Type::Unknown,
    }
}

fn numeric_or_string_arithmetic_compatible(left: &Type, right: &Type) -> bool {
    matches!(
        (left, right),
        (Type::String, Type::String)
            | (Type::Unknown, _)
            | (_, Type::Unknown)
            | (Type::Int, Type::Int)
            | (Type::Int, Type::Float)
            | (Type::Float, Type::Int)
            | (Type::Float, Type::Float)
    )
}

fn numeric_or_string_comparison_compatible(left: &Type, right: &Type) -> bool {
    matches!(
        (left, right),
        (Type::String, Type::String)
            | (Type::Unknown, _)
            | (_, Type::Unknown)
            | (Type::Int, Type::Int)
            | (Type::Int, Type::Float)
            | (Type::Float, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool)
    )
}

// Empty nested literals contribute inference holes, not permission to forget
// concrete types observed in later compatible elements. This does not broaden
// compatibility or choose a new numeric common-type policy.
fn refine_array_inference(expected: &Type, actual: &Type) -> Type {
    match (expected, actual) {
        (Type::Unknown, actual) => actual.clone(),
        (Type::Array(expected), Type::Array(actual)) => {
            Type::Array(Box::new(refine_array_inference(expected, actual)))
        }
        _ => expected.clone(),
    }
}

fn types_compatible(expected: &Type, actual: &Type, named_types: &HashMap<String, Type>) -> bool {
    let expected = expected.canonicalize(named_types);
    let actual = actual.canonicalize(named_types);
    if matches!(actual, Type::Unknown) || matches!(expected, Type::Unknown) {
        return true;
    }
    if expected == actual {
        return true;
    }
    if let (Type::Array(expected_items), Type::Array(actual_items)) = (&expected, &actual) {
        return types_compatible(expected_items, actual_items, named_types);
    }
    if let (Type::Named(expected_name), Type::Named(actual_name)) = (&expected, &actual) {
        return expected_name == actual_name;
    }
    matches!(
        (expected, actual),
        (Type::Float, Type::Int) | (Type::String, Type::String)
    )
}

fn diagnostic(code: &'static str, message: impl std::fmt::Display, span: Span) -> Diagnostic {
    Diagnostic {
        source_file: None,
        severity: Severity::Error,
        code,
        message: crate::compiler::diagnostics::format_message(format_args!("{message}")),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_semantic_names_do_not_inflate_retained_messages() {
        let name = "x".repeat(10000);
        let source = format!("fn main() {{ print({name}); }}");
        let parsed = Parser::new().parse_source(&source).unwrap();
        let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
        assert_eq!(errors.items.len(), 1);
        let error = &errors.items[0];
        assert_eq!(error.code, "E3001");
        assert_eq!(&source[error.span.start..error.span.end], name);
        assert!(error.message.len() <= 4096);
        assert!(error.message.capacity() <= 4096);
        assert!(error.message.ends_with("[message truncated]"));
    }

    #[test]
    fn streamed_nested_type_debug_matches_previous_message_and_source_metadata() {
        let name = format!("Record{}", "x".repeat(10000));
        let source =
            format!("struct {name} {{ value: Float }} fn main() {{ let item: {name} = 1; }}");
        let parsed = Parser::new().parse_source(&source).unwrap();
        let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
        assert_eq!(errors.items.len(), 1);
        let error = &errors.items[0];
        assert_eq!(error.code, "E3002");
        assert!(source[error.span.start..error.span.end].contains("item"));
        let mut previous = error.clone();
        let expected = Type::Named(name);
        previous.message =
            format!("binding type mismatch for `item`: expected {expected:?}, found Int");
        assert_eq!(previous.bounded(), *error);
    }

    #[test]
    fn semantic_error_floods_are_bounded_and_keep_the_stopping_range() {
        for count in [255, 256, 1000] {
            let source = format!("fn main() {{ {} }}", "print(missing);".repeat(count));
            let program = Parser::new().parse_source(&source).unwrap();
            let errors = SemanticAnalyzer::new().analyze(&program).unwrap_err();
            assert_eq!(errors.items.len(), count.min(256));
            let last = errors.items.last().unwrap();
            assert_eq!(last.code, if count < 256 { "E3001" } else { "E3019" });
            assert_eq!(&source[last.span.start..last.span.end], "missing");
        }
    }

    #[test]
    fn semantic_declaration_and_type_scope_error_floods_are_bounded() {
        for source in [
            format!("{} fn main() {{}}", "type Alias = Int;".repeat(1000)),
            format!(
                "mod local {{ {} }} fn main() {{}}",
                "type Int = Int;".repeat(1000)
            ),
        ] {
            let program = Parser::new().parse_source(&source).unwrap();
            let errors = SemanticAnalyzer::new().analyze(&program).unwrap_err();
            assert_eq!(errors.items.len(), 256);
            assert_eq!(errors.items.last().unwrap().code, "E3019");
            assert!(crate::compiler::ir::lower_program(&program).is_err());
        }
    }

    #[test]
    fn process_exit_status_requires_one_int_argument() {
        for (source, code) in [
            ("fn main() { std::set_exit_code(); }", "E3006"),
            ("fn main() { std::set_exit_code(1, 2); }", "E3006"),
            ("fn main() { std::set_exit_code(\"bad\"); }", "E3007"),
        ] {
            let parsed = Parser::new().parse_source(source).unwrap();
            let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
            assert!(
                errors.items.iter().any(|error| error.code == code),
                "{source}"
            );
        }
        let parsed = Parser::new()
            .parse_source("fn main() { std::set_exit_code(1); }")
            .unwrap();
        SemanticAnalyzer::new().analyze(&parsed).unwrap();
    }

    #[test]
    fn long_module_alias_chains_resolve_without_native_recursion() {
        let mut source = String::from("mod chain { ");
        for index in 0..4096 {
            source.push_str(&format!(
                "type T{index} = {}; ",
                if index == 4095 {
                    "Float".into()
                } else {
                    format!("T{}", index + 1)
                }
            ));
        }
        source.push_str(
            "export fn value(x: T0) -> T0 { return x } } fn main() { print(chain::value(3) / 2); }",
        );
        let parsed = Parser::new().parse_source(&source).unwrap();
        let typed = SemanticAnalyzer::new().analyze(&parsed).unwrap();
        assert_eq!(
            typed.program.modules[0].functions[0].parameters[0]
                .type_name
                .as_deref(),
            Some("chain::T0")
        );
        assert!(typed.program.modules[0]
            .type_declarations
            .iter()
            .all(|alias| alias.target == "Float"));
        let lowered = crate::compiler::ir::lower(&typed);
        assert_eq!(
            crate::compiler::interpreter::run(&lowered).unwrap(),
            ["1.5"]
        );
    }

    #[test]
    fn module_types_do_not_leak_bare_names_or_nominal_identity() {
        for source in [
            "mod a { struct Point { x: Int } } fn read(p: Point) {}",
            "mod a { type Scalar = Int; } mod b { fn read(x: Scalar) {} }",
            "mod a { struct Point { x: Int } } mod b { struct Point { x: Int } export fn read(p: Point) {} } fn main() { b::read(a::Point { x: 1 }); }",
        ] {
            let program = crate::compiler::parser::Parser::new().parse_source(source).unwrap();
            assert!(SemanticAnalyzer::new().analyze(&program).is_err(), "{source}");
        }
    }

    #[test]
    fn module_type_resolution_preserves_input_spans_and_is_idempotent() {
        let source = "mod a { type Scalar = Float; struct Point { x: Scalar } fn make(p: Point) -> Point { if (true) { let q: Point = Point { x: 3 }; return q } else { return p } } }";
        let parsed = Parser::new().parse_source(source).unwrap();
        let original = parsed.clone();
        let typed = SemanticAnalyzer::new().analyze(&parsed).unwrap();
        assert_eq!(parsed, original);
        let function = &typed.program.modules[0].functions[0];
        assert_eq!(
            function.parameters[0].type_name.as_deref(),
            Some("a::Point")
        );
        assert_eq!(
            function.parameters[0].type_span,
            original.modules[0].functions[0].parameters[0].type_span
        );
        assert_eq!(
            typed,
            SemanticAnalyzer::new().analyze(&typed.program).unwrap()
        );
        let shadowed = Parser::new().parse_source("struct Point { x: String } type GlobalPoint = Point; mod a { struct Point { x: Int } type Alias = GlobalPoint; fn read(p: Alias) -> String { return p.x } }").unwrap();
        let typed = SemanticAnalyzer::new().analyze(&shadowed).unwrap();
        assert_eq!(
            typed,
            SemanticAnalyzer::new().analyze(&typed.program).unwrap()
        );
    }

    #[test]
    fn duplicate_and_unresolved_module_types_still_fail_in_their_owner() {
        for source in [
            "mod a { type X = Int; type X = Float; }",
            "mod a { struct X {} struct X {} }",
            "mod a { struct X {} type X = Int; }",
            "mod a { type Float = Int; }",
            "mod a { struct Int {} }",
            "mod a { type X = Y; type Y = X; }",
            "mod a { type X = Missing; } mod b { type Missing = Int; }",
        ] {
            let parsed = Parser::new().parse_source(source).unwrap();
            let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
            assert!(
                errors
                    .items
                    .iter()
                    .all(|error| ["E3008", "E3017"].contains(&error.code)),
                "{errors:?}"
            );
            assert!(errors
                .items
                .iter()
                .all(|error| error.span.end <= source.len()));
        }
    }

    #[test]
    fn manually_constructed_ast_respects_structural_depth() {
        for depth in [128, 129] {
            for in_module in [false, true] {
                let mut program = crate::compiler::parser::Parser::new()
                    .parse_source(if in_module {
                        "fn main() {} mod helper { fn unused() { 1; } }"
                    } else {
                        "fn main() { 1; }"
                    })
                    .unwrap();
                let span = Span {
                    start: 12,
                    end: 13,
                    line: 0,
                    column: 12,
                };
                let mut expression = Expression {
                    kind: ExpressionKind::Integer("1".into()),
                    span,
                };
                for _ in 1..depth {
                    expression = Expression {
                        kind: ExpressionKind::Binary {
                            left: Box::new(expression),
                            operator: "+".into(),
                            right: Box::new(Expression {
                                kind: ExpressionKind::Integer("1".into()),
                                span,
                            }),
                        },
                        span,
                    };
                }
                let function = if in_module {
                    &mut program.modules[0].functions[0]
                } else {
                    &mut program.functions[0]
                };
                function.body = vec![Statement::Expression(expression)];
                let result = SemanticAnalyzer::new().analyze(&program);
                if depth == 128 {
                    assert!(result.is_ok());
                } else {
                    let diagnostics = result.unwrap_err();
                    assert_eq!(diagnostics.items.len(), 1);
                    assert_eq!(diagnostics.items[0].code, "E3018");
                    assert_eq!(diagnostics.items[0].span, span);
                    assert!(crate::compiler::ir::lower_program(&program).is_err());
                }
            }
        }
    }
    use crate::compiler::parser::Parser;

    #[test]
    fn checks_excess_arguments_for_user_and_builtin_calls() {
        for call in [
            "helper(1, missing)",
            "std::len(\"ok\", missing)",
            "print(1, missing)",
        ] {
            let source = format!("fn helper(value: Int) {{}} fn main() {{ {call} }}");
            let parsed = Parser::new().parse_source(&source).unwrap();
            let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
            assert_eq!(errors.items.len(), 2, "{errors:?}");
            assert_eq!(errors.items[0].code, "E3006");
            assert_eq!(errors.items[1].code, "E3001");
            let span = errors.items[1].span;
            assert_eq!(&source[span.start..span.end], "missing");
        }
        let parsed = Parser::new()
            .parse_source("fn helper() {} fn main() { helper(std::len(1)) }")
            .unwrap();
        let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
        assert_eq!(
            errors
                .items
                .iter()
                .map(|error| error.code)
                .collect::<Vec<_>>(),
            ["E3006", "E3007"]
        );
    }

    #[test]
    fn private_helpers_are_visible_only_in_their_module() {
        let declarations = "mod math { fn helper(value: Int) -> Int { return value } export fn answer() -> Int { return math::helper(42) } }";
        let parsed = Parser::new().parse_source(declarations).unwrap();
        assert!(SemanticAnalyzer::new().analyze(&parsed).is_ok());
        for caller in [
            "fn main() { math::helper(1) }",
            "mod other { export fn call() { math::helper(1) } }",
            "mod other { fn call() { math::helper(1) } }",
        ] {
            let parsed = Parser::new()
                .parse_source(&format!("{declarations} {caller}"))
                .unwrap();
            let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
            assert!(errors.items.iter().any(|error| error.code == "E3004"));
        }
        for (body, code) in [
            ("math::helper()", "E3006"),
            ("math::helper(true)", "E3007"),
            ("let value = math::helper", "E3015"),
            ("helper(1)", "E3004"),
        ] {
            let source = format!("mod math {{ fn helper(value: Int) -> Int {{ return value }} export fn call() {{ {body} }} }}");
            let parsed = Parser::new().parse_source(&source).unwrap();
            let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
            assert!(
                errors.items.iter().any(|error| error.code == code),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn unknown_annotation_ranges_identify_only_type_tokens() {
        let source = "// λ\r\nmod sample { export fn identity(value: // parameter\r\n Strng) -> // return\r\n Strng { let copy: // local\r\n Strng = value; return copy } }";
        let parsed = Parser::new().parse_source(source).unwrap();
        let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
        let expected: Vec<_> = source
            .match_indices("Strng")
            .map(|(offset, _)| offset)
            .collect();
        assert_eq!(errors.items.len(), expected.len());
        for (error, start) in errors.items.iter().zip(expected) {
            assert_eq!(error.code, "E3017");
            assert_eq!(error.span.start, start);
            assert_eq!(error.span.end, start + 5);
            assert_eq!(error.span.line, source[..start].matches('\n').count());
            assert_eq!(error.span.column, 1);
        }
    }

    #[test]
    fn annotation_diagnostics_fall_back_for_ast_without_type_spans() {
        let mut parsed = Parser::new()
            .parse_source(
                "fn identity(value: Typo) -> Typo { let copy: Typo = value; return copy }",
            )
            .unwrap();
        let function = &mut parsed.functions[0];
        function.parameters[0].type_span = None;
        function.return_type_span = None;
        let mut expected = vec![function.parameters[0].span, function.span];
        if let Statement::Let {
            type_span, span, ..
        } = &mut function.body[0]
        {
            *type_span = None;
            expected.push(*span);
        }
        let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
        assert_eq!(errors.items.len(), 3);
        for (error, span) in errors.items.iter().zip(expected) {
            assert_eq!(error.code, "E3017");
            assert_eq!(error.span, span);
        }
    }

    #[test]
    fn primitive_annotations_and_local_inference_remain_valid() {
        let mut source = String::new();
        for (index, name) in ["Unit", "Bool", "Int", "Float", "String"]
            .iter()
            .enumerate()
        {
            source.push_str(&format!("fn identity{index}(value: {name}) -> {name} {{ let copy: {name} = value; return copy }}\n"));
        }
        source.push_str("fn main() { let inferred = 2; print(identity3(inferred)) }");
        let parsed = Parser::new().parse_source(&source).unwrap();
        let typed = SemanticAnalyzer::new().analyze(&parsed).unwrap();
        assert_eq!(
            crate::compiler::interpreter::run(&crate::compiler::ir::lower(&typed)).unwrap(),
            vec!["2"]
        );
    }

    #[test]
    fn named_types_are_compatible_by_declaration_name() {
        let user = Type::Named("User".to_string());
        let same = Type::Named("User".to_string());
        let other = Type::Named("Other".to_string());
        let empty = HashMap::new();
        assert!(types_compatible(&user, &same, &empty));
        assert!(!types_compatible(&user, &other, &empty));
        let known = HashMap::from([
            ("User".to_string(), Type::Named("User".to_string())),
            ("Alias".to_string(), Type::Int),
        ]);
        assert_eq!(
            Type::from_name_with_known("User", Some(&known)),
            Type::Named("User".to_string())
        );
        assert_eq!(Type::from_name_with_known("Alias", Some(&known)), Type::Int);
        assert_eq!(
            Type::from_name_with_known("Missing", Some(&known)),
            Type::Unknown
        );
    }

    #[test]
    fn alias_declarations_are_resolved_transitively_and_order_independent() {
        let source = "type User = Int; type Account = User; fn helper(value: Account) -> User { return value } fn main() { let value: User = 1; let account: Account = value; let result: Int = helper(account); }";
        let parsed = Parser::new().parse_source(source).unwrap();
        assert!(
            SemanticAnalyzer::new().analyze(&parsed).is_ok(),
            "{source:?}"
        );

        let reversed = "type Account = User; type User = Int; fn helper(value: Account) -> User { return value } fn main() { let value: User = 1; let account: Account = value; let result: Int = helper(account); }";
        let parsed = Parser::new().parse_source(reversed).unwrap();
        assert!(
            SemanticAnalyzer::new().analyze(&parsed).is_ok(),
            "{reversed:?}"
        );
    }

    #[test]
    fn module_local_type_aliases_are_visible_inside_module_scope() {
        let source =
            "mod math { type User = Int; export fn identity(value: User) -> User { return value } }
            fn main() { let value: Int = math::identity(2); }
        ";
        let parsed = Parser::new().parse_source(source).unwrap();
        assert!(
            SemanticAnalyzer::new().analyze(&parsed).is_ok(),
            "{source:?}"
        );
    }

    #[test]
    fn aliases_can_target_declared_structs() {
        let source = "struct User { value: Int } type UserAlias = User; fn make() -> UserAlias { let user = User { value: 7 }; return user } fn main() { let user: UserAlias = make(); let total: Int = user.value; }";
        let parsed = Parser::new().parse_source(source).unwrap();
        assert!(
            SemanticAnalyzer::new().analyze(&parsed).is_ok(),
            "{source:?}"
        );

        let module_source = "mod math { struct User { value: Int } type UserAlias = User; export fn make() -> UserAlias { let user = User { value: 9 }; return user } } fn main() { let user: math::UserAlias = math::make(); let total: Int = user.value; }";
        let parsed = Parser::new().parse_source(module_source).unwrap();
        assert!(
            SemanticAnalyzer::new().analyze(&parsed).is_ok(),
            "{module_source:?}"
        );
    }

    #[test]
    fn struct_literals_and_field_access_match_declared_fields() {
        let literal_source = "
            struct User { name: String, age: Int }
            fn main() {
                let user = User { name: 42, age: 42 };
            }
        ";
        let literal_errors = SemanticAnalyzer::new()
            .analyze(&Parser::new().parse_source(literal_source).unwrap())
            .unwrap_err();
        assert!(literal_errors.items.iter().any(|item| item.code == "E3007"));

        let binding_source = "
            struct User { name: String, age: Int }
            fn main() {
                let user = User { name: \"Ada\", age: 42 };
                let bad: Int = user.name;
            }
        ";
        let binding_errors = SemanticAnalyzer::new()
            .analyze(&Parser::new().parse_source(binding_source).unwrap())
            .unwrap_err();
        assert!(binding_errors.items.iter().any(|item| item.code == "E3002"));
    }

    #[test]
    fn records_construct_access_pass_and_return_across_functions() {
        let source = "
            struct Point { x: Int, y: Int }
            type Position = Point;
            struct Marker { label: String, position: Position }
            fn point_x(point: Point) -> Int { return point.x }
            fn make_marker() -> Marker {
                let point: Position = Point { x: 12, y: 34 };
                return Marker { label: \"origin\", position: point }
            }
            fn main() {
                let marker = make_marker();
                print(point_x(marker.position));
                print(marker.position.y);
                print(std::to_string(marker));
            }
        ";
        let program = Parser::new().parse_source(source).expect("valid syntax");
        let ir = crate::compiler::ir::lower_program(&program).expect("valid record types");
        let output = crate::compiler::interpreter::run(&ir).expect("record execution");
        assert_eq!(
            output,
            [
                "12",
                "34",
                "Marker { label: origin, position: Point { x: 12, y: 34 } }"
            ]
        );
    }

    #[test]
    fn module_scoped_structs_use_qualified_names() {
        let source = "mod math { struct User { value: Int } export fn make() -> math::User { let user = math::User { value: 42 }; return user } } fn main() { let user: math::User = math::User { value: 7 }; let total: Int = user.value; }";
        let parsed = Parser::new().parse_source(source).unwrap();
        assert!(
            SemanticAnalyzer::new().analyze(&parsed).is_ok(),
            "{source:?}"
        );
    }

    #[test]
    fn duplicate_fields_in_struct_literals_are_rejected() {
        let source = "struct User { name: String, age: Int } fn main() { let user = User { name: \"Ada\", age: 42, name: \"Grace\" }; }";
        let parsed = Parser::new().parse_source(source).unwrap();
        let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
        assert!(errors.items.iter().any(|item| item.code == "E3008"));
    }

    #[test]
    fn struct_literals_require_all_declared_fields() {
        let source = "struct User { name: String, age: Int } fn main() { let user = User { name: \"Ada\" }; }";
        let parsed = Parser::new().parse_source(source).unwrap();
        let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
        assert!(errors.items.iter().any(|item| item.code == "E3001"));

        let extra_source = "struct User { name: String, age: Int } fn main() { let user = User { name: \"Ada\", age: 42, extra: true }; }";
        let parsed = Parser::new().parse_source(extra_source).unwrap();
        let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
        assert!(errors.items.iter().any(|item| item.code == "E3001"));
    }

    #[test]
    fn recursive_type_aliases_are_rejected() {
        let sources = [
            "type Loop = Loop; fn main() {}",
            "type A = B; type B = A; fn main() {}",
            "type Bad = Missing; fn main() {}",
        ];
        for source in sources {
            let parsed = Parser::new().parse_source(source).unwrap();
            let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
            assert!(
                errors.items.iter().any(|e| e.code == "E3017"),
                "{source:?} => {errors:?}"
            );
        }
    }

    #[test]
    fn unknown_annotations_are_reported_once_at_declarations() {
        let source = "fn helper(value: Typo) -> Typo { return value } fn main() -> Typo { helper(1); helper(2) } fn missing(value) {}";
        let parsed = Parser::new().parse_source(source).unwrap();
        let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
        assert_eq!(errors.items.iter().filter(|e| e.code == "E3017").count(), 3);
        assert_eq!(errors.items.iter().filter(|e| e.code == "E3014").count(), 1);
        assert_eq!(errors.items.len(), 4, "{errors:?}");
    }

    #[test]
    fn rejects_unresolved_annotations_in_every_function() {
        for declaration in [
            "fn identity(value: Strng) -> Strng { let copy: Strng = value; return copy }",
            "fn unused(value: Any) {}",
            "fn unused(value: Text) {}",
        ] {
            for wrapper in ["top", "private", "exported"] {
                let source = match wrapper {
                    "top" => declaration.to_owned(),
                    "private" => format!("mod sample {{ {declaration} }}"),
                    _ => format!("mod sample {{ export {declaration} }}"),
                };
                let parsed = Parser::new().parse_source(&source).unwrap();
                let errors = SemanticAnalyzer::new()
                    .analyze(&parsed)
                    .expect_err("unknown type");
                let expected = if declaration.contains("Strng") { 3 } else { 1 };
                assert_eq!(
                    errors.items.iter().filter(|e| e.code == "E3017").count(),
                    expected,
                    "{errors:?}"
                );
            }
        }
    }

    #[test]
    fn process_input_builtins_reject_wrong_argument_contracts() {
        let source = "fn main() { std::arg(\"zero\"); std::read_line(1); std::arg_count(1); }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .unwrap();
        let diagnostics = SemanticAnalyzer::new().analyze(&program).unwrap_err();
        let codes: Vec<_> = diagnostics.items.iter().map(|item| item.code).collect();
        assert_eq!(codes, ["E3007", "E3006", "E3006"]);
    }

    #[test]
    fn text_file_builtins_validate_types_and_fields() {
        let valid = "fn main() { let loaded: std::TextRead = std::read_text(\"x\"); if (loaded.ok) { print(loaded.text); } let saved: std::TextWrite = std::write_text(\"x\", loaded.text); print(saved.error); }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(valid)
            .unwrap();
        SemanticAnalyzer::new().analyze(&program).unwrap();

        let invalid = "fn main() { std::read_text(1); std::write_text(\"x\", 1); }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(invalid)
            .unwrap();
        let errors = SemanticAnalyzer::new().analyze(&program).unwrap_err();
        assert_eq!(
            errors
                .items
                .iter()
                .map(|item| item.code)
                .collect::<Vec<_>>(),
            ["E3007", "E3007"]
        );
    }

    #[test]
    fn conditional_text_write_checks_signature_and_result() {
        let valid = "fn main() { let saved: std::TextWrite = std::write_text_if(\"x\", false, \"\", \"new\"); print(saved.ok); print(saved.error); }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(valid)
            .unwrap();
        SemanticAnalyzer::new().analyze(&program).unwrap();
        for invalid in [
            "fn main() { std::write_text_if(\"x\", \"yes\", \"\", \"new\"); }",
            "fn main() { std::write_text_if(\"x\", false, \"\"); }",
            "fn main() { std::write_text_if(1, false, \"\", \"new\"); }",
            "fn main() { std::write_text_if(\"x\", false, 1, \"new\"); }",
            "fn main() { std::write_text_if(\"x\", false, \"\", 1); }",
        ] {
            let program = crate::compiler::parser::Parser::new()
                .parse_source(invalid)
                .unwrap();
            assert!(SemanticAnalyzer::new().analyze(&program).is_err());
        }
    }

    #[test]
    fn empty_nested_arrays_do_not_erase_known_element_types() {
        for body in [
            "let values = [[], [1], [\"bad\"]];",
            "let values = [[[]], [[1]], [[true]]];",
            "let mut values = [[], [2.0]]; values[0] = [\"bad\"];",
            "let values = [[], [1], [2.0]];",
            "let empty = []; let values = [empty, [1], [\"bad\"]];",
        ] {
            let source = format!("fn main() {{ {body} }}");
            let program = crate::compiler::parser::Parser::new()
                .parse_source(&source)
                .unwrap();
            let errors = SemanticAnalyzer::new()
                .analyze(&program)
                .expect_err("empty arrays must not erase a known element type");
            assert!(errors.items.iter().any(|error| error.code == "E3007"));
        }
        for body in [
            "let values = [[], [1], [2]];",
            "let mut values = [[], [2.0], [1]]; values[0] = [3];",
            "let values = [[[]], [[1]], [[2]]];",
            "let values = [[], []];",
            "let empty = []; let ints = [empty, [1]]; let strings = [empty, [\"ok\"]];",
        ] {
            let source = format!("fn main() {{ {body} }}");
            let program = crate::compiler::parser::Parser::new()
                .parse_source(&source)
                .unwrap();
            SemanticAnalyzer::new().analyze(&program).unwrap();
        }
        let nominal = "struct Left { value: Int } struct Right { value: Int }
            fn main() { let values = [[], [Left { value: 1 }], [Right { value: 1 }]]; }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(nominal)
            .unwrap();
        let errors = SemanticAnalyzer::new().analyze(&program).unwrap_err();
        assert!(errors.items.iter().any(|error| error.code == "E3007"));
    }

    #[test]
    fn text_decoding_builtins_validate_types_and_fields() {
        let valid = "fn main() { let split: std::SplitOnce = std::split_once(\"a,b\", \",\"); if (split.found) { print(split.after); } let parsed: std::ParsedInt = std::parse_int(split.before); if (parsed.ok) { print(parsed.value); } }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(valid)
            .unwrap();
        SemanticAnalyzer::new().analyze(&program).unwrap();

        let invalid = "fn main() { std::split_once(1, \"x\"); std::parse_int(true); }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(invalid)
            .unwrap();
        let errors = SemanticAnalyzer::new().analyze(&program).unwrap_err();
        assert_eq!(
            errors
                .items
                .iter()
                .map(|item| item.code)
                .collect::<Vec<_>>(),
            ["E3007", "E3007"]
        );
    }

    #[test]
    fn lines_unique_requires_one_string_argument() {
        let valid =
            "fn main() { let unique: Bool = std::lines_unique(\"a\\nb\\n\"); print(unique); }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(valid)
            .unwrap();
        SemanticAnalyzer::new().analyze(&program).unwrap();

        let invalid = "fn main() { std::lines_unique(1); std::lines_unique(); }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(invalid)
            .unwrap();
        let errors = SemanticAnalyzer::new().analyze(&program).unwrap_err();
        assert_eq!(
            errors
                .items
                .iter()
                .map(|item| item.code)
                .collect::<Vec<_>>(),
            ["E3007", "E3006"]
        );
    }

    #[test]
    fn escape_terminal_requires_exactly_one_string_argument() {
        let valid =
            "fn main() { let shown: String = std::escape_terminal(\"title\"); print(shown); }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(valid)
            .unwrap();
        SemanticAnalyzer::new().analyze(&program).unwrap();

        let invalid = "fn main() { std::escape_terminal(1); std::escape_terminal(); std::escape_terminal(\"a\", \"b\"); }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(invalid)
            .unwrap();
        let errors = SemanticAnalyzer::new().analyze(&program).unwrap_err();
        assert_eq!(
            errors
                .items
                .iter()
                .map(|item| item.code)
                .collect::<Vec<_>>(),
            ["E3007", "E3006", "E3006"]
        );
    }

    #[test]
    fn compound_equality_requires_matching_array_or_nominal_record_types() {
        let source = "struct A { value: Int } struct B { value: Int } fn main() { print(A { value: 1 } == B { value: 1 }); print([1] == [\"one\"]); }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .unwrap();
        let errors = SemanticAnalyzer::new().analyze(&program).unwrap_err();
        assert_eq!(
            errors
                .items
                .iter()
                .filter(|item| item.code == "E3005")
                .count(),
            2
        );
    }

    #[test]
    fn standard_input_line_identity_cannot_be_redeclared() {
        for name in ["InputLine", "TextRead", "TextWrite"] {
            let source =
                format!("mod std {{ export struct {name} {{ value: Int }} }} fn main() {{}}");
            let program = crate::compiler::parser::Parser::new()
                .parse_source(&source)
                .unwrap();
            let diagnostics = SemanticAnalyzer::new().analyze(&program).unwrap_err();
            assert!(diagnostics.items.iter().any(|item| item.code == "E3008"));
        }
    }

    #[test]
    fn rejects_exact_builtin_callable_collisions() {
        let mut sources = vec![("fn print() {} fn main() {}".to_owned(), "print".to_owned())];
        for builtin in stdlib::functions() {
            let member = builtin.name.strip_prefix("std::").unwrap();
            sources.push((
                format!("mod std {{ export fn {member}() {{}} }} fn main() {{}}"),
                builtin.name.to_owned(),
            ));
            sources.push((
                format!("mod std {{ fn {member}() {{}} }} fn main() {{}}"),
                builtin.name.to_owned(),
            ));
        }
        for (source, name) in sources {
            let parsed = Parser::new().parse_source(&source).unwrap();
            let errors = SemanticAnalyzer::new()
                .analyze(&parsed)
                .expect_err("builtin collision");
            let collisions: Vec<_> = errors.items.iter().filter(|e| e.code == "E3016").collect();
            assert_eq!(collisions.len(), 1, "{errors:?}");
            assert!(collisions[0].message.contains(&name));
            assert!(source[collisions[0].span.start..collisions[0].span.end].starts_with("fn "));
        }
    }

    #[test]
    fn allows_noncolliding_std_members_and_private_names() {
        let parsed = Parser::new().parse_source(
            "mod std { fn helper() {} export fn extra() -> Int { return 7 } }
             mod other { export fn print() -> Int { return 8 } }
             fn len() -> Int { return 9 }
             fn main() { print(std::extra()); print(other::print()); print(len()); print(std::len(\"hi\")) }"
        ).unwrap();
        let typed = SemanticAnalyzer::new()
            .analyze(&parsed)
            .expect("only exact callable collisions are forbidden");
        assert_eq!(
            crate::compiler::interpreter::run(&crate::compiler::ir::lower(&typed)).unwrap(),
            vec!["7", "8", "9", "2"]
        );
    }

    #[test]
    fn rejects_duplicate_module_functions_regardless_of_visibility() {
        for (first, second) in [
            ("", ""),
            ("export ", ""),
            ("", "export "),
            ("export ", "export "),
        ] {
            let source = format!(
                "mod sample {{ {first}fn helper() {{}} {second}fn helper() {{}} }} fn main() {{}}"
            );
            let parsed = Parser::new().parse_source(&source).unwrap();
            let errors = SemanticAnalyzer::new()
                .analyze(&parsed)
                .expect_err("duplicate declaration");
            let duplicates: Vec<_> = errors.items.iter().filter(|e| e.code == "E3008").collect();
            assert_eq!(duplicates.len(), 1);
            assert_eq!(duplicates[0].span.start, source.rfind("fn helper").unwrap());
        }
    }

    #[test]
    fn rejects_qualified_functions_used_as_values() {
        for expression in ["std::len", "sample::value"] {
            for body in [
                format!("let value = {expression}"),
                format!("print({expression})"),
                expression.to_owned(),
            ] {
                let source = format!("mod sample {{ export fn value() -> Int {{ return 1 }} }} fn main() {{ {body} }}");
                let parsed = Parser::new().parse_source(&source).unwrap();
                let errors = SemanticAnalyzer::new()
                    .analyze(&parsed)
                    .expect_err("function values are unsupported");
                let error = errors
                    .items
                    .iter()
                    .find(|e| e.code == "E3015")
                    .expect("function-value diagnostic");
                assert_eq!(&source[error.span.start..error.span.end], expression);
            }
        }
    }

    #[test]
    fn concatenation_preserves_string_type_contracts() {
        for (source, code) in [
            ("fn main() { let number: Int = \"a\" + \"b\" }", "E3002"),
            ("fn value() -> Int { return \"a\" + \"b\" }", "E3002"),
            (
                "fn take(value: Int) {} fn main() { take(\"a\" + \"b\") }",
                "E3007",
            ),
            (
                "fn main() { let text = \"a\" + \"b\"; let number: Int = text }",
                "E3002",
            ),
        ] {
            let program = Parser::new().parse_source(source).unwrap();
            let errors = SemanticAnalyzer::new()
                .analyze(&program)
                .expect_err("concatenation is String");
            assert!(
                errors.items.iter().any(|error| error.code == code),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn inferred_concatenation_can_be_used_as_string() {
        let source = "fn join(value: String) -> String { return value + \"!\" } fn main() { let text = \"a\" + \"b\"; print(join(text + \"c\")); print(std::len(text)); print(text == \"ab\") }";
        let program = Parser::new().parse_source(source).unwrap();
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("inferred String");
        let output =
            crate::compiler::interpreter::run(&crate::compiler::ir::lower(&typed)).unwrap();
        assert_eq!(output, vec!["abc!", "2", "true"]);
    }

    #[test]
    fn expression_diagnostics_identify_the_failing_source() {
        for (body, code, spelling) in [
            ("print(missing)", "E3001", "missing"),
            ("let value = missing", "E3001", "missing"),
            ("return missing", "E3001", "missing"),
            ("print(9223372036854775808)", "E3012", "9223372036854775808"),
            ("print(true + 1)", "E3005", "true + 1"),
            ("std::len(42)", "E3007", "42"),
            ("helper(42)", "E3007", "42"),
            ("absent(1)", "E3004", "absent"),
            ("std::absent(1)", "E3004", "std::absent"),
            ("std::len()", "E3006", "std::len()"),
            ("42()", "E3003", "42"),
        ] {
            for prefix in ["fn main() {", "mod sample { fn private() {"] {
                let source = format!(
                    "fn helper(value: String) {{}}\n{prefix}\n  print(\"é\"); {body}\n}}{}",
                    if prefix.starts_with("mod") { "}" } else { "" }
                );
                let program = Parser::new().parse_source(&source).expect("valid syntax");
                let errors = SemanticAnalyzer::new()
                    .analyze(&program)
                    .expect_err("invalid body");
                let error = errors
                    .items
                    .iter()
                    .find(|error| error.code == code)
                    .unwrap();
                let start = source.rfind(spelling).unwrap();
                assert_eq!(error.span.start, start, "{source}: {error:?}");
                assert_eq!(error.span.end, start + spelling.len(), "{source}");
                assert_eq!(error.span.line, 2);
                let line_start = source[..start].rfind('\n').unwrap() + 1;
                assert_eq!(error.span.column, source[line_start..start].chars().count());
            }
        }
    }

    #[test]
    fn requires_parameter_annotations_in_all_functions() {
        for source in [
            "fn helper(value) {} fn main() {}",
            "export fn helper(value) {} fn main() {}",
            "mod sample { fn helper(value) {} } fn main() {}",
            "mod sample { export fn helper(value) {} } fn main() {}",
            include_str!("../../tests/fixtures/untyped-parameter.svr"),
        ] {
            let parsed = Parser::new()
                .parse_source(source)
                .expect("recoverable syntax");
            let diagnostics = SemanticAnalyzer::new()
                .analyze(&parsed)
                .expect_err("every function parameter requires a type");
            let missing_types: Vec<_> = diagnostics
                .items
                .iter()
                .filter(|diagnostic| diagnostic.code == "E3014")
                .collect();
            assert_eq!(missing_types.len(), 1, "{source}: {diagnostics:?}");
            assert_eq!(
                missing_types[0].message,
                "parameter `value` requires an explicit type annotation; write `value: Type`"
            );
            let span = missing_types[0].span;
            assert_eq!(&source[span.start..span.end], "value");
        }
    }

    #[test]
    fn reports_each_missing_parameter_annotation() {
        let source = "fn helper(first, typed: Int,\n    last) {} fn main() {}";
        let parsed = Parser::new()
            .parse_source(source)
            .expect("recoverable syntax");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&parsed)
            .expect_err("missing types");
        let missing: Vec<_> = diagnostics
            .items
            .iter()
            .filter(|diagnostic| diagnostic.code == "E3014")
            .map(|diagnostic| {
                let span = diagnostic.span;
                (&source[span.start..span.end], span.line, span.column)
            })
            .collect();
        assert_eq!(missing, vec![("first", 0, 10), ("last", 1, 4)]);
    }

    #[test]
    fn typed_parameters_preserve_local_inference() {
        let parsed = Parser::new()
            .parse_source(include_str!("../../examples/functions/main.svr"))
            .expect("valid function example");
        let typed = SemanticAnalyzer::new()
            .analyze(&parsed)
            .expect("inferred locals");
        let output = crate::compiler::interpreter::run(&crate::compiler::ir::lower(&typed))
            .expect("function example executes");
        assert_eq!(output, vec!["5"]);

        let invalid = Parser::new()
            .parse_source(
                "fn length(value: String) -> Int { return std::len(value) }
                fn main() { let number = 42; length(number) }",
            )
            .expect("valid syntax");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&invalid)
            .expect_err("wrong argument type");
        assert!(diagnostics
            .items
            .iter()
            .any(|diagnostic| diagnostic.code == "E3007"));
        assert!(!diagnostics
            .items
            .iter()
            .any(|diagnostic| diagnostic.code == "E3014"));
    }

    #[test]
    fn rejects_missing_required_return() {
        for source in [
            "fn value() -> Int {} fn main() {}",
            "fn value() -> String { let result = \"hi\" } fn main() {}",
            "mod values { export fn value() -> Float {} } fn main() {}",
            "mod values { fn value() -> Int { print(42) } } fn main() {}",
            "fn value(flag: Bool) -> Int { if (flag) { return 1 } } fn main() {}",
            "fn value(flag: Bool) -> Int { if (flag) { return 1 } else { print(2) } } fn main() {}",
            "fn value(flag: Bool) -> Int { while (flag) { return 1 } } fn main() {}",
        ] {
            let parsed = Parser::new().parse_source(source).expect("valid syntax");
            let errors = SemanticAnalyzer::new()
                .analyze(&parsed)
                .expect_err("value-returning function cannot fall through");
            assert!(errors.items.iter().any(|error| error.code == "E3013"));
        }
        for source in [
            "fn noop() -> Unit {} fn value() -> Int { return 42 } fn main() {}",
            "fn value(flag: Bool) -> Int { if (flag) { return 1 } else { return 2 } } fn main() {}",
            "fn value(flag: Bool, other: Bool) -> Int { if (flag) { if (other) { return 1 } else { return 2 } } else { return 3 } } fn main() {}",
            "fn value(flag: Bool) -> Int { if (flag) { return 1 } else if (false) { return 2 } else { return 3 } } fn main() {}",
        ] {
            let parsed = Parser::new().parse_source(source).expect("valid syntax");
            assert!(
                SemanticAnalyzer::new().analyze(&parsed).is_ok(),
                "{source}"
            );
        }
    }

    #[test]
    fn numeric_integer_literals_must_fit_i64() {
        for source in [
            "fn main() { print(9223372036854775808) }",
            "mod values { export fn bad() -> Int { return 99999999999999999999 } } fn main() {}",
        ] {
            let parsed = Parser::new().parse_source(source).expect("valid syntax");
            let errors = SemanticAnalyzer::new()
                .analyze(&parsed)
                .expect_err("out-of-range literals must be diagnosed");
            assert!(errors.items.iter().any(|error| error.code == "E3012"));
        }
    }

    #[test]
    fn mutable_int_bindings_reject_float_reassignment() {
        let program = Parser::new()
            .parse_source("fn main() { let mut count = 1; count = 1.5; }")
            .expect("valid syntax");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&program)
            .expect_err("an Int binding cannot be reassigned a Float");
        assert!(
            diagnostics.items.iter().any(|item| item.code == "E3002"),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn resolves_bindings_and_builtin_print() {
        let program = Parser::new()
            .parse_source("fn main() { let message = \"hi\"; print(message) }")
            .expect("source should parse");
        assert!(SemanticAnalyzer::new().analyze(&program).is_ok());
    }

    #[test]
    fn array_element_assignment_requires_mutable_direct_binding() {
        let source = "fn mutate() { let mut values = [1, 2]; values[0] = 9; print(values[0]); }
            fn main() { print(mutate()); }";
        let program = Parser::new().parse_source(source).expect("valid syntax");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("mutable array slot assignment is valid");
        let output = crate::compiler::interpreter::run(&crate::compiler::ir::lower(&typed))
            .expect("updated array should execute");
        assert_eq!(output, ["9", ""]);

        for (source, expected_code) in [
            ("fn main() { let values = [1]; values[0] = 2; }", "E3011"),
            (
                "fn main() { let mut values = [[1]]; values[0][0] = 2; }",
                "E3003",
            ),
            (
                "fn main() { let mut values = [1]; values[0] = \"two\"; }",
                "E3007",
            ),
        ] {
            let program = Parser::new().parse_source(source).expect("valid syntax");
            let diagnostics = SemanticAnalyzer::new()
                .analyze(&program)
                .expect_err("invalid array assignment");
            assert!(
                diagnostics
                    .items
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected_code),
                "{source}: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn reports_undefined_names() {
        let program = Parser::new()
            .parse_source("fn main() { print(missing) }")
            .expect("source should parse");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&program)
            .expect_err("source should fail semantic analysis");
        assert!(diagnostics.items.iter().any(|item| item.code == "E3001"));
    }

    #[test]
    fn checks_return_types() {
        let program = Parser::new()
            .parse_source("fn main() -> Int { return \"no\" }")
            .expect("source should parse");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&program)
            .expect_err("source should fail semantic analysis");
        assert!(diagnostics.items.iter().any(|item| item.code == "E3002"));
    }

    #[test]
    fn logical_operators_require_boolean_operands() {
        let valid = Parser::new()
            .parse_source("fn main() { let value = true || false && true; if (value) {} }")
            .expect("valid syntax");
        assert!(SemanticAnalyzer::new().analyze(&valid).is_ok());

        for source in [
            "fn main() { print(true && 1) }",
            "fn main() { print(\"yes\" || false) }",
            "fn main() { print(false && 1) }",
        ] {
            let program = Parser::new().parse_source(source).expect("valid syntax");
            let diagnostics = SemanticAnalyzer::new()
                .analyze(&program)
                .expect_err("logical operands must be Bool");
            assert!(
                diagnostics.items.iter().any(|item| item.code == "E3005"),
                "{source}: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn checks_function_call_arity_and_parameter_types() {
        let program = Parser::new()
            .parse_source("fn add(value: Int) -> Int { return value } fn main() { add(\"no\") }")
            .expect("source should parse");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&program)
            .expect_err("source should fail semantic analysis");
        assert!(diagnostics.items.iter().any(|item| item.code == "E3007"));
    }

    #[test]
    fn accepts_typed_local_bindings_and_numeric_widening() {
        let program = Parser::new()
            .parse_source(
                "fn main() { let scaled: Float = 2; let total = scaled + 3.5; print(total) }",
            )
            .expect("source should parse");
        assert!(SemanticAnalyzer::new().analyze(&program).is_ok());
    }

    #[test]
    fn resolves_module_exported_functions() {
        let program = Parser::new()
            .parse_source(
                "mod math { export fn add(a: Int, b: Int) -> Int { return a + b } } fn main() { print(math::add(2, 3)) }",
            )
            .expect("source should parse");
        assert!(SemanticAnalyzer::new().analyze(&program).is_ok());
    }

    #[test]
    fn rejects_invalid_module_function_bodies() {
        let cases = [
            ("fn bad() { print(missing) }", "E3001"),
            ("fn bad() -> Int { return \"wrong\" }", "E3002"),
            ("fn bad(value: Int, value: Int) {}", "E3011"),
            ("fn bad() { let value: Int = \"wrong\" }", "E3002"),
            ("fn bad() { missing() }", "E3004"),
            ("fn bad() { std::len() }", "E3006"),
            ("fn bad() { std::len(42) }", "E3007"),
            ("fn bad() { print(true + 1) }", "E3005"),
        ];
        for visibility in ["", "export "] {
            for (function, code) in cases {
                let source = format!("mod sample {{ {visibility}{function} }} fn main() {{}}");
                let program = Parser::new().parse_source(&source).expect("valid syntax");
                let diagnostics = SemanticAnalyzer::new()
                    .analyze(&program)
                    .expect_err(&source);
                assert!(
                    diagnostics.items.iter().any(|item| item.code == code),
                    "expected {code} for {source}: {diagnostics:?}"
                );
            }
        }
    }

    #[test]
    fn module_functions_have_independent_local_scopes() {
        let program = Parser::new()
            .parse_source(
                "mod sample { export fn first(value: Int) -> Int { return value }
                 export fn second() -> Int { return value } } fn main() {}",
            )
            .expect("valid syntax");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&program)
            .expect_err("parameters must not leak into another function");
        assert!(diagnostics.items.iter().any(|item| item.code == "E3001"));
    }

    #[test]
    fn module_main_is_an_ordinary_function() {
        let program = Parser::new()
            .parse_source(
                "mod sample { export fn main(value: Int) -> Int { return value } }
                 fn main() { print(sample::main(42)) }",
            )
            .expect("valid syntax");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("entry restrictions apply only to top-level main");
        let output = crate::compiler::interpreter::run(&crate::compiler::ir::lower(&typed))
            .expect("exported function should execute");
        assert_eq!(output, vec!["42"]);
    }

    #[test]
    fn module_example_checks_and_executes() {
        let program = Parser::new()
            .parse_source(include_str!("../../examples/modules/main.svr"))
            .expect("module example should parse");
        let typed = SemanticAnalyzer::new()
            .analyze(&program)
            .expect("module example should check");
        let output = crate::compiler::interpreter::run(&crate::compiler::ir::lower(&typed))
            .expect("module example should execute");
        assert_eq!(output, vec!["42"]);
    }

    #[test]
    fn resolves_std_library_calls() {
        let program = Parser::new()
            .parse_source("fn main() { std::println(42); let text = std::to_string(42); print(std::len(text)) }")
            .expect("source should parse");
        assert!(SemanticAnalyzer::new().analyze(&program).is_ok());
    }

    #[test]
    fn missing_record_fields_have_deterministic_diagnostic_order() {
        let source = "struct Point { z: Int, a: Int, m: Int } fn main() { Point {} }";
        let parsed = Parser::new().parse_source(source).unwrap();
        for _ in 0..16 {
            let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
            let messages: Vec<_> = errors
                .items
                .iter()
                .map(|item| item.message.as_str())
                .collect();
            assert_eq!(
                messages,
                [
                    "struct `Point` is missing field `a`",
                    "struct `Point` is missing field `m`",
                    "struct `Point` is missing field `z`",
                ]
            );
        }
    }

    #[test]
    fn invalid_record_literals_check_every_field_value_once() {
        for source in [
            "fn main() { Missing { a: missing_first, b: missing_second }; print(later); }",
            "struct Point { x: Int } fn main() { Point { x: 1, x: missing_first, extra: missing_second }; print(later); }",
            "type Scalar = Int; fn main() { Scalar { a: missing_first, b: missing_second }; print(later); }",
        ] {
            let parsed = Parser::new().parse_source(source).unwrap();
            let errors = SemanticAnalyzer::new().analyze(&parsed).unwrap_err();
            for name in ["missing_first", "missing_second", "later"] {
                let start = source.find(name).unwrap();
                assert_eq!(errors.items.iter().filter(|item| {
                    item.code == "E3001" && item.span.start == start
                        && item.span.end == start + name.len()
                }).count(), 1, "{name}: {errors:?}");
            }
        }
    }

    #[test]
    fn invalid_targets_do_not_hide_independent_expression_errors() {
        for (source, primary, names) in [
            (
                "fn main() { missing_target = missing_value; print(later); }",
                "E3001",
                vec!["missing_target", "missing_value", "later"],
            ),
            (
                "fn main() { let mut scalar = 1; scalar[0] = missing_value; print(later); }",
                "E3007",
                vec!["missing_value", "later"],
            ),
            (
                "fn main() { 42(missing_first, missing_second); print(later); }",
                "E3003",
                vec!["missing_first", "missing_second", "later"],
            ),
            (
                "fn main() { missing_array[missing_index](missing_argument); }",
                "E3003",
                vec!["missing_array", "missing_index", "missing_argument"],
            ),
        ] {
            let program = Parser::new().parse_source(source).unwrap();
            let diagnostics = SemanticAnalyzer::new().analyze(&program).unwrap_err();
            assert!(diagnostics.items.iter().any(|item| item.code == primary));
            for name in names {
                let start = source.find(name).unwrap();
                assert_eq!(
                    diagnostics
                        .items
                        .iter()
                        .filter(|item| {
                            item.code == "E3001"
                                && item.span.start == start
                                && item.span.end == start + name.len()
                        })
                        .count(),
                    1,
                    "missing or duplicated error for {name}: {diagnostics:?}"
                );
            }
        }
    }

    #[test]
    fn entry_return_aliases_obey_the_unit_contract() {
        for (declarations, return_type, body, valid) in [
            ("type Result = Unit;", "Result", "return;", true),
            ("type Base = Unit; type Result = Base;", "Result", "", true),
            ("type Result = Int;", "Result", "return 1;", false),
            (
                "type Base = Float; type Result = Base;",
                "Result",
                "return 1;",
                false,
            ),
            (
                "struct Result { value: Int }",
                "Result",
                "return Result { value: 1 };",
                false,
            ),
            (
                "mod types { type Result = Bool; }",
                "types::Result",
                "return true;",
                false,
            ),
        ] {
            let source = format!("{declarations} fn main() -> {return_type} {{ {body} }}");
            let program = Parser::new().parse_source(&source).unwrap();
            let result = SemanticAnalyzer::new().analyze(&program);
            if valid {
                assert!(result.is_ok(), "{source}: {result:?}");
            } else {
                let diagnostics = result.expect_err(&source);
                assert!(
                    diagnostics.items.iter().any(|item| item.code == "E3010"),
                    "{source}: {diagnostics:?}"
                );
            }
        }
    }

    #[test]
    fn rejects_duplicate_declarations_and_invalid_main() {
        let program = Parser::new()
            .parse_source(
                "fn helper() {} fn helper() {} \
                 fn main(value: Int, value: Int) -> Int {}",
            )
            .expect("source should parse");
        let diagnostics = SemanticAnalyzer::new()
            .analyze(&program)
            .expect_err("source should fail semantic analysis");
        assert!(diagnostics.items.iter().any(|item| item.code == "E3008"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E3009"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E3010"));
        assert!(diagnostics.items.iter().any(|item| item.code == "E3011"));
    }
}
