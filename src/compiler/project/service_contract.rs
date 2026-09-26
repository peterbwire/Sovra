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

#[cfg(test)]
mod tests {
    use super::*;

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
