//! Deterministic malformed-input checks. Mutated programs are never executed.

use sovra::compiler::resources::FrontendLimits;
use sovra::compiler::{backend, ir, native_image, parser::Parser, semantic::SemanticAnalyzer};

fn next(state: &mut u64) -> usize {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*state >> 32) as usize
}

#[test]
fn mutated_source_preserves_checked_pipeline_and_diagnostic_ranges() {
    let seeds = [
        "fn main() { print(\"λ\\n\"); }",
        "type Number = Int; fn value(x: Number) -> Number { return x + 1; } fn main() { print(value(2)); }",
        "struct Item { x: Float } fn main() { let x = Item { x: 1 }; print(x.x); }",
        "fn main() { let mut xs = [1, 2]; xs[0] = 3; if (true) { print(xs[0]); } else { print(0); } }",
        "mod local { export fn value() -> Int { return 1; } } fn main() { print(local::value()); }",
        "type A = B; type B = A; fn main() {}",
        "fn main() { if (false) {} else if (true) { print(1); } }",
        "fn main() { missing[0](unknown); }",
    ];
    let alphabet: Vec<_> = "{}()[];,:=+*/<>!&|\"\\ abcdef0123456789\nλ\0"
        .chars()
        .collect();
    let mut state = 0x5356_5253_4f55_5243;
    for case in 0..4096 {
        let mut input: Vec<_> = seeds[case % seeds.len()].chars().collect();
        for _ in 0..1 + next(&mut state) % 8 {
            let at = next(&mut state) % (input.len() + 1);
            let character = alphabet[next(&mut state) % alphabet.len()];
            match next(&mut state) % 3 {
                0 => input.insert(at, character),
                1 if at < input.len() => {
                    input.remove(at);
                }
                _ if at < input.len() => input[at] = character,
                _ => input.push(character),
            }
        }
        let source: String = input.into_iter().collect();
        let parsed = Parser::new().parse_source(&source);
        let generous = Parser::new().parse_source_with_limits(
            &source,
            FrontendLimits {
                max_source_bytes: source.len(),
                max_tokens: source.len(),
                max_diagnostics: source.len().saturating_mul(8) + 1,
            },
        );
        assert_eq!(parsed, generous, "budget compatibility case {case}");
        if let Err(errors) = Parser::new().parse_source_with_limits(
            &source,
            FrontendLimits {
                max_source_bytes: 64,
                max_tokens: 16,
                max_diagnostics: 4,
            },
        ) {
            assert!(!errors.items.is_empty() && errors.items.len() <= 4);
        }
        let result = parsed.and_then(|program| SemanticAnalyzer::new().analyze(&program));
        match result {
            Ok(typed) => {
                let lowered = ir::lower(&typed);
                let image = native_image::encode(&lowered).unwrap();
                assert_eq!(
                    native_image::decode(&image).unwrap(),
                    lowered,
                    "case {case}"
                );
                // Rendering must treat source-derived names/literals as data.
                // Do not execute the resulting JavaScript or any mutated source.
                assert!(!backend::render_javascript(&lowered).is_empty());
            }
            Err(errors) => {
                assert!(
                    !errors.items.is_empty() && errors.items.len() <= 256,
                    "case {case}"
                );
                for error in errors.items {
                    assert!(error.message.len() <= 4096 && error.message.capacity() <= 4096);
                    let span = error.span;
                    assert!(
                        span.start <= span.end && span.end <= source.len(),
                        "case {case}: {error:?}"
                    );
                    assert!(
                        source.is_char_boundary(span.start) && source.is_char_boundary(span.end),
                        "case {case}: {error:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn mutated_native_images_reject_or_round_trip_without_execution() {
    let program = Parser::new().parse_source(
        "struct Point { x: Float } fn main() { let mut xs = [1, 2]; xs[0] = 3; if (true) { print(Point { x: 1 }); } }"
    ).unwrap();
    let lowered = ir::lower_program(&program).unwrap();
    let image = native_image::encode(&lowered).unwrap();
    for length in 0..image.len() {
        assert!(
            native_image::decode(&image[..length]).is_err(),
            "truncation {length}"
        );
    }
    let mut appended = image.clone();
    appended.push(0);
    assert!(native_image::decode(&appended).is_err());
    let mut state = 0x5356_5249_4d41_4745;
    for case in 0..4096 {
        let mut bytes = image.clone();
        for _ in 0..1 + next(&mut state) % 8 {
            let at = next(&mut state) % bytes.len();
            bytes[at] ^= 1 << (next(&mut state) % 8);
        }
        if let Ok(decoded) = native_image::decode(&bytes) {
            let encoded = native_image::encode(&decoded).unwrap();
            assert_eq!(
                native_image::decode(&encoded).unwrap(),
                decoded,
                "case {case}"
            );
        }
    }
}
