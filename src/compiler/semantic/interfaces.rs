//! Copy exported metadata with canonical types and precharged string payloads.

use super::{collect_named_types, diagnostic, Diagnostics, Function, HashMap, Program, Span, Type};
use crate::compiler::{
    ast::{Parameter, StructDeclaration, StructField},
    resources::interface_strings::InterfaceStrings,
};
use std::collections::BTreeMap;

/// Immutable type-resolution context for one prepared package. It is reused for
/// exports and every consumer import, without rescanning the owner's declarations.
pub(crate) struct InterfaceTypes {
    known: HashMap<String, Type>,
}

impl InterfaceTypes {
    pub(crate) fn new(owner: &Program) -> Result<Self, Diagnostics> {
        let mut diagnostics = Diagnostics::new();
        let known = collect_named_types(owner, &mut diagnostics);
        if diagnostics.is_empty() {
            Ok(Self { known })
        } else {
            Err(diagnostics)
        }
    }
}

fn normalized_type(
    name: &str,
    span: Span,
    known: &HashMap<String, Type>,
    records: &HashMap<String, String>,
    diagnostics: &mut Diagnostics,
    strings: &mut InterfaceStrings,
) -> Result<String, Diagnostics> {
    let resolved = Type::from_name_with_known(name, Some(known)).canonicalize(known);
    let mut leaf = &resolved;
    let mut arrays = 0;
    while let Type::Array(inner) = leaf {
        arrays += 1;
        leaf = inner;
    }
    let canonical = match leaf {
        Type::Unit => "Unit",
        Type::Bool => "Bool",
        Type::Int => "Int",
        Type::Float => "Float",
        Type::String => "String",
        Type::Unknown => {
            diagnostics.push_semantic(diagnostic(
                "E3017",
                format_args!("unresolved type `{name}` in imported signature"),
                span,
            ));
            name
        }
        Type::Named(record) if records.contains_key(record) => &records[record],
        Type::Named(_) | Type::Array(_) => {
            diagnostics.push_semantic(diagnostic("E4116", format_args!("type `{name}` is private or unsupported in an exported interface; records must be explicitly exported"), span));
            name
        }
    };
    if arrays == 0 {
        return strings.copy(canonical, span);
    }
    strings.charge(canonical.len() + 2 * arrays, span)?;
    Ok(format!(
        "{}{}{}",
        "[".repeat(arrays),
        canonical,
        "]".repeat(arrays)
    ))
}

pub(crate) fn normalize_imported_signatures(
    types: &InterfaceTypes,
    functions: &[&Function],
    records: &HashMap<String, String>,
    strings: &mut InterfaceStrings,
) -> Result<Vec<Function>, Diagnostics> {
    let mut diagnostics = Diagnostics::new();
    let mut signatures = Vec::new();
    for function in functions {
        let name = strings.copy(&function.name, function.span)?;
        let mut parameters = Vec::new();
        for parameter in &function.parameters {
            parameters.push(Parameter {
                name: strings.copy(&parameter.name, parameter.span)?,
                type_name: parameter
                    .type_name
                    .as_deref()
                    .map(|name| {
                        normalized_type(
                            name,
                            parameter.type_span.unwrap_or(parameter.span),
                            &types.known,
                            records,
                            &mut diagnostics,
                            strings,
                        )
                    })
                    .transpose()?,
                type_span: parameter.type_span,
                span: parameter.span,
            });
        }
        let return_type = function
            .return_type
            .as_deref()
            .map(|name| {
                normalized_type(
                    name,
                    function.return_type_span.unwrap_or(function.span),
                    &types.known,
                    records,
                    &mut diagnostics,
                    strings,
                )
            })
            .transpose()?;
        signatures.push(Function {
            name,
            parameters,
            return_type,
            is_exported: function.is_exported,
            return_type_span: function.return_type_span,
            body: Vec::new(),
            span: function.span,
        });
        if diagnostics.limit_reached("E3019") {
            return Err(diagnostics);
        }
    }
    if diagnostics.is_empty() {
        Ok(signatures)
    } else {
        Err(diagnostics)
    }
}

pub(crate) fn exported_record_fields(
    owner: &Program,
    types: &InterfaceTypes,
    records: &HashMap<String, String>,
    imported_spans: &BTreeMap<String, Span>,
    strings: &mut InterfaceStrings,
) -> Result<Vec<StructDeclaration>, Diagnostics> {
    let mut diagnostics = Diagnostics::new();
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
        let context = imported_spans.get(&record.name).copied();
        let span = context.unwrap_or(record.span);
        let name = strings.copy(&records[&identity], span)?;
        let mut fields = Vec::new();
        for field in &record.fields {
            fields.push(StructField {
                name: strings.copy(&field.name, context.unwrap_or(field.span))?,
                type_name: normalized_type(
                    &field.type_name,
                    context.unwrap_or(field.type_span.unwrap_or(field.span)),
                    &types.known,
                    records,
                    &mut diagnostics,
                    strings,
                )?,
                type_span: field.type_span,
                span: field.span,
            });
        }
        result.push(StructDeclaration {
            name,
            fields,
            is_exported: record.is_exported,
            span: record.span,
        });
        if diagnostics.limit_reached("E3019") {
            return Err(diagnostics);
        }
    }
    if diagnostics.is_empty() {
        Ok(result)
    } else {
        Err(diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::parser::Parser;

    #[test]
    fn reusable_contexts_keep_owner_types_and_charge_every_copy() {
        let source =
            "type Number = Int; mod api { export fn f(v: Number) -> Number { return v; } }";
        let mut owner = Parser::new().parse_source(source).unwrap();
        let int_types = InterfaceTypes::new(&owner).unwrap();
        owner.type_declarations[0].target = "Float".into();
        let float_types = InterfaceTypes::new(&owner).unwrap();
        let functions = [&owner.modules[0].functions[0]];
        let records = HashMap::new();
        let mut strings = InterfaceStrings::new(Some(16));
        // f + v + Int + Int is eight copied bytes per normalization.
        for _ in 0..2 {
            let signatures =
                normalize_imported_signatures(&int_types, &functions, &records, &mut strings)
                    .unwrap();
            assert_eq!(
                signatures[0].parameters[0].type_name.as_deref(),
                Some("Int")
            );
            assert_eq!(signatures[0].return_type.as_deref(), Some("Int"));
        }
        assert_eq!(
            normalize_imported_signatures(&int_types, &functions, &records, &mut strings)
                .unwrap_err()
                .items[0]
                .code,
            "E3024"
        );
        let mut unlimited = InterfaceStrings::new(None);
        let signatures =
            normalize_imported_signatures(&float_types, &functions, &records, &mut unlimited)
                .unwrap();
        assert_eq!(
            signatures[0].parameters[0].type_name.as_deref(),
            Some("Float")
        );
        assert_eq!(signatures[0].return_type.as_deref(), Some("Float"));
    }
}
