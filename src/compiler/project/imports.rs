//! Project-relative application imports; independent of executable modules.

/// Parse a complete single-line import after source comments are removed.
pub(super) fn parse(line: &str) -> Result<Option<Vec<String>>, &'static str> {
    let Some(rest) = line.strip_prefix("use") else {
        return Ok(None);
    };
    if !rest.is_empty() && !rest.starts_with(|ch: char| ch.is_ascii_whitespace()) {
        return Ok(None);
    }
    let path = rest.trim().strip_suffix(';').unwrap_or(rest.trim()).trim();
    let segments: Vec<_> = path.split('.').collect();
    if segments
        .iter()
        .any(|segment| !super::is_identifier(segment))
    {
        return Err("expected a project-relative import such as `use app.services`");
    }
    Ok(Some(segments.into_iter().map(str::to_owned).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_project_relative_imports() {
        assert_eq!(
            parse("use app.services;").unwrap(),
            Some(vec!["app".into(), "services".into()])
        );
        assert_eq!(parse("use\tmodels").unwrap(), Some(vec!["models".into()]));
        assert_eq!(parse("useful()"), Ok(None));
        for source in [
            "use",
            "use ../outside",
            "use app..models",
            "use /absolute",
            "use app.models trailing",
            "use app.models;;",
        ] {
            assert!(parse(source).is_err(), "{source}");
        }
    }
}
