//! Stream human diagnostic arguments without terminal control scalars.

use std::fmt;

/// Printable C0/DEL/C1 escaping for human CLI errors. This preserves ordinary
/// Unicode and literal backslashes, without allocating a whole escaped message.
pub(crate) fn terminal_message(arguments: fmt::Arguments<'_>) -> impl fmt::Display + '_ {
    struct Message<'a>(fmt::Arguments<'a>);
    struct Escaped<'a, 'b> {
        output: &'a mut fmt::Formatter<'b>,
    }
    impl fmt::Write for Escaped<'_, '_> {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            let mut start = 0;
            for (index, character) in text.char_indices() {
                let value = character as u32;
                if value <= 0x1f || (0x7f..=0x9f).contains(&value) {
                    self.output.write_str(&text[start..index])?;
                    write!(self.output, "\\u{{{value:04X}}}")?;
                    start = index + character.len_utf8();
                }
            }
            self.output.write_str(&text[start..])
        }
    }
    impl fmt::Display for Message<'_> {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt::write(&mut Escaped { output: formatter }, self.0)
        }
    }
    Message(arguments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_terminal_policy_matches_existing_scalar_escaping() {
        let controls: String = (0..=0x1f)
            .chain(0x7f..=0x9f)
            .filter_map(char::from_u32)
            .collect();
        for value in [controls, "λ\\tail\u{00a0}\u{2028}".into(), String::new()] {
            let actual = format!(
                "{}",
                terminal_message(format_args!("prefix {value}: {value:?} suffix"))
            );
            let expected = crate::compiler::stdlib::escape_terminal(&format!(
                "prefix {value}: {value:?} suffix"
            ));
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn streaming_terminal_adapter_propagates_output_exhaustion() {
        use std::cell::Cell;
        struct Repeated<'a>(&'a Cell<usize>);
        impl fmt::Display for Repeated<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for _ in 0..100_000 {
                    self.0.set(self.0.get() + 1);
                    formatter.write_str("\u{001b}")?;
                }
                Ok(())
            }
        }
        let calls = Cell::new(0);
        let result = crate::compiler::diagnostics::format_message(format_args!(
            "{}",
            terminal_message(format_args!("{}", Repeated(&calls)))
        ));
        assert!(calls.get() < 10_000);
        assert!(result.ends_with("[message truncated]"));
        assert!(!result.contains('\u{001b}'));
        assert!(result.capacity() <= 4096);
    }
}
