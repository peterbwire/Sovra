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
            vec!["std::print", "std::println", "std::len", "std::to_string"]
        );
    }

    #[test]
    fn keeps_print_alias_for_compatibility() {
        assert_eq!(lookup("print"), lookup("std::print"));
    }
}
