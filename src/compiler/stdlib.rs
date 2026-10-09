//! Standard-library function registry for the M9 foundation.

/// A standard-library function exposed to Sovra programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StdFunction {
    /// Fully qualified function name.
    pub name: &'static str,
    /// Parameter type names, in call order.
    pub parameters: &'static [&'static str],
    /// Return type name.
    pub return_type: &'static str,
}

const PRINT_PARAMETERS: &[&str] = &["Any"];
const LEN_PARAMETERS: &[&str] = &["String"];
const TO_STRING_PARAMETERS: &[&str] = &["Any"];
const NO_PARAMETERS: &[&str] = &[];
const INDEX_PARAMETERS: &[&str] = &["Int"];
const EXIT_CODE_PARAMETERS: &[&str] = &["Int"];
const TEXT_PATH_PARAMETERS: &[&str] = &["String"];
const TEXT_WRITE_PARAMETERS: &[&str] = &["String", "String"];

/// Maximum byte length of one line returned from standard input.
pub const MAX_INPUT_LINE_BYTES: usize = 1024 * 1024;

/// Public nominal identity of the standard input line record.
pub const INPUT_LINE_TYPE: &str = "std::InputLine";
/// Maximum UTF-8 byte length of a text-file read or write.
pub const MAX_TEXT_FILE_BYTES: usize = 16 * 1024 * 1024;
/// Public nominal result type for text-file reads.
pub const TEXT_READ_TYPE: &str = "std::TextRead";
/// Public nominal result type for text-file writes.
pub const TEXT_WRITE_TYPE: &str = "std::TextWrite";
/// Result of splitting a string at its first delimiter.
pub const SPLIT_ONCE_TYPE: &str = "std::SplitOnce";
/// Result of strict signed decimal parsing.
pub const PARSED_INT_TYPE: &str = "std::ParsedInt";

/// Whether a named record belongs to the standard library's public API.
pub fn is_standard_record_type(name: &str) -> bool {
    matches!(
        name,
        INPUT_LINE_TYPE | TEXT_READ_TYPE | TEXT_WRITE_TYPE | SPLIT_ONCE_TYPE | PARSED_INT_TYPE
    )
}

const FUNCTIONS: &[StdFunction] = &[
    StdFunction {
        name: "std::print",
        parameters: PRINT_PARAMETERS,
        return_type: "Unit",
    },
    StdFunction {
        name: "std::println",
        parameters: PRINT_PARAMETERS,
        return_type: "Unit",
    },
    StdFunction {
        name: "std::len",
        parameters: LEN_PARAMETERS,
        return_type: "Int",
    },
    StdFunction {
        name: "std::to_string",
        parameters: TO_STRING_PARAMETERS,
        return_type: "String",
    },
    StdFunction {
        name: "std::arg_count",
        parameters: NO_PARAMETERS,
        return_type: "Int",
    },
    StdFunction {
        name: "std::arg",
        parameters: INDEX_PARAMETERS,
        return_type: "String",
    },
    StdFunction {
        name: "std::set_exit_code",
        parameters: EXIT_CODE_PARAMETERS,
        return_type: "Unit",
    },
    StdFunction {
        name: "std::read_line",
        parameters: NO_PARAMETERS,
        return_type: INPUT_LINE_TYPE,
    },
    StdFunction {
        name: "std::read_text",
        parameters: TEXT_PATH_PARAMETERS,
        return_type: TEXT_READ_TYPE,
    },
    StdFunction {
        name: "std::write_text",
        parameters: TEXT_WRITE_PARAMETERS,
        return_type: TEXT_WRITE_TYPE,
    },
    StdFunction {
        name: "std::split_once",
        parameters: TEXT_WRITE_PARAMETERS,
        return_type: SPLIT_ONCE_TYPE,
    },
    StdFunction {
        name: "std::lines_unique",
        parameters: LEN_PARAMETERS,
        return_type: "Bool",
    },
    StdFunction {
        name: "std::parse_int",
        parameters: LEN_PARAMETERS,
        return_type: PARSED_INT_TYPE,
    },
];

/// Return all stable M9 standard-library functions.
pub const fn functions() -> &'static [StdFunction] {
    FUNCTIONS
}

/// Look up a standard-library function by name.
///
/// Bare `print` remains available as a compatibility alias for earlier
/// examples, but new code should prefer `std::print` or `std::println`.
pub fn lookup(name: &str) -> Option<StdFunction> {
    let canonical = if name == "print" { "std::print" } else { name };
    FUNCTIONS
        .iter()
        .copied()
        .find(|function| function.name == canonical)
}

/// Return whether the exact LF-delimited lines in `text` are pairwise distinct.
///
/// A terminal LF ends the last line without adding an extra empty line.
pub fn lines_unique(text: &str) -> bool {
    let mut seen = std::collections::HashSet::new();
    text.split_terminator('\n').all(|line| seen.insert(line))
}

/// Parse the exact decimal grammar of `std::parse_int`, rejecting overflow.
pub fn parse_int(text: &str) -> Option<i64> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty()
        || digits.len() > 19
        || (digits.len() > 1 && digits.starts_with('0'))
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    text.parse::<i64>().ok()
}

/// Whether a parameter type accepts any Sovra value.
pub const fn is_any_type(type_name: &str) -> bool {
    matches!(type_name.as_bytes(), b"Any")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_stable_std_namespace() {
        let names: Vec<_> = functions().iter().map(|function| function.name).collect();
        assert_eq!(
            names,
            vec![
                "std::print",
                "std::println",
                "std::len",
                "std::to_string",
                "std::arg_count",
                "std::arg",
                "std::set_exit_code",
                "std::read_line",
                "std::read_text",
                "std::write_text",
                "std::split_once",
                "std::lines_unique",
                "std::parse_int"
            ]
        );
    }

    #[test]
    fn line_uniqueness_preserves_empty_and_exact_unicode_lines() {
        for (text, unique) in [
            ("", true),
            ("one", true),
            ("one\n", true),
            ("one\none", false),
            ("\n", true),
            ("\n\n", false),
            ("one\n\n", true),
            ("a\r\na\n", true),
            ("猫\n猫\n", false),
            ("猫\n犬\n", true),
        ] {
            assert_eq!(lines_unique(text), unique, "{text:?}");
        }
    }

    #[test]
    fn keeps_print_alias_for_compatibility() {
        assert_eq!(lookup("print"), lookup("std::print"));
    }

    #[test]
    fn strict_decimal_parsing() {
        for (text, expected) in [
            ("0", Some(0)),
            ("-0", Some(0)),
            ("9223372036854775807", Some(i64::MAX)),
            ("-9223372036854775808", Some(i64::MIN)),
            ("9223372036854775808", None),
            ("-9223372036854775809", None),
            ("", None),
            ("+1", None),
            ("01", None),
            ("-01", None),
            (" 1", None),
            ("1\n", None),
            ("١", None),
        ] {
            assert_eq!(parse_int(text), expected, "{text:?}");
        }
        assert_eq!(parse_int(&"9".repeat(1024 * 1024)), None);
    }
}
