//! Name resolution and basic type checking for M3.

use std::collections::{HashMap, HashSet};

use crate::compiler::ast::{Expression, ExpressionKind, Function, Program, Statement};
use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};
use crate::compiler::stdlib;

mod type_scopes;

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

impl Type {
    /// Resolve a type name against the built-in scalar types and a caller-supplied
    /// registry of declared named types. This keeps the checker forward-compatible
    /// with user-defined types without changing the existing source-language checks.
    pub fn from_name_with_known(
        name: &str,
        known_named_types: Option<&HashMap<String, Type>>,
    ) -> Self {
        match name {
            "Unit" => Self::Unit,
            "Bool" => Self::Bool,
            "Int" => Self::Int,
            "Float" => Self::Float,
            "String" => Self::String,
            _ => known_named_types
                .and_then(|known| known.get(name).cloned())
                .unwrap_or(Self::Unknown),
        }
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
    pub fn analyze(&self, program: &Program) -> Result<TypedProgram, Diagnostics> {
        self.analyze_with_imports(program, &[])
    }

    pub(crate) fn analyze_with_imports<'a>(
        &self,
        program: &'a Program,
        imports: &[(String, &'a Function)],
    ) -> Result<TypedProgram, Diagnostics> {
        let mut diagnostics = Diagnostics::new();
        if let Err(span) = super::limits::check_program_depth(program) {
            diagnostics.push(diagnostic("E3018", super::limits::depth_message(), span));
            return Err(diagnostics);
        }
        let mut resolved_program = type_scopes::resolve(program)?;
        let program = &resolved_program;
        let named_types = collect_named_types(program, &mut diagnostics);
        let struct_fields = collect_struct_fields(program, &named_types, &mut diagnostics);
        let mut declared_functions = HashMap::new();
        for function in &program.functions {
            check_builtin_collision(&function.name, function.span, &mut diagnostics);
            if declared_functions
                .insert(function.name.clone(), function.span)
                .is_some()
            {
                diagnostics.push(diagnostic(
                    "E3008",
                    format!("duplicate function `{}`", function.name),
                    function.span,
                ));
            }
        }
        let mut module_names = HashMap::new();
        for module in &program.modules {
            let mut seen = HashMap::new();
            for function in &module.functions {
                check_builtin_collision(
                    &format!("{}::{}", module.name, function.name),
                    function.span,
                    &mut diagnostics,
                );
                if seen.insert(function.name.clone(), function.span).is_some() {
                    diagnostics.push(diagnostic(
                        "E3008",
                        format!(
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
                diagnostics.push(diagnostic(
                    "E3008",
                    format!("duplicate module `{}`", module.name),
                    module.span,
                ));
            }
        }
        let mut functions: HashMap<String, &Function> = program
            .functions
            .iter()
            .map(|function| (function.name.clone(), function))
            .collect();
        for (name, function) in imports {
            if functions.insert(name.clone(), function).is_some() {
                diagnostics.push(diagnostic(
                    "E3008",
                    format!("duplicate imported function `{name}`"),
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
            if function.name == "main" && !function.parameters.is_empty() {
                diagnostics.push(diagnostic(
                    "E3009",
                    "entry function `main` cannot declare parameters",
                    function.span,
                ));
            }
            if function.name == "main"
                && function.return_type.as_deref().is_some_and(|return_type| {
                    !matches!(type_from_name(return_type), Type::Unit | Type::Unknown)
                })
            {
                diagnostics.push(diagnostic(
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
            );
        }
        for module in &program.modules {
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

fn collect_named_types(program: &Program, diagnostics: &mut Diagnostics) -> HashMap<String, Type> {
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
        if matches!(name, "Unit" | "Bool" | "Int" | "Float" | "String")
            || aliases.contains_key(name)
        {
            diagnostics.push(diagnostic(
                "E3008",
                format!("duplicate type `{name}`"),
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
        if matches!(name, "Unit" | "Bool" | "Int" | "Float" | "String")
            || structured_names.contains(name)
        {
            diagnostics.push(diagnostic(
                "E3008",
                format!("duplicate type `{name}`"),
                declaration.span,
            ));
            return;
        }
        structured_names.insert(name.to_owned());
    }

    for declaration in &program.type_declarations {
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
        insert_struct(
            &declaration.name,
            declaration,
            &mut structured_names,
            diagnostics,
        );
    }
    for module in &program.modules {
        for declaration in &module.struct_declarations {
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
        if structured_names.contains(*alias_name) {
            diagnostics.push(diagnostic(
                "E3008",
                format!("duplicate type `{alias_name}`"),
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

    let mut resolved = HashMap::new();
    for name in structured_names.iter() {
        resolved.insert(name.clone(), Type::Named(name.clone()));
    }

    for name in ordered_aliases {
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
            diagnostics.push(diagnostic(
                "E3017",
                format!(
                    "type `{}` creates a recursive alias cycle",
                    declaration.name
                ),
                declaration.span,
            ));
            break Type::Unknown;
        }
        path.push(current);
        let target = aliases[current].as_str();
        if target == current {
            diagnostics.push(diagnostic(
                "E3017",
                format!("type `{current}` cannot alias itself"),
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
            diagnostics.push(diagnostic("E3017", format!("unknown type `{target}`; expected Unit, Bool, Int, Float, String or a declared alias"), declaration.span));
            break Type::Unknown;
        }
    };
    for alias in path {
        resolved.insert(alias.to_owned(), result.clone());
    }
    result
}
fn collect_struct_fields(
    program: &Program,
    named_types: &HashMap<String, Type>,
    diagnostics: &mut Diagnostics,
) -> HashMap<String, HashMap<String, Type>> {
    let mut structs = HashMap::new();
    let mut seen = HashSet::new();
    let mut insert_struct = |name: &str, declaration: &crate::compiler::ast::StructDeclaration| {
        if !seen.insert(name.to_owned()) {
            diagnostics.push(diagnostic(
                "E3008",
                format!("duplicate type `{name}`"),
                declaration.span,
            ));
            return;
        }
        let mut fields = HashMap::new();
        let mut seen_fields = HashSet::new();
        for field in &declaration.fields {
            if !seen_fields.insert(field.name.clone()) {
                diagnostics.push(diagnostic(
                    "E3008",
                    format!("duplicate field `{}` in struct `{name}`", field.name),
                    field.span,
                ));
                continue;
            }
            let field_type = Type::from_name_with_known(&field.type_name, Some(named_types));
            if field_type == Type::Unknown {
                diagnostics.push(diagnostic(
                    "E3017",
                    format!(
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
// Primitive aliases normalize to primitives; exported records normalize to the
// package-qualified identity supplied by the linker, never consumer spellings.
pub(crate) fn normalize_imported_signatures(
    owner: &Program,
    functions: &[&Function],
    records: &HashMap<String, String>,
) -> Result<Vec<Function>, Diagnostics> {
    let mut diagnostics = Diagnostics::new();
    let known = collect_named_types(owner, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let mut normalize = |name: &str, span: Span| -> String {
        let resolved = Type::from_name_with_known(name, Some(&known)).canonicalize(&known);
        let canonical = match resolved {
            Type::Unit => "Unit",
            Type::Bool => "Bool",
            Type::Int => "Int",
            Type::Float => "Float",
            Type::String => "String",
            Type::Unknown => {
                diagnostics.push(diagnostic(
                    "E3017",
                    format!("unresolved type `{name}` in imported signature"),
                    span,
                ));
                return name.to_owned();
            }
            Type::Named(ref record) if records.contains_key(record) => {
                return records[record].clone();
            }
            Type::Named(_) | Type::Array(_) => {
                diagnostics.push(diagnostic("E4116", format!("type `{name}` is private or unsupported in an exported interface; records must be explicitly exported"), span));
                return name.to_owned();
            }
        };
        canonical.to_owned()
    };
    let mut signatures = Vec::new();
    for function in functions {
        let mut signature = (*function).clone();
        signature.body.clear();
        for parameter in &mut signature.parameters {
            if let Some(name) = &parameter.type_name {
                parameter.type_name = Some(normalize(
                    name,
                    parameter.type_span.unwrap_or(parameter.span),
                ));
            }
        }
        if let Some(name) = &signature.return_type {
            signature.return_type = Some(normalize(
                name,
                signature.return_type_span.unwrap_or(signature.span),
            ));
        }
        signatures.push(signature);
    }
    if diagnostics.is_empty() {
        Ok(signatures)
    } else {
        Err(diagnostics)
    }
}

pub(crate) fn exported_record_fields(
    owner: &Program,
    records: &HashMap<String, String>,
) -> Result<Vec<crate::compiler::ast::StructDeclaration>, Diagnostics> {
    let mut result = Vec::new();
    for (identity, record) in owner
        .struct_declarations
        .iter()
        .map(|record| (record.name.clone(), record))
        .chain(owner.modules.iter().flat_map(|module| {
            module
                .struct_declarations
                .iter()
                .map(move |record| (format!("{}::{}", module.name, record.name), record))
        }))
        .filter(|(_, record)| record.is_exported)
    {
        // Reuse signature resolution for field annotations, keeping their source spans.
        let signature = Function {
            name: record.name.clone(),
            is_exported: true,
            parameters: record
                .fields
                .iter()
                .map(|field| crate::compiler::ast::Parameter {
                    name: field.name.clone(),
                    type_name: Some(field.type_name.clone()),
                    type_span: field.type_span,
                    span: field.span,
                })
                .collect(),
            return_type: None,
            return_type_span: None,
            body: Vec::new(),
            span: record.span,
        };
        let normalized = normalize_imported_signatures(owner, &[&signature], records)?;
        let mut record = record.clone();
        record.name = records[&identity].clone();
        for (field, parameter) in record.fields.iter_mut().zip(&normalized[0].parameters) {
            field.type_name = parameter.type_name.clone().unwrap();
        }
        result.push(record);
    }
    Ok(result)
}

fn check_builtin_collision(name: &str, span: Span, diagnostics: &mut Diagnostics) {
    if stdlib::lookup(name).is_some() {
        diagnostics.push(diagnostic(
            "E3016",
            format!("function `{name}` conflicts with a builtin; choose a different name"),
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
) {
    let mut scope = LocalScope::default();
    for parameter in &function.parameters {
        if scope.types.contains_key(&parameter.name) {
            diagnostics.push(diagnostic(
                "E3011",
                format!("duplicate parameter `{}`", parameter.name),
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
                diagnostics.push(diagnostic(
                    "E3014",
                    format!(
                        "parameter `{name}` requires an explicit type annotation; write `{name}: Type`",
                        name = parameter.name
                    ),
                    parameter.span,
                ));
                Type::Unknown
            }
        };
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
    if !matches!(expected_return, Type::Unit | Type::Unknown)
        && !block_guarantees_return(&function.body)
    {
        diagnostics.push(diagnostic(
            "E3013",
            format!(
                "function `{}` must return {expected_return:?}",
                function.name
            ),
            function.span,
        ));
    }
    for statement in &function.body {
        check_statement(
            statement,
            &mut scope,
            functions,
            named_types,
            struct_fields,
            &expected_return,
            diagnostics,
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

fn check_statement(
    statement: &Statement,
    scope: &mut LocalScope,
    functions: &HashMap<String, &Function>,
    named_types: &HashMap<String, Type>,
    struct_fields: &HashMap<String, HashMap<String, Type>>,
    expected_return: &Type,
    diagnostics: &mut Diagnostics,
) {
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
            );
            let declared_type = type_name
                .as_deref()
                .map(|name| {
                    check_annotation(name, type_span.unwrap_or(*span), named_types, diagnostics)
                })
                .unwrap_or(value_type.clone());
            if !types_compatible(&declared_type, &value_type, named_types) {
                diagnostics.push(diagnostic(
                    "E3002",
                    format!(
                        "binding type mismatch for `{name}`: expected {declared_type:?}, found {value_type:?}"
                    ),
                    *span,
                ));
            }
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
                    diagnostics.push(diagnostic(
                        "E3001",
                        format!("undefined variable `{name}`"),
                        target.span,
                    ));
                    return;
                };
                if !scope.mutable_names.contains(name) {
                    diagnostics.push(diagnostic(
                        "E3011",
                        format!("variable `{name}` is not mutable"),
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
                );
                if !types_compatible(&current_type, &value_type, named_types) {
                    diagnostics.push(diagnostic(
                            "E3002",
                            format!(
                                "assignment type mismatch for `{name}`: expected {current_type:?}, found {value_type:?}"
                            ),
                            *span,
                        ));
                }
                scope.types.insert(name.clone(), current_type);
            }
            ExpressionKind::Index { target, index } => {
                if let ExpressionKind::Identifier(name) = &target.kind {
                    if scope.types.contains_key(name) && !scope.mutable_names.contains(name) {
                        diagnostics.push(diagnostic(
                            "E3011",
                            format!("variable `{name}` is not mutable"),
                            target.span,
                        ));
                    }
                } else {
                    diagnostics.push(diagnostic(
                        "E3003",
                        "indexed assignment must target a mutable array binding directly",
                        target.span,
                    ));
                }
                let target_type = check_expression(
                    target,
                    &scope.types,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                );
                let index_type = check_expression(
                    index,
                    &scope.types,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                );
                if !types_compatible(&Type::Int, &index_type, named_types) {
                    diagnostics.push(diagnostic(
                        "E3005",
                        format!("array index must be Int, found {index_type:?}"),
                        index.span,
                    ));
                }
                let array_type = target_type.canonicalize(named_types);
                let element_type = match &array_type {
                    Type::Array(element_type) => element_type.as_ref(),
                    _ => {
                        diagnostics.push(diagnostic(
                            "E3007",
                            format!("indexing requires an array, found {array_type:?}"),
                            target.span,
                        ));
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
                );
                if !types_compatible(element_type, &value_type, named_types) {
                    diagnostics.push(diagnostic(
                            "E3007",
                            format!(
                                "assignment type mismatch for index target: expected {element_type:?}, found {value_type:?}"
                            ),
                            value.span,
                        ));
                }
            }
            _ => {
                diagnostics.push(diagnostic(
                    "E3003",
                    "assignment target must be a mutable variable or array index",
                    *span,
                ));
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
                    )
                })
                .unwrap_or(Type::Unit);
            if !types_compatible(expected_return, &actual, named_types) {
                diagnostics.push(diagnostic(
                    "E3002",
                    format!("return type mismatch: expected {expected_return:?}, found {actual:?}"),
                    *span,
                ));
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
            );
            if !types_compatible(&Type::Bool, &condition_type, named_types) {
                diagnostics.push(diagnostic(
                    "E3005",
                    format!("if condition must be Bool, found {condition_type:?}"),
                    *span,
                ));
            }
            let mut then_scope = scope.clone();
            for statement in then_block {
                check_statement(
                    statement,
                    &mut then_scope,
                    functions,
                    named_types,
                    struct_fields,
                    expected_return,
                    diagnostics,
                );
            }
            if let Some(else_block) = else_block {
                let mut else_scope = scope.clone();
                for statement in else_block {
                    check_statement(
                        statement,
                        &mut else_scope,
                        functions,
                        named_types,
                        struct_fields,
                        expected_return,
                        diagnostics,
                    );
                }
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
            );
            if !types_compatible(&Type::Bool, &condition_type, named_types) {
                diagnostics.push(diagnostic(
                    "E3005",
                    format!("while condition must be Bool, found {condition_type:?}"),
                    *span,
                ));
            }
            let mut loop_scope = scope.clone();
            for statement in body {
                check_statement(
                    statement,
                    &mut loop_scope,
                    functions,
                    named_types,
                    struct_fields,
                    expected_return,
                    diagnostics,
                );
            }
        }
        Statement::Expression(expression) => {
            check_expression(
                expression,
                &scope.types,
                functions,
                named_types,
                struct_fields,
                diagnostics,
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
) -> Type {
    let span = expression.span;
    match &expression.kind {
        ExpressionKind::String(_) => Type::String,
        ExpressionKind::Integer(value) => {
            if value.parse::<i64>().is_err() {
                diagnostics.push(diagnostic(
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
            diagnostics.push(diagnostic(
                "E3001",
                format!("undefined variable `{name}`"),
                span,
            ));
            Type::Unknown
        }),
        ExpressionKind::QualifiedName { path } => {
            let qualified = path.join("::");
            if stdlib::lookup(&qualified).is_some() || functions.contains_key(&qualified) {
                diagnostics.push(diagnostic(
                    "E3015",
                    format!("function `{qualified}` cannot be used as a value; call it with parentheses and the required arguments"),
                    span,
                ));
                Type::Unknown
            } else {
                diagnostics.push(diagnostic(
                    "E3004",
                    format!("undefined function `{qualified}`"),
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
                    diagnostics.push(diagnostic(
                        "E3004",
                        format!("undefined struct `{type_name}`"),
                        span,
                    ));
                    return Type::Unknown;
                }
            };
            let declared_fields: HashSet<_> = type_fields.keys().cloned().collect();
            let mut seen_fields = HashSet::new();
            for (field_name, value) in fields {
                if !seen_fields.insert(field_name.clone()) {
                    diagnostics.push(diagnostic(
                        "E3008",
                        format!("duplicate field `{field_name}` in struct literal `{type_name}`"),
                        value.span,
                    ));
                    continue;
                }
                let field_type = type_fields.get(field_name).cloned().unwrap_or_else(|| {
                    diagnostics.push(diagnostic(
                        "E3001",
                        format!("struct `{type_name}` has no field `{field_name}`"),
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
                );
                if !types_compatible(&field_type, &value_type, named_types) {
                    diagnostics.push(diagnostic(
                        "E3007",
                        format!(
                            "field `{field_name}` on `{type_name}` expects {field_type:?}, found {value_type:?}"
                        ),
                        value.span,
                    ));
                }
            }
            for field_name in declared_fields {
                if !fields.iter().any(|(name, _)| name == &field_name) {
                    diagnostics.push(diagnostic(
                        "E3001",
                        format!("struct `{type_name}` is missing field `{field_name}`"),
                        span,
                    ));
                }
            }
            resolved
        }
        ExpressionKind::ArrayLiteral(items) => {
            let mut element_type = Type::Unknown;
            for item in items {
                let item_type = check_expression(
                    item,
                    scope,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                );
                if element_type == Type::Unknown {
                    element_type = item_type.clone();
                } else if element_type == Type::Int && item_type == Type::Float {
                    element_type = Type::Float;
                } else if !types_compatible(&element_type, &item_type, named_types) {
                    diagnostics.push(diagnostic(
                        "E3007",
                        format!("array elements must share a compatible type, found {element_type:?} and {item_type:?}"),
                        item.span,
                    ));
                }
            }
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
            );
            let struct_type = match receiver_type.canonicalize(named_types) {
                Type::Named(name) => name,
                _ => {
                    diagnostics.push(diagnostic(
                        "E3007",
                        format!("field access `{field}` requires a named struct instance"),
                        span,
                    ));
                    return Type::Unknown;
                }
            };
            struct_fields
                .get(&struct_type)
                .and_then(|fields| fields.get(field).cloned())
                .unwrap_or_else(|| {
                    diagnostics.push(diagnostic(
                        "E3001",
                        format!("struct `{struct_type}` has no field `{field}`"),
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
            );
            let index_type = check_expression(
                index,
                scope,
                functions,
                named_types,
                struct_fields,
                diagnostics,
            );
            if !types_compatible(&Type::Int, &index_type, named_types) {
                diagnostics.push(diagnostic(
                    "E3005",
                    format!("array index must be Int, found {index_type:?}"),
                    index.span,
                ));
            }
            match target_type.canonicalize(named_types) {
                Type::Array(element_type) => *element_type,
                _ => {
                    diagnostics.push(diagnostic(
                        "E3007",
                        format!("indexing requires an array, found {target_type:?}"),
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
                    diagnostics.push(diagnostic(
                        "E3003",
                        "call target must be a function name",
                        callee.span,
                    ));
                    return Type::Unknown;
                }
            };
            let Some(name) = name else {
                return Type::Unknown;
            };
            if let Some(function) = stdlib::lookup(&name) {
                check_std_call(StdCallContext {
                    function,
                    source_name: &name,
                    arguments,
                    scope,
                    functions,
                    named_types,
                    struct_fields,
                    diagnostics,
                    span,
                });
                return type_from_name(function.return_type);
            }
            if let Some(function) = functions.get(&name) {
                if arguments.len() != function.parameters.len() {
                    diagnostics.push(diagnostic(
                        "E3006",
                        format!(
                            "function `{name}` expects {} argument(s), found {}",
                            function.parameters.len(),
                            arguments.len()
                        ),
                        span,
                    ));
                }
                for (argument, parameter) in arguments.iter().zip(&function.parameters) {
                    let argument_type = check_expression(
                        argument,
                        scope,
                        functions,
                        named_types,
                        struct_fields,
                        diagnostics,
                    );
                    if let Some(parameter_type) = parameter.type_name.as_deref() {
                        let expected =
                            Type::from_name_with_known(parameter_type, Some(named_types));
                        if !types_compatible(&expected, &argument_type, named_types) {
                            diagnostics.push(diagnostic(
                                "E3007",
                                format!(
                                    "argument type mismatch for `{name}`: expected {expected:?}, found {argument_type:?}"
                                ),
                                argument.span,
                            ));
                        }
                    }
                }
                for argument in arguments.iter().skip(function.parameters.len()) {
                    check_expression(
                        argument,
                        scope,
                        functions,
                        named_types,
                        struct_fields,
                        diagnostics,
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
                    );
                }
                diagnostics.push(diagnostic(
                    "E3004",
                    format!("undefined function `{name}`"),
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
            );
            let right_type = check_expression(
                right,
                scope,
                functions,
                named_types,
                struct_fields,
                diagnostics,
            );
            if matches!(operator.as_str(), "&&" | "||") {
                if !types_compatible(&Type::Bool, &left_type, named_types)
                    || !types_compatible(&Type::Bool, &right_type, named_types)
                {
                    diagnostics.push(diagnostic(
                        "E3005",
                        format!("operator `{operator}` requires Bool operands"),
                        span,
                    ));
                }
                return Type::Bool;
            }
            let comparable = numeric_or_string_comparison_compatible(&left_type, &right_type)
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
            if comparable {
                Type::Bool
            } else if arithmetic {
                arithmetic_result_type(&left_type, &right_type)
            } else {
                diagnostics.push(diagnostic(
                    "E3005",
                    format!("operator `{operator}` cannot be applied to these types"),
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
}

fn check_std_call(context: StdCallContext<'_>) {
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
    } = context;

    if arguments.len() != function.parameters.len() {
        diagnostics.push(diagnostic(
            "E3006",
            format!(
                "function `{source_name}` expects {} argument(s), found {}",
                function.parameters.len(),
                arguments.len()
            ),
            span,
        ));
    }
    for (argument, expected) in arguments.iter().zip(function.parameters) {
        let argument_type = check_expression(
            argument,
            scope,
            functions,
            named_types,
            struct_fields,
            diagnostics,
        );
        if stdlib::is_any_type(expected) {
            continue;
        }
        let expected = Type::from_name_with_known(expected, Some(named_types));
        if !types_compatible(&expected, &argument_type, named_types) {
            diagnostics.push(diagnostic(
                "E3007",
                format!(
                    "argument type mismatch for `{source_name}`: expected {expected:?}, found {argument_type:?}"
                ),
                argument.span,
            ));
        }
    }
    for argument in arguments.iter().skip(function.parameters.len()) {
        check_expression(
            argument,
            scope,
            functions,
            named_types,
            struct_fields,
            diagnostics,
        );
    }
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
        diagnostics.push(diagnostic(
            "E3017",
            format!(
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

fn diagnostic(code: &'static str, message: impl Into<String>, span: Span) -> Diagnostic {
    Diagnostic {
        source_file: None,
        severity: Severity::Error,
        code,
        message: message.into(),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
