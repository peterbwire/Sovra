//! Local package graph resolution and executable entry-module linking.

mod executable;
pub use executable::compile;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::{Manifest, MANIFEST_FILE};
use crate::compiler::diagnostics::{Diagnostic, Diagnostics, Severity, Span};

const MAX_PACKAGES: usize = 1024;

pub(crate) fn declares_dependencies(root: &Path) -> Result<bool, Diagnostics> {
    let source = super::read_manifest(&root.join(MANIFEST_FILE))?;
    Ok(Manifest::parse_mode(&source, true)
        .sections
        .iter()
        .any(|(section, _)| section.starts_with("dependencies.")))
}

/// One local package; its canonical root is its local graph identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPackage {
    /// Canonical package directory.
    pub root: PathBuf,
    /// Manifest display name, not a globally unique identity.
    pub name: String,
    /// Unresolved version metadata; no version selection is performed.
    pub version: Option<String>,
    /// Canonical source entry contained inside this package root.
    pub entry: PathBuf,
    /// Direct dependency aliases mapped to canonical package roots.
    pub dependencies: BTreeMap<String, PathBuf>,
}

/// A fully resolved, acyclic local dependency graph, without source linking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalGraph {
    /// Root package identity.
    pub root: PathBuf,
    /// Packages keyed by canonical root, never by display name.
    pub packages: BTreeMap<PathBuf, LocalPackage>,
}

