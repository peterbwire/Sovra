//! Structured recognition of the service headers supported by project checking.

/// Body layout supported by the incremental service parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BodyStart {
    Pending,
    Open,
    Empty,
}

/// A service declaration header, excluding its source location.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Header {
    pub name: String,
    pub body: BodyStart,
}

/// Parse a comment-stripped line. Unrelated declarations return `Ok(None)`.
pub(super) fn parse_header(line: &str) -> Result<Option<Header>, &'static str> {
    let Some(rest) = line.strip_prefix("service") else {
        return Ok(None);
    };
    if !rest.is_empty() && !rest.starts_with(|ch: char| ch.is_ascii_whitespace()) {
        return Ok(None);
    }
    let rest = rest.trim_start();
    let end = rest
        .find(|ch: char| ch.is_ascii_whitespace() || ch == '{')
        .unwrap_or(rest.len());
    let name = &rest[..end];
    if !super::is_identifier(name) {
        return Err("service declaration requires a valid identifier");
    }
    let suffix = rest[end..].trim();
    let body = if suffix.is_empty() {
        BodyStart::Pending
    } else if suffix == "{" {
        BodyStart::Open
    } else if suffix
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .is_some_and(|value| value.trim().is_empty())
    {
        BodyStart::Empty
    } else {
        return Err("unsupported service header; use a multiline block or an empty block");
    };
    Ok(Some(Header {
        name: name.to_owned(),
        body,
    }))
}

/// Operation structure retained without interpreting application type syntax.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Operation<'a> {
    pub name: &'a str,
    pub parameters: &'a str,
    pub declarations: Vec<Parameter<'a>>,
    pub suffix: &'a str,
    pub return_annotation: Option<&'a str>,
    pub body: Option<&'a str>,
}

fn parse_suffix(suffix: &str) -> Result<(Option<&str>, Option<&str>), &'static str> {
    if suffix.is_empty() || suffix == ";" {
        return Ok((None, None));
    }
    if suffix.starts_with('{') {
        return Ok((None, Some(suffix)));
    }
    let rest = suffix
        .strip_prefix("->")
        .ok_or("expected a return annotation, body or end of service operation")?
        .trim();
    let end = rest.find(['{', ';']).unwrap_or(rest.len());
    let annotation = rest[..end].trim();
    if annotation.is_empty() {
        return Err("service return annotation cannot be empty");
    }
    validate_return_annotation(annotation)?;
    let tail = rest[end..].trim();
    if tail.is_empty() || tail == ";" {
        Ok((Some(annotation), None))
    } else if tail.starts_with('{') {
        Ok((Some(annotation), Some(tail)))
    } else {
        Err("unexpected text after service operation terminator")
    }
}

// Validate grouping only. Names and operators still belong to the future
// application type resolver; accepting text here does not establish a type.
fn validate_return_annotation(annotation: &str) -> Result<(), &'static str> {
    let mut delimiters = Vec::new();
    for (offset, character) in annotation.char_indices() {
        match character {
            '<' | '(' | '[' => delimiters.push(character),
            '>' if annotation[..offset].ends_with('-') => {}
            '>' | ')' | ']' => {
                let expected = match character {
                    '>' => '<',
                    ')' => '(',
                    _ => '[',
                };
                if delimiters.pop() != Some(expected) {
                    return Err("mismatched delimiter in service return annotation");
                }
            }
            ',' if delimiters.is_empty() => {
                return Err("unexpected comma in service return annotation")
            }
            _ => {}
        }
    }
    if delimiters.is_empty() {
        Ok(())
    } else {
        Err("unclosed delimiter in service return annotation")
    }
}

/// A parameter name and optional, unresolved annotation text.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Parameter<'a> {
    pub name: &'a str,
    pub annotation: Option<&'a str>,
}

fn parse_parameters(source: &str) -> Result<Vec<Parameter<'_>>, &'static str> {
    let mut result = Vec::new();
    let mut delimiters = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0;
    for (offset, ch) in source.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        match ch {
            '"' => quoted = true,
            '(' | '[' | '<' => delimiters.push(ch),
            '>' if source[..offset].ends_with('-') => {}
            ')' | ']' | '>' => {
                let expected = match ch {
                    ')' => '(',
                    ']' => '[',
                    _ => '<',
                };
                if delimiters.pop() != Some(expected) {
                    return Err("mismatched delimiter in service parameter");
                }
            }
            ',' if delimiters.is_empty() => {
                append_parameter(&source[start..offset], &mut result)?;
                start = offset + 1;
            }
            _ => {}
        }
    }
    if quoted || !delimiters.is_empty() {
        return Err("unclosed delimiter in service parameter");
    }
    let tail = source[start..].trim();
    if !tail.is_empty() {
        append_parameter(tail, &mut result)?;
    }
    Ok(result)
}

fn append_parameter<'a>(
    source: &'a str,
    parameters: &mut Vec<Parameter<'a>>,
) -> Result<(), &'static str> {
    let (name, annotation) = match source.trim().split_once(':') {
        Some((name, annotation)) => {
            let annotation = annotation.trim();
            if annotation.is_empty() {
                return Err("service parameter annotation cannot be empty");
            }
            (name.trim(), Some(annotation))
        }
        None => (source.trim(), None),
    };
    if !super::is_identifier(name) {
        return Err("service parameter requires a valid name");
    }
    if parameters.iter().any(|parameter| parameter.name == name) {
        return Err("duplicate service parameter name");
    }
    parameters.push(Parameter { name, annotation });
    Ok(())
}

