#![no_main]

use libfuzzer_sys::fuzz_target;
use sovra::compiler::resources::{FrontendLimits, LoweringLimits};
use sovra::compiler::{backend, ir, native_image, parser::Parser, semantic::SemanticAnalyzer};

fuzz_target!(|data: &[u8]| {
    // Harness budget only; this does not change accepted language inputs.
    if data.len() > 16 * 1024 {
        return;
    }
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let bounded = Parser::new().parse_source_with_limits(
        source,
        FrontendLimits {
            max_source_bytes: 4096,
            max_tokens: 512,
            max_diagnostics: 8,
        },
    );
    if let Err(errors) = bounded {
        assert!(!errors.items.is_empty() && errors.items.len() <= 8);
        for error in errors.items {
            assert!(error.span.start <= error.span.end && error.span.end <= source.len());
            assert!(
                source.is_char_boundary(error.span.start)
                    && source.is_char_boundary(error.span.end)
            );
        }
    }
    if let Ok(parsed) = Parser::new().parse_source(source) {
        match ir::lower_program_with_limits(
            &parsed,
            256,
            LoweringLimits {
                max_instructions: 512,
                max_string_bytes: 4096,
            },
        ) {
            Ok(program) => {
                assert!(
                    program
                        .functions
                        .iter()
                        .map(|function| function.instructions.len())
                        .sum::<usize>()
                        <= 512
                );
            }
            Err(errors) => {
                assert!(!errors.items.is_empty() && errors.items.len() <= 256);
                for error in errors.items {
                    assert!(error.span.start <= error.span.end && error.span.end <= source.len());
                    assert!(
                        source.is_char_boundary(error.span.start)
                            && source.is_char_boundary(error.span.end)
                    );
                    assert!(error.message.len() <= 4096 && error.message.capacity() <= 4096);
                }
            }
        }
    }
    match Parser::new()
        .parse_source(source)
        .and_then(|program| SemanticAnalyzer::new().analyze(&program))
    {
        Ok(typed) => {
            let program = ir::lower(&typed);
            let image = native_image::encode(&program).expect("small checked program must encode");
            assert_eq!(native_image::decode(&image).unwrap(), program);
            let _ = backend::render_javascript(&program);
        }
        Err(errors) => {
            assert!(!errors.items.is_empty() && errors.items.len() <= 256);
            for error in errors.items {
                assert!(error.message.len() <= 4096 && error.message.capacity() <= 4096);
                let span = error.span;
                assert!(span.start <= span.end && span.end <= source.len());
                assert!(source.is_char_boundary(span.start) && source.is_char_boundary(span.end));
            }
        }
    }
});