/// Resolve local dependencies in deterministic alias order without executing code.
/// Traversal is iterative, deduplicates canonical roots and rejects cycles.
/// At most 1024 packages may be loaded. Mutable paths are not content locks.
pub fn resolve(root: impl AsRef<Path>) -> Result<LocalGraph, Diagnostics> {
    let root = canonical_root(root.as_ref())?;
    let mut graph = LocalGraph {
        root: root.clone(),
        packages: BTreeMap::new(),
    };
    let mut pending = vec![(root, false)];
    let mut active = Vec::new();
    let mut done = BTreeSet::new();
    while let Some((root, exiting)) = pending.pop() {
        if exiting {
            active.pop();
            done.insert(root);
            continue;
        }
        if let Some(start) = active.iter().position(|path| path == &root) {
            let chain = active[start..]
                .iter()
                .chain(std::iter::once(&root))
                .map(|path: &PathBuf| path.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(error(
                "E4111",
                format!("local dependency cycle: {chain}"),
                None,
            ));
        }
        if done.contains(&root) {
            continue;
        }
        if graph.packages.len() >= MAX_PACKAGES {
            return Err(error(
                "E4112",
                "local dependency graph exceeds 1024 packages".into(),
                None,
            ));
        }
        let package = load(&root)?;
        active.push(root.clone());
        pending.push((root.clone(), true));
        for target in package.dependencies.values().rev() {
            pending.push((target.clone(), false));
        }
        graph.packages.insert(root, package);
    }
    Ok(graph)
}

fn canonical_root(path: &Path) -> Result<PathBuf, Diagnostics> {
    let canonical = path.canonicalize().map_err(|cause| {
        error(
            "E4110",
            format!("cannot resolve package root `{}`: {cause}", path.display()),
            None,
        )
    })?;
    if !canonical.is_dir() {
        return Err(error(
            "E4110",
            format!("package root `{}` is not a directory", path.display()),
            None,
        ));
    }
    Ok(canonical)
}

fn load(root: &Path) -> Result<LocalPackage, Diagnostics> {
    let manifest_path = root.join(MANIFEST_FILE);
    // A symlinked manifest must not silently change the package's authority root.
    let canonical_manifest = manifest_path.canonicalize().map_err(|cause| {
        error(
            "E4110",
            format!(
                "cannot read package manifest `{}`: {cause}",
                manifest_path.display()
            ),
            None,
        )
    })?;
    if !canonical_manifest.starts_with(root) {
        return Err(error(
            "E4113",
            "package manifest escapes its root".into(),
            None,
        ));
    }
    let source = super::read_manifest(&manifest_path)?;
    let mut manifest = Manifest::parse_mode(&source, true);
    manifest.source_file = Some(manifest_path.to_string_lossy().into_owned());
    super::attach_line_locations(&manifest_path, &source, &mut manifest.diagnostics.items);
    if !manifest.diagnostics.is_empty() {
        return Err(manifest.diagnostics);
    }
    let mut diagnostics = Diagnostics::new();
    let name = super::require_manifest_value(&manifest, "project", "name", &mut diagnostics);
    let entry = super::require_manifest_value(&manifest, "project", "entry", &mut diagnostics);
    if let Some(name) = &name {
        let first = diagnostics.items.len();
        super::validate_project_name(name, &mut diagnostics);
        manifest.attach("project", "name", &mut diagnostics.items[first..]);
    }
    if let Some(target) = manifest.value("runtime", "target") {
        let first = diagnostics.items.len();
        super::validate_runtime_target(target, &mut diagnostics);
        manifest.attach("runtime", "target", &mut diagnostics.items[first..]);
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let first = diagnostics.items.len();
    let entry = super::resolve_project_path(root, entry.as_deref().unwrap(), &mut diagnostics);
    if let Some(entry) = &entry {
        super::validate_entry_path(entry, &mut diagnostics);
    }
    manifest.attach("project", "entry", &mut diagnostics.items[first..]);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let entry = entry.unwrap().canonicalize().map_err(|cause| {
        error(
            "E4110",
            format!("cannot resolve package entry: {cause}"),
            None,
        )
    })?;
    if !entry.starts_with(root) {
        let mut diagnostics = error("E4113", "package entry escapes its root".into(), None);
        manifest.attach("project", "entry", &mut diagnostics.items);
        return Err(diagnostics);
    }
    let mut dependencies = BTreeMap::new();
    for (section, span) in &manifest.sections {
        let Some(alias) = section.strip_prefix("dependencies.") else {
            continue;
        };
        let Some(path) = manifest
            .value(section, "path")
            .filter(|path| !path.is_empty())
        else {
            return Err(error(
                "E4110",
                format!("dependency `{alias}` requires a nonempty path"),
                Some((&manifest_path, *span)),
            ));
        };
        let target = canonical_root(&root.join(path)).map_err(|mut diagnostics| {
            manifest.attach(section, "path", &mut diagnostics.items);
            diagnostics
        })?;
        dependencies.insert(alias.to_owned(), target);
    }
    Ok(LocalPackage {
        root: root.to_owned(),
        name: name.unwrap(),
        entry,
        version: manifest.value("project", "version").map(str::to_owned),
        dependencies,
    })
}

fn error(code: &'static str, message: String, location: Option<(&Path, Span)>) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();
    diagnostics.push(Diagnostic {
        severity: Severity::Error,
        code,
        message,
        span: location.map_or(
            Span {
                start: 0,
                end: 0,
                line: 0,
                column: 0,
            },
            |(_, span)| span,
        ),
        source_file: location.map(|(path, _)| path.to_string_lossy().into_owned()),
    });
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "sovra-packages-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn package(&self, name: &str, dependencies: &str) -> PathBuf {
            let root = self.0.join(name);
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join("main.svr"), "fn main() {}").unwrap();
            std::fs::write(
                root.join(MANIFEST_FILE),
                format!(
                    "[project]\nname = \"same-display-name\"\nentry = \"main.svr\"\n{dependencies}"
                ),
            )
            .unwrap();
            root
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn resolves_diamonds_aliases_and_equal_display_names_deterministically() {
        let fixture = Fixture::new();
        fixture.package("shared", "");
        fixture.package("left", "[dependencies.shared]\npath = \"../shared\"");
        fixture.package("right", "[dependencies.shared]\npath = \"../shared\"");
        let root = fixture.package("app", "[dependencies.right]\npath = \"../right\"\n[dependencies.left]\npath = \"../left\"\n[dependencies.alias]\npath = \"../left\"");
        let graph = resolve(&root).unwrap();
        assert_eq!(graph.packages.len(), 4);
        assert_eq!(graph, resolve(root).unwrap());
        let app = &graph.packages[&graph.root];
        assert_eq!(
            app.dependencies
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["alias", "left", "right"]
        );
        assert_eq!(app.dependencies["alias"], app.dependencies["left"]);
        assert!(!app.dependencies.contains_key("shared"));
    }

    #[test]
    fn rejects_cycles_with_the_chain() {
        let fixture = Fixture::new();
        let root = fixture.package("a", "[dependencies.b]\npath = \"../b\"");
        fixture.package("b", "[dependencies.a]\npath = \"../a\"");
        let errors = resolve(root).unwrap_err();
        assert_eq!(errors.items[0].code, "E4111");
        assert!(errors.items[0].message.contains(" -> "));
    }

    #[test]
    fn rejects_invalid_missing_and_duplicate_dependencies() {
        for dependency in [
            "[dependencies.std]\npath = \".\"",
            "[dependencies.bad-name]\npath = \".\"",
            "[dependencies.a]",
            "[dependencies.a]\npath = \"\"",
            "[dependencies.a]\npath = \"../missing\"",
            "[dependencies.a]\npath = 123",
            "[dependencies.a]\ngit = \"url\"",
            "[dependencies.a]\npath = \".\"\npath = \".\"",
            "[dependencies.a]\npath = \".\"\n[dependencies.a]\npath = \".\"",
        ] {
            let fixture = Fixture::new();
            let root = fixture.package("app", dependency);
            let errors = resolve(root).unwrap_err();
            assert!(
                errors.items.iter().any(|error| error.source_file.is_some()),
                "{dependency}: {errors:?}"
            );
        }
    }

    #[test]
    fn transitive_packages_link_without_exposing_undeclared_dependencies() {
        let fixture = Fixture::new();
        let leaf = fixture.package("leaf", "");
        std::fs::write(
            leaf.join("main.svr"),
            "mod math { export fn value() -> Int { return 20 } }",
        )
        .unwrap();
        let middle = fixture.package("middle", "[dependencies.leaf]\npath = \"../leaf\"");
        std::fs::write(middle.join("main.svr"), "use leaf::math; mod math { export fn value() -> Int { return leaf::math::value() + 1 } }").unwrap();
        let root = fixture.package("app", "[dependencies.middle]\npath = \"../middle\"");
        std::fs::write(
            root.join("main.svr"),
            "use middle::math; fn main() { print(middle::math::value() * 2); }",
        )
        .unwrap();
        assert_eq!(
            crate::compiler::interpreter::run(&compile(&root).unwrap()).unwrap(),
            ["42"]
        );
        std::fs::write(root.join("main.svr"), "use leaf::math; fn main() {}").unwrap();
        assert_eq!(compile(&root).unwrap_err().items[0].code, "E4115");
        std::fs::write(root.join("main.svr"), "fn main() { leaf::math::value(); }").unwrap();
        assert_eq!(compile(&root).unwrap_err().items[0].code, "E3004");
    }

    #[test]
    fn exported_records_support_qualified_construction_fields_and_alias_identity() {
        let fixture = Fixture::new();
        let library = fixture.package("lib", "");
        std::fs::write(library.join("main.svr"), "mod geometry { export struct Point { x: Float, y: Float } export fn origin() -> Point { return Point { x: 0, y: 0 } } export fn x(point: Point) -> Float { return point.x } }").unwrap();
        let root = fixture.package(
            "app",
            "[dependencies.shapes]\npath = \"../lib\"\n[dependencies.other]\npath = \"../lib\"",
        );
        std::fs::write(root.join("main.svr"), "use shapes::geometry; use other::geometry; fn read(point: shapes::geometry::Point) -> Float { return point.x } fn main() { let point = shapes::geometry::Point { x: 3, y: 4 }; print(point.x / 2); print(read(point) / 2); print(other::geometry::x(point)); print(shapes::geometry::origin().y); }").unwrap();
        let linked = compile(&root).unwrap();
        assert_eq!(
            crate::compiler::interpreter::run(&linked).unwrap(),
            ["1.5", "1.5", "3", "0"]
        );
        let output = std::process::Command::new("node")
            .arg("-e")
            .arg(crate::compiler::backend::render_javascript(&linked))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
            "1.5\n1.5\n3\n0\n"
        );
    }

    #[test]
    fn exported_records_from_distinct_packages_are_nominally_distinct() {
        let fixture = Fixture::new();
        for name in ["left", "right"] {
            let root = fixture.package(name, "");
            std::fs::write(root.join("main.svr"), "mod geometry { export struct Point { x: Int } export fn read(point: Point) -> Int { return point.x } }").unwrap();
        }
        let root = fixture.package(
            "app",
            "[dependencies.left]\npath = \"../left\"\n[dependencies.right]\npath = \"../right\"",
        );
        std::fs::write(root.join("main.svr"), "use left::geometry; use right::geometry; fn main() { let a = left::geometry::Point { x: 1 }; let b = right::geometry::Point { x: 2 }; print(left::geometry::read(a)); print(right::geometry::read(b)); }").unwrap();
        assert_eq!(
            crate::compiler::interpreter::run(&compile(&root).unwrap()).unwrap(),
            ["1", "2"]
        );
        std::fs::write(root.join("main.svr"), "use left::geometry; use right::geometry; fn main() { let point = left::geometry::Point { x: 1 }; right::geometry::read(point); }").unwrap();
        let errors = compile(root).unwrap_err();
        assert!(errors.items.iter().any(|error| error.code == "E3007"));
    }

    #[test]
    fn exported_record_constructors_validate_fields_and_import_scope() {
        let fixture = Fixture::new();
        let library = fixture.package("lib", "");
        std::fs::write(
            library.join("main.svr"),
            "mod geometry { export struct Point { x: Float } struct Hidden { x: Int } }",
        )
        .unwrap();
        let root = fixture.package("app", "[dependencies.shapes]\npath = \"../lib\"");
        for source in [
            "use shapes::geometry; fn main() { shapes::geometry::Point {}; }",
            "use shapes::geometry; fn main() { shapes::geometry::Point { x: 1, x: 2 }; }",
            "use shapes::geometry; fn main() { shapes::geometry::Point { x: true }; }",
            "use shapes::geometry; fn main() { shapes::geometry::Point { x: 1, y: 2 }; }",
            "use shapes::geometry; fn main() { let p = shapes::geometry::Point { x: 1 }; print(p.y); }",
            "fn main() { shapes::geometry::Point { x: 1 }; }",
            "use shapes::geometry; fn main() { Point { x: 1 }; }",
            "use shapes::geometry; fn read(p: shapes::geometry::Hidden) {} fn main() {}",
        ] {
            std::fs::write(root.join("main.svr"), source).unwrap();
            let errors = compile(&root).expect_err(source);
            let entry = root.join("main.svr").canonicalize().unwrap();
            assert!(errors.items.iter().all(|error| error.source_file.as_deref() == entry.to_str()));
            assert!(errors.items.iter().all(|error| error.span.end <= source.len()));
        }
    }

    #[test]
    fn nested_exported_records_preserve_transitive_identity_and_scalar_aliases() {
        let fixture = Fixture::new();
        let base = fixture.package("base", "");
        std::fs::write(
            base.join("main.svr"),
            "mod geometry { type Scalar = Float; export struct Point { x: Scalar } }",
        )
        .unwrap();
        let wrapper = fixture.package("wrapper", "[dependencies.base]\npath = \"../base\"");
        std::fs::write(wrapper.join("main.svr"), "use base::geometry; mod boxes { export struct Box { point: base::geometry::Point } export fn make() -> Box { return Box { point: base::geometry::Point { x: 3 } } } }").unwrap();
        let root = fixture.package("app", "[dependencies.wrapper]\npath = \"../wrapper\"\n[dependencies.direct]\npath = \"../base\"");
        std::fs::write(root.join("main.svr"), "use wrapper::boxes; use direct::geometry; struct Point { x: String } fn read(p: direct::geometry::Point) -> Float { return p.x } fn main() { let box = wrapper::boxes::make(); print(box.point.x / 2); print(read(box.point)); }").unwrap();
        let linked = compile(&root).unwrap();
        assert_eq!(
            crate::compiler::interpreter::run(&linked).unwrap(),
            ["1.5", "3"]
        );
        let output = std::process::Command::new("node")
            .arg("-e")
            .arg(crate::compiler::backend::render_javascript(&linked))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
            "1.5\n3\n"
        );
    }

    #[test]
    fn private_record_construction_and_public_field_leaks_are_rejected() {
        let fixture = Fixture::new();
        let library = fixture.package("lib", "");
        let root = fixture.package("app", "[dependencies.shapes]\npath = \"../lib\"");
        std::fs::write(
            library.join("main.svr"),
            "mod geometry { struct Hidden { x: Int } }",
        )
        .unwrap();
        std::fs::write(
            root.join("main.svr"),
            "use shapes::geometry; fn main() { let value = shapes::geometry::Hidden { x: 1 }; }",
        )
        .unwrap();
        assert!(compile(&root)
            .unwrap_err()
            .items
            .iter()
            .any(|error| error.code == "E3004"));
        std::fs::write(root.join("main.svr"), "use shapes::geometry; fn main() {}").unwrap();
        std::fs::write(
            library.join("main.svr"),
            "mod geometry { struct Hidden { x: Int } export struct Public { hidden: Hidden } }",
        )
        .unwrap();
        assert_eq!(compile(root).unwrap_err().items[0].code, "E4116");
    }

    #[test]
    fn consumer_alias_cannot_reinterpret_a_library_signature() {
        let fixture = Fixture::new();
        let library = fixture.package("lib", "");
        std::fs::write(library.join("main.svr"), "type Shared = Float; mod math { export fn identity(value: Shared) -> Shared { return value } }").unwrap();
        let root = fixture.package("app", "[dependencies.util]\npath = \"../lib\"");
        std::fs::write(
            root.join("main.svr"),
            "use util::math; type Shared = Bool; fn main() { util::math::identity(true); }",
        )
        .unwrap();
        let errors = compile(&root).unwrap_err();
        assert!(errors.items.iter().any(|error| error.code == "E3007"));
        std::fs::write(root.join("main.svr"), "use util::math; fn main() { let mut value = util::math::identity(3); value = 5; print(value / 2); }").unwrap();
        let linked = compile(root).unwrap();
        assert_eq!(crate::compiler::interpreter::run(&linked).unwrap(), ["2.5"]);
        let output = std::process::Command::new("node")
            .arg("-e")
            .arg(crate::compiler::backend::render_javascript(&linked))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "2.5");
    }

    #[test]
    fn unrelated_package_records_do_not_alias_by_spelling() {
        let fixture = Fixture::new();
        let library = fixture.package("lib", "");
        std::fs::write(library.join("main.svr"), "struct Token { value: Int } mod api { export fn read(token: Token) -> Int { return token.value } }").unwrap();
        let root = fixture.package("app", "[dependencies.util]\npath = \"../lib\"");
        std::fs::write(root.join("main.svr"), "use util::api; struct Token { value: Int } fn main() { print(util::api::read(Token { value: 1 })); }").unwrap();
        let errors = compile(root).unwrap_err();
        assert_eq!(errors.items[0].code, "E4116");
        assert_eq!(
            errors.items[0].source_file.as_deref(),
            Some(
                library
                    .join("main.svr")
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .as_ref()
            )
        );
    }

    #[test]
    fn imported_float_results_preserve_inferred_widening() {
        let fixture = Fixture::new();
        let library = fixture.package("lib", "");
        std::fs::write(
            library.join("main.svr"),
            "mod math { export fn value() -> Float { return 1.5 } }",
        )
        .unwrap();
        let root = fixture.package("app", "[dependencies.util]\npath = \"../lib\"");
        std::fs::write(root.join("main.svr"), "use util::math; fn main() { let mut value = util::math::value(); value = 3; print(value / 2); let values = [util::math::value(), 3]; print(values[1] / 2); }").unwrap();
        let linked = compile(&root).unwrap();
        assert_eq!(
            crate::compiler::interpreter::run(&linked).unwrap(),
            ["1.5", "1.5"]
        );
        let output = std::process::Command::new("node")
            .arg("-e")
            .arg(crate::compiler::backend::render_javascript(&linked))
            .output()
            .expect("Node required for backend verification");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
            "1.5\n1.5\n"
        );
    }

    #[test]
    fn linked_ir_is_relocatable_and_numeric_widening_crosses_packages() {
        let mut results = Vec::new();
        for _ in 0..2 {
            let fixture = Fixture::new();
            let library = fixture.package("lib", "");
            std::fs::write(
                library.join("main.svr"),
                "mod math { export fn half(value: Float) -> Float { return value / 2 } }",
            )
            .unwrap();
            let root = fixture.package("app", "[dependencies.util]\npath = \"../lib\"");
            std::fs::write(
                root.join("main.svr"),
                "use util::math; fn main() { print(util::math::half(3)); }",
            )
            .unwrap();
            results.push(compile(&root).unwrap());
        }
        assert_eq!(results[0], results[1]);
        assert_eq!(
            crate::compiler::interpreter::run(&results[0]).unwrap(),
            ["1.5"]
        );
    }

    #[test]
    fn imported_libraries_preserve_duplicate_and_builtin_collision_errors() {
        let fixture = Fixture::new();
        let library = fixture.package("lib", "");
        let root = fixture.package("app", "[dependencies.util]\npath = \"../lib\"");
        std::fs::write(root.join("main.svr"), "fn main() {}").unwrap();
        for (source, code) in [
            ("mod math {} mod math {}", "E3008"),
            ("mod math { fn f() {} export fn f() {} }", "E3008"),
            ("fn print() {}", "E3016"),
            (
                "mod std { export fn len(value: String) -> Int { return 0 } }",
                "E3016",
            ),
            ("mod math { export fn bad(value: Unknown) {} }", "E3017"),
        ] {
            std::fs::write(library.join("main.svr"), source).unwrap();
            assert!(
                compile(&root)
                    .unwrap_err()
                    .items
                    .iter()
                    .any(|error| error.code == code),
                "{source}"
            );
        }
    }

    #[test]
    fn compiles_isolated_libraries_with_private_helpers_and_aliases() {
        let fixture = Fixture::new();
        let library = fixture.package("lib", "");
        std::fs::write(library.join("main.svr"), "fn helper(value: Int) -> Int { return value + 1 } fn main() { print(999) } mod math { fn private(value: Int) -> Int { return helper(value) } export fn answer(value: Int) -> Int { return math::private(value) } }").unwrap();
        let root = fixture.package(
            "app",
            "[dependencies.util]\npath = \"../lib\"\n[dependencies.second]\npath = \"../lib\"",
        );
        let source = "use util::math; use second::math; use util::math; fn helper(value: Int) -> Int { return 999 } fn main() { print(util::math::answer(41)); print(second::math::answer(1)); }";
        std::fs::write(root.join("main.svr"), source).unwrap();
        let linked = compile(&root).unwrap();
        assert_eq!(
            crate::compiler::interpreter::run(&linked).unwrap(),
            ["42", "2"]
        );
        assert_eq!(linked, compile(&root).unwrap());
        for (call, code) in [
            ("util::math::private(1)", "E3004"),
            ("util::math::answer()", "E3006"),
            ("util::math::answer(true)", "E3007"),
            ("other::math::answer(1)", "E3004"),
        ] {
            let source = format!("// λ\r\nuse util::math; fn main() {{ {call}; }}");
            std::fs::write(root.join("main.svr"), &source).unwrap();
            let errors = compile(&root).unwrap_err();
            assert!(
                errors.items.iter().any(|error| error.code == code),
                "{errors:?}"
            );
            assert!(errors.items.iter().all(|error| error.source_file.as_deref()
                == Some(
                    root.join("main.svr")
                        .canonicalize()
                        .unwrap()
                        .to_string_lossy()
                        .as_ref()
                )));
        }
    }

    #[test]
    fn imported_modules_require_explicit_direct_dependencies() {
        let fixture = Fixture::new();
        let library = fixture.package("lib", "");
        std::fs::write(
            library.join("main.svr"),
            "mod math { export fn answer() -> Int { return 42 } }",
        )
        .unwrap();
        let root = fixture.package("app", "[dependencies.util]\npath = \"../lib\"");
        for (source, code) in [
            ("use util.math; fn main() {}", "E4114"),
            ("use missing::math; fn main() {}", "E4115"),
            ("use util::absent; fn main() {}", "E4115"),
            ("fn main() { util::math::answer(); }", "E3004"),
        ] {
            std::fs::write(root.join("main.svr"), source).unwrap();
            assert_eq!(compile(&root).unwrap_err().items[0].code, code);
        }
        std::fs::write(root.join("main.svr"), "use util::math; fn main() {}").unwrap();
        std::fs::write(
            library.join("main.svr"),
            "mod math { fn invalid() { missing(); } }",
        )
        .unwrap();
        let errors = compile(root).unwrap_err();
        assert_eq!(errors.items[0].code, "E3004");
        assert_eq!(
            errors.items[0].source_file.as_deref(),
            Some(
                library
                    .join("main.svr")
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .as_ref()
            )
        );
    }

    #[test]
    fn graph_resolution_does_not_claim_source_validation_or_cli_support() {
        let fixture = Fixture::new();
        fixture.package("lib", "");
        let root = fixture.package("app", "[dependencies.lib]\npath = \"../lib\"");
        std::fs::write(root.join("main.svr"), "not executable Sovra").unwrap();
        assert!(resolve(&root).is_ok());
        let errors = super::super::check_project(root).unwrap_err();
        assert!(errors.items.iter().any(|error| error.code == "E4010"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_entry_and_manifest_escapes() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        let outside = fixture.package("outside", "");
        let root = fixture.package("app", "");
        std::fs::remove_file(root.join("main.svr")).unwrap();
        symlink(outside.join("main.svr"), root.join("main.svr")).unwrap();
        assert_eq!(resolve(&root).unwrap_err().items[0].code, "E4113");
        std::fs::remove_file(root.join(MANIFEST_FILE)).unwrap();
        symlink(outside.join(MANIFEST_FILE), root.join(MANIFEST_FILE)).unwrap();
        assert_eq!(resolve(root).unwrap_err().items[0].code, "E4113");
    }

    #[test]
    fn rejects_missing_manifests_and_escaping_entries() {
        let fixture = Fixture::new();
        assert!(resolve(&fixture.0).is_err());
        let root = fixture.package("app", "");
        std::fs::write(
            root.join(MANIFEST_FILE),
            "[project]\nname = \"app\"\nentry = \"../outside.svr\"",
        )
        .unwrap();
        assert_eq!(resolve(root).unwrap_err().items[0].code, "E4007");
    }
}