/// Recognize the name and balanced parameter list of a single-line operation.
/// Parameter types and the return/body suffix remain opaque application syntax.
pub(super) fn parse_operation(line: &str) -> Result<Option<Operation<'_>>, &'static str> {
    let Some(rest) = line.strip_prefix("fn") else {
        return Ok(None);
    };
    if !rest.is_empty() && !rest.starts_with(|ch: char| ch.is_ascii_whitespace()) {
        return Ok(None);
    }
    let rest = rest.trim_start();
    let end = rest
        .find(|ch: char| ch.is_ascii_whitespace() || ch == '(')
        .unwrap_or(rest.len());
    let name = &rest[..end];
    if !super::is_identifier(name) {
        return Err("service operation requires a valid name");
    }
    let parameters = rest[end..]
        .trim_start()
        .strip_prefix('(')
        .ok_or("service operation requires a parenthesized parameter list")?;
    let mut depth = 1usize;
    let mut quoted = false;
    let mut escaped = false;
    for (offset, ch) in parameters.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        match ch {
            '"' => quoted = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let suffix = parameters[offset + 1..].trim();
                    let (return_annotation, body) = parse_suffix(suffix)?;
                    return Ok(Some(Operation {
                        name,
                        parameters: &parameters[..offset],
                        declarations: parse_parameters(&parameters[..offset])?,
                        suffix,
                        return_annotation,
                        body,
                    }));
                }
            }
            _ => {}
        }
    }
    Err("service operation parameter list must close on the same line")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_annotations_require_balanced_grouping() {
        for annotation in [
            "Result<Int, Error",
            "Result<Int, Error]",
            "Text>",
            "(Text]",
            "[Text",
            "Text, Int",
        ] {
            assert!(
                parse_operation(&format!("fn send() -> {annotation}")).is_err(),
                "{annotation}"
            );
        }
        for annotation in [
            "Result<Int, List<Text>>",
            "(Int, Text) -> Bool",
            "[Text]",
            "UnknownApplicationType",
        ] {
            let source = format!("fn send() -> {annotation};");
            let parsed = parse_operation(&source).unwrap().unwrap();
            assert_eq!(parsed.return_annotation, Some(annotation));
        }
    }

    #[test]
    fn separates_return_annotations_and_bodies() {
        for (suffix, annotation, body) in [
            ("", None, None),
            (";", None, None),
            ("-> Result<Int, Error>;", Some("Result<Int, Error>"), None),
            ("-> Text {", Some("Text"), Some("{")),
            ("{}", None, Some("{}")),
        ] {
            let source = format!("fn send() {suffix}");
            let operation = parse_operation(&source).unwrap().unwrap();
            assert_eq!(operation.return_annotation, annotation);
            assert_eq!(operation.body, body);
        }
        for suffix in ["->", "-> ;", "-> {", "unexpected", "-> Text; trailing"] {
            assert!(parse_operation(&format!("fn send() {suffix}")).is_err());
        }
    }

    #[test]
    fn parameter_declarations_preserve_nested_types_and_optional_annotations() {
        let operation = parse_operation(
            "fn send(value: Result<Int, List<Text>>, callback: (Int, Text) -> Bool, inferred,)",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            operation.declarations,
            vec![
                Parameter {
                    name: "value",
                    annotation: Some("Result<Int, List<Text>>")
                },
                Parameter {
                    name: "callback",
                    annotation: Some("(Int, Text) -> Bool")
                },
                Parameter {
                    name: "inferred",
                    annotation: None
                },
            ]
        );
        assert!(parse_operation("fn empty()")
            .unwrap()
            .unwrap()
            .declarations
            .is_empty());
    }

    #[test]
    fn rejects_malformed_parameter_declarations() {
        for parameters in [
            ",",
            "a,,b",
            "a, a",
            "a:",
            "bad-name: Int",
            "value: List<Int]",
            "value: List<Int",
            ": Int",
        ] {
            assert!(
                parse_operation(&format!("fn send({parameters})")).is_err(),
                "{parameters}"
            );
        }
    }

    #[test]
    fn operation_structure_preserves_application_types() {
        let parsed = parse_operation(
            "fn send(value: Result<Int, Error>, callback: (Int)) -> Result<Text, Error>",
        )
        .unwrap()
        .unwrap();
        assert_eq!(parsed.name, "send");
        assert_eq!(
            parsed.parameters,
            "value: Result<Int, Error>, callback: (Int)"
        );
        assert_eq!(parsed.suffix, "-> Result<Text, Error>");
        assert_eq!(
            parse_operation("fn quoted(value: Text = \")\")")
                .unwrap()
                .unwrap()
                .parameters,
            "value: Text = \")\""
        );
        assert_eq!(parse_operation("fn_extra()"), Ok(None));
    }

    #[test]
    fn malformed_operation_structure_is_rejected() {
        for line in [
            "fn",
            "fn ()",
            "fn bad-name()",
            "fn send",
            "fn send(value: Int",
            "fn send(callback: (Int)",
        ] {
            assert!(parse_operation(line).is_err(), "{line}");
        }
    }

    #[test]
    fn recognizes_supported_header_forms() {
        for (line, body) in [
            ("service mail", BodyStart::Pending),
            ("service\tmail {", BodyStart::Open),
            ("service mail{ \t }", BodyStart::Empty),
        ] {
            assert_eq!(
                parse_header(line),
                Ok(Some(Header {
                    name: "mail".into(),
                    body
                }))
            );
        }
        assert_eq!(parse_header("services: [mail]"), Ok(None));
        assert_eq!(parse_header("service_extra {}"), Ok(None));
    }

    #[test]
    fn rejects_missing_invalid_names_and_unsupported_bodies() {
        for line in [
            "service",
            "service {}",
            "service bad-name {}",
            "service 123 {}",
            "service mail unexpected",
            "service mail { fn send() }",
        ] {
            assert!(parse_header(line).is_err(), "{line}");
        }
    }
}
