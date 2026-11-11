//! Frontend budget boundaries without changing the default compiler contract.

use sovra::compiler::{lexer::Lexer, parser::Parser, resources::FrontendLimits};

fn limits(bytes: usize, tokens: usize, diagnostics: usize) -> FrontendLimits {
    FrontendLimits {
        max_source_bytes: bytes,
        max_tokens: tokens,
        max_diagnostics: diagnostics,
    }
}

#[test]
fn source_byte_limit_counts_utf8_and_checks_before_tokenization() {
    let source = "fn main() { print(\"λ\"); }";
    assert!(Parser::new()
        .parse_source_with_limits(source, limits(source.len(), 100, 10))
        .is_ok());
    let errors = Parser::new()
        .parse_source_with_limits(source, limits(source.len() - 1, 100, 10))
        .unwrap_err();
    assert_eq!(errors.items.len(), 1);
    assert_eq!(errors.items[0].code, "E1003");
    assert!(errors.items[0].message.contains("UTF-8 bytes"));
    assert_eq!(errors.items[0].span.start, 0);
    assert!(
        Parser::new().parse_source(source).is_ok(),
        "default acceptance must not change"
    );
}

#[test]
fn token_budget_excludes_eof_whitespace_and_comments() {
    let source = "fn main() {} // comment\n";
    let count = Lexer::new().tokenize(source).unwrap().len() - 1;
    assert!(Parser::new()
        .parse_source_with_limits(source, limits(100, count, 10))
        .is_ok());
    let errors = Parser::new()
        .parse_source_with_limits(source, limits(100, count - 1, 10))
        .unwrap_err();
    assert_eq!(errors.items[0].code, "E1003");
    assert!(errors.items[0].message.contains("token limit"));
    let span = errors.items[0].span;
    assert!(source.is_char_boundary(span.start) && span.end <= source.len());
    assert!(Parser::new()
        .parse_source_with_limits(" // nothing\n", limits(100, 0, 1))
        .is_ok());
}

#[test]
fn lexical_diagnostic_budget_stops_error_floods() {
    for cap in [0, 1, 2, 8] {
        let source = "@".repeat(1000);
        let errors = Lexer::new()
            .tokenize_with_limits(&source, limits(1000, 1000, cap))
            .unwrap_err();
        assert_eq!(errors.items.len(), cap.max(1));
        assert_eq!(errors.items.last().unwrap().code, "E1003");
        assert!(errors.items.last().unwrap().span.start < source.len());
    }
}

#[test]
fn parser_diagnostic_budget_stops_recovery_without_partial_success() {
    let source = "fn bad( { fn next( { fn last( {";
    for cap in [0, 1, 2] {
        let errors = Parser::new()
            .parse_source_with_limits(source, limits(1000, 1000, cap))
            .unwrap_err();
        assert_eq!(errors.items.len(), cap.max(1));
        assert_eq!(errors.items.last().unwrap().code, "E2008");
    }
    let plain = Parser::new().parse_source(source).unwrap_err();
    let generous = Parser::new()
        .parse_source_with_limits(source, limits(1000, 1000, 100))
        .unwrap_err();
    assert_eq!(
        plain, generous,
        "generous budgets retain ordinary diagnostics"
    );
}

#[test]
fn zero_budgets_never_make_invalid_input_successful() {
    assert!(Parser::new()
        .parse_source_with_limits("", limits(0, 0, 0))
        .is_ok());
    assert!(Parser::new()
        .parse_source_with_limits("fn main() {}", limits(0, 0, 0))
        .is_err());
    assert!(Parser::new()
        .parse_source_with_limits("@", limits(1, 0, 0))
        .is_err());
}

#[test]
fn approved_defaults_bound_diagnostics_and_source() {
    assert_eq!(FrontendLimits::DEFAULT.max_source_bytes, 16 * 1024 * 1024);
    assert_eq!(FrontendLimits::DEFAULT.max_tokens, 1_000_000);
    assert_eq!(FrontendLimits::DEFAULT.max_diagnostics, 256);
    let errors = Lexer::new().tokenize(&"@".repeat(1000)).unwrap_err();
    assert_eq!(errors.items.len(), 256);
    assert_eq!(errors.items.last().unwrap().code, "E1003");
    let errors = Parser::new()
        .parse_source(&"fn broken() { let x = ; } ".repeat(1000))
        .unwrap_err();
    assert_eq!(errors.items.len(), 256);
    assert_eq!(errors.items.last().unwrap().code, "E2008");
    let errors = Lexer::new()
        .tokenize(&" ".repeat(16 * 1024 * 1024 + 1))
        .unwrap_err();
    assert_eq!(errors.items.len(), 1);
    assert_eq!(errors.items[0].code, "E1003");
}

#[test]
fn approved_default_token_boundary_excludes_eof() {
    let mut source = ";".repeat(FrontendLimits::DEFAULT.max_tokens);
    let tokens = Lexer::new().tokenize(&source).unwrap();
    assert_eq!(tokens.len(), FrontendLimits::DEFAULT.max_tokens + 1);
    drop(tokens);
    source.push(';');
    match Lexer::new().tokenize(&source) {
        Err(errors) => assert_eq!(errors.items[0].code, "E1003"),
        Ok(_) => panic!("default token overflow must fail"),
    }
}
