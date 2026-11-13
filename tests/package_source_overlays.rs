//! Unsaved graph buffers are borrowed, bounded and never written to source files.
use sovra::compiler::{
    interpreter,
    project::packages::{self, SourceOverlay, SourceOverlayLimits},
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "svr-overlays-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::create_dir_all(root.join("lib")).unwrap();
        std::fs::write(root.join("app/sovra.toml"), "[project]\nname = \"app\"\nentry = \"main.svr\"\n[dependencies.a]\npath = \"../lib\"\n[dependencies.b]\npath = \"../lib\"\n").unwrap();
        std::fs::write(
            root.join("lib/sovra.toml"),
            "[project]\nname = \"lib\"\nentry = \"main.svr\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("lib/main.svr"),
            "mod api { export fn value(x: Int) -> Int { return x; } } fn main() {}",
        )
        .unwrap();
        std::fs::write(
            root.join("app/main.svr"),
            "use a::api; use b::api; fn main() { print(a::api::value(2)); }",
        )
        .unwrap();
        Self(root)
    }
    fn overlay(&self, name: &str, source: &str) -> SourceOverlay {
        SourceOverlay {
            entry: self.0.join(name),
            source: source.to_owned(),
        }
    }
    fn compile(
        &self,
        sources: &[SourceOverlay],
        limits: SourceOverlayLimits,
    ) -> Result<packages::CheckedPackage, packages::CheckedPackageError> {
        packages::compile_with_source_overlays(self.0.join("app"), sources, limits)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn simultaneous_unsaved_provider_and_consumer_use_fresh_canonical_graph_without_writes() {
    let fixture = Fixture::new();
    let disk = std::fs::read(fixture.0.join("app/main.svr")).unwrap();
    let provider_disk = std::fs::read(fixture.0.join("lib/main.svr")).unwrap();
    let buffers = [
        fixture.overlay(
            "lib/main.svr",
            "mod api { export fn value(x: Float) -> Float { return x; } } fn main() {}",
        ),
        fixture.overlay(
            "app/main.svr",
            "use a::api; use b::api; fn main() { print(b::api::value(3) / 2); }",
        ),
    ];
    let checked = fixture
        .compile(&buffers, SourceOverlayLimits::DEFAULT)
        .unwrap();
    assert_eq!(
        checked.units, 2,
        "two aliases must not duplicate provider compilation"
    );
    assert_eq!(interpreter::run(&checked.ir).unwrap(), ["1.5"]);
    assert_eq!(std::fs::read(fixture.0.join("app/main.svr")).unwrap(), disk);
    assert_eq!(
        std::fs::read(fixture.0.join("lib/main.svr")).unwrap(),
        provider_disk
    );
    let ordinary = packages::compile(fixture.0.join("app")).unwrap();
    assert_eq!(
        fixture
            .compile(&[], SourceOverlayLimits::DEFAULT)
            .unwrap()
            .ir,
        ordinary
    );
    assert_eq!(interpreter::run(&ordinary).unwrap(), ["2"]);
    let again = fixture
        .compile(&buffers, SourceOverlayLimits::DEFAULT)
        .unwrap();
    assert_eq!(again.ir, checked.ir);
    assert_eq!(again.remaining, checked.remaining);
}

#[test]
fn overlay_diagnostics_use_unsaved_spans_and_do_not_reuse_a_successful_snapshot() {
    let fixture = Fixture::new();
    let valid = fixture.overlay("app/main.svr", "fn main() { print(7); }");
    assert!(fixture
        .compile(&[valid], SourceOverlayLimits::DEFAULT)
        .is_ok());
    let source = "// λ😀\r\nfn main() { missing(); }";
    let invalid = fixture.overlay("app/main.svr", source);
    let errors = fixture
        .compile(&[invalid], SourceOverlayLimits::DEFAULT)
        .unwrap_err()
        .into_diagnostics();
    let error = errors.items.iter().find(|d| d.code == "E3004").unwrap();
    assert_eq!(
        error.source_file.as_deref(),
        fixture
            .0
            .join("app/main.svr")
            .canonicalize()
            .unwrap()
            .to_str()
    );
    assert_eq!(&source[error.span.start..error.span.end], "missing");
    assert_eq!(
        interpreter::run(&packages::compile(fixture.0.join("app")).unwrap()).unwrap(),
        ["2"]
    );
}

#[test]
fn exact_overlay_allowances_and_each_independent_overflow_are_fail_closed() {
    let fixture = Fixture::new();
    let buffers = [
        fixture.overlay("app/main.svr", "// λ\nfn main() {}"),
        fixture.overlay("lib/main.svr", "fn main() {}"),
    ];
    let bytes = buffers.iter().map(|s| s.source.len()).sum();
    let exact = SourceOverlayLimits {
        max_files: 2,
        max_source_bytes: buffers.iter().map(|s| s.source.len()).max().unwrap(),
        max_total_source_bytes: bytes,
    };
    let expected = fixture.compile(&buffers, exact).unwrap();
    for limited in [
        SourceOverlayLimits {
            max_files: 1,
            ..exact
        },
        SourceOverlayLimits {
            max_source_bytes: exact.max_source_bytes - 1,
            ..exact
        },
        SourceOverlayLimits {
            max_total_source_bytes: bytes - 1,
            ..exact
        },
    ] {
        let diagnostics = fixture
            .compile(&buffers, limited)
            .unwrap_err()
            .into_diagnostics();
        assert_eq!(diagnostics.items[0].code, "E4118");
        assert!(diagnostics.items[0].source_file.is_none());
        assert_eq!(fixture.compile(&buffers, exact).unwrap().ir, expected.ir);
    }
}

#[test]
fn duplicate_canonical_entries_missing_and_unrelated_targets_are_rejected() {
    let fixture = Fixture::new();
    let valid = fixture.overlay("app/main.svr", "fn main() {}");
    let mut alias = valid.clone();
    alias.entry = fixture.0.join("app/../app/main.svr");
    assert_eq!(
        fixture
            .compile(&[valid, alias], SourceOverlayLimits::DEFAULT)
            .unwrap_err()
            .into_diagnostics()
            .items[0]
            .code,
        "E4117"
    );
    std::fs::write(fixture.0.join("outside.svr"), "fn main() {}").unwrap();
    for name in ["outside.svr", "missing.svr", "app/sovra.toml"] {
        assert_eq!(
            fixture
                .compile(
                    &[fixture.overlay(name, "fn main() {}")],
                    SourceOverlayLimits::DEFAULT
                )
                .unwrap_err()
                .into_diagnostics()
                .items[0]
                .code,
            "E4117"
        );
    }
}

#[test]
fn navigation_uses_source_owned_declarations_and_success_only_metadata() {
    let fixture = Fixture::new();
    let buffers = [fixture.overlay(
        "app/main.svr",
        "use a::api; use b::api; fn main() { a::api::value(1); b::api::value(2); }",
    )];
    let result = packages::compile_with_source_navigation(
        fixture.0.join("app"),
        &buffers,
        SourceOverlayLimits::DEFAULT,
    )
    .unwrap();
    let ordinary = fixture
        .compile(&buffers, SourceOverlayLimits::DEFAULT)
        .unwrap();
    assert_eq!(result.compiled.ir, ordinary.ir);
    let json = result.navigation.as_json();
    assert!(json.contains("\"unavailable\":false"));
    assert!(json.contains("api::value"));
    assert!(json.contains("fn value(x: Int) -> Int"));
    assert!(json.contains("\"sha256\":"));
    let bad = [fixture.overlay("app/main.svr", "fn main() { missing(); }")];
    assert!(packages::compile_with_source_navigation(
        fixture.0.join("app"),
        &bad,
        SourceOverlayLimits::DEFAULT
    )
    .is_err());
}

#[test]
fn navigation_source_admission_is_optional_and_publishes_no_partial_tables() {
    let fixture = Fixture::new();
    let source = format!(
        "//{}\nmod api {{ export fn value(x: Int) -> Int {{ return x; }} }} fn main() {{}}",
        "x".repeat(512 * 1024)
    );
    std::fs::write(fixture.0.join("lib/main.svr"), source).unwrap();
    let result = packages::compile_with_source_navigation(
        fixture.0.join("app"),
        &[],
        SourceOverlayLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        result.navigation.as_json(),
        "{\"schema_version\":1,\"unavailable\":true}"
    );
    assert_eq!(
        result.compiled.ir,
        packages::compile(fixture.0.join("app")).unwrap()
    );
}

#[test]
fn navigation_file_count_exact_and_overflow_leave_compilation_unchanged() {
    let fixture = Fixture::new();
    let manifest = fixture.0.join("app/sovra.toml");
    let mut text = std::fs::read_to_string(&manifest).unwrap();
    for index in 0..31 {
        let name = format!("p{index}");
        std::fs::create_dir(fixture.0.join(&name)).unwrap();
        std::fs::write(
            fixture.0.join(&name).join("sovra.toml"),
            format!("[project]\nname = \"{name}\"\nentry = \"main.svr\"\n"),
        )
        .unwrap();
        std::fs::write(fixture.0.join(&name).join("main.svr"), "fn main() {}").unwrap();
        text.push_str(&format!("[dependencies.{name}]\npath = \"../{name}\"\n"));
        std::fs::write(&manifest, &text).unwrap();
        if index >= 29 {
            let result = packages::compile_with_source_navigation(
                fixture.0.join("app"),
                &[],
                SourceOverlayLimits::DEFAULT,
            )
            .unwrap();
            assert_eq!(result.compiled.units, index + 3);
            if index == 29 {
                assert!(result
                    .navigation
                    .as_json()
                    .contains("\"unavailable\":false"));
            } else {
                assert_eq!(
                    result.navigation.as_json(),
                    "{\"schema_version\":1,\"unavailable\":true}"
                );
            }
            assert_eq!(
                result.compiled.ir,
                packages::compile(fixture.0.join("app")).unwrap()
            );
        }
    }
}

#[test]
fn navigation_serialized_byte_limit_is_independent_of_successful_compilation() {
    let fixture = Fixture::new();
    let mut source = std::fs::read_to_string(fixture.0.join("lib/main.svr")).unwrap();
    for index in 0..5000 {
        source.push_str(&format!("fn f{index}() {{}}"));
    }
    std::fs::write(fixture.0.join("lib/main.svr"), source).unwrap();
    let result = packages::compile_with_source_navigation(
        fixture.0.join("app"),
        &[],
        SourceOverlayLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        result.navigation.as_json(),
        "{\"schema_version\":1,\"unavailable\":true}"
    );
    assert_eq!(
        result.compiled.ir,
        packages::compile(fixture.0.join("app")).unwrap()
    );
}
