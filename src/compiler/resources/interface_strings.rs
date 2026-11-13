//! Cumulative interface payload materialization, independent of IR storage.

use super::{Diagnostic, Diagnostics, Severity, Span};

pub(crate) struct InterfaceStrings {
    limit: Option<usize>,
    used: usize,
}

impl InterfaceStrings {
    pub(crate) fn new(limit: Option<usize>) -> Self {
        Self { limit, used: 0 }
    }

    pub(crate) fn charge(&mut self, bytes: usize, span: Span) -> Result<(), Diagnostics> {
        if let Some(limit) = self.limit {
            if bytes > limit.saturating_sub(self.used) {
                let mut errors = Diagnostics::new();
                errors.push_bounded(Diagnostic {
                    severity: Severity::Error,
                    code: "E3024",
                    message: format!("aggregate interface string byte limit of {limit} exceeded"),
                    source_file: None,
                    span,
                });
                return Err(errors);
            }
            self.used += bytes;
        }
        Ok(())
    }

    pub(crate) fn copy(&mut self, value: &str, span: Span) -> Result<String, Diagnostics> {
        self.charge(value.len(), span)?;
        Ok(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charges_utf8_bytes_without_overflow_or_hidden_default() {
        let span = Span {
            start: 0,
            end: 2,
            line: 0,
            column: 0,
        };
        let mut exact = InterfaceStrings::new(Some(2));
        assert_eq!(exact.copy("λ", span).unwrap(), "λ");
        assert!(exact.copy("", span).is_ok());
        let error = exact.copy("x", span).unwrap_err();
        assert_eq!(error.items[0].code, "E3024");
        assert_eq!(error.items[0].span, span);
        let mut huge = InterfaceStrings::new(Some(usize::MAX));
        huge.charge(usize::MAX, span).unwrap();
        assert!(huge.charge(1, span).is_err());
        let mut unlimited = InterfaceStrings::new(None);
        unlimited.charge(usize::MAX, span).unwrap();
        unlimited.charge(usize::MAX, span).unwrap();
        assert_eq!(unlimited.copy("λ", span).unwrap(), "λ");
    }
}
