//! Independent consumer execution and source-owned registered callable failures.
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux {
    use sovra::compiler::{backend, interpreter, project::packages};
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "sovra-registered-functions-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn package(&self, name: &str, modules: &str, deps: &str) -> PathBuf {
            let root = self.0.join(name);
            std::fs::create_dir(&root).unwrap();
            std::fs::write(
                root.join("sovra.toml"),
                format!(
                    "[project]\nname=\"same-display-name\"\nentry=\"main.svr\"\n{modules}{deps}"
                ),
            )
            .unwrap();
            std::fs::write(root.join("main.svr"), "fn main(){}").unwrap();
            root
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn independent_consumer_prints_49_in_interpreter_and_javascript() {
        let package =
            packages::compile_registered_primitive_functions("examples/multi-file-packages/app")
                .unwrap();
        assert_eq!(package.checked_units().len(), 4);
        assert_eq!(interpreter::run(package.ir()).unwrap(), ["49"]);
        let js = backend::render_javascript(package.ir());
        let output = std::process::Command::new("node")
            .arg("--eval")
            .arg(js)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "49");
        assert!(package
            .checked_units()
            .iter()
            .all(|unit| unit.source().is_none())); // AST-normalized checked units; captured sources live in wrapper.
    }
    #[test]
    fn private_helpers_unused_bodies_and_original_error_spans_are_checked() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "[modules]\nmath=\"math.svr\"\n", "");
        let path = root.join("math.svr");
        std::fs::write(&path,"mod math{fn private()->Int{return 2;} export fn public()->Int{return math::private();}}").unwrap();
        std::fs::write(root.join("main.svr"), "fn main(){print(math::public());}").unwrap();
        assert_eq!(
            interpreter::run(
                packages::compile_registered_primitive_functions(&root)
                    .unwrap()
                    .ir()
            )
            .unwrap(),
            ["2"]
        );
        std::fs::write(root.join("main.svr"), "fn main(){math::private();}").unwrap();
        let errors = packages::compile_registered_primitive_functions(&root)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors
            .items
            .iter()
            .all(|d| d.source_file.as_deref() == Some(root.join("main.svr").to_str().unwrap())));
        std::fs::write(root.join("main.svr"), "fn main(){}").unwrap();
        let source = "// λ\r\nmod math{\r\nfn unused()->Int{return true;}\r\n}";
        std::fs::write(&path, source).unwrap();
        let errors = packages::compile_registered_primitive_functions(&root)
            .unwrap_err()
            .into_diagnostics();
        let program = sovra::compiler::parser::Parser::new()
            .parse_source(source)
            .unwrap();
        let expected = sovra::compiler::semantic::SemanticAnalyzer::new()
            .analyze(&program)
            .unwrap_err();
        assert_eq!(
            errors
                .items
                .iter()
                .map(|d| (d.code, &d.message, d.span))
                .collect::<Vec<_>>(),
            expected
                .items
                .iter()
                .map(|d| (d.code, &d.message, d.span))
                .collect::<Vec<_>>()
        );
        assert!(errors
            .items
            .iter()
            .all(|d| d.source_file.as_deref() == Some(path.to_str().unwrap())));
    }
    #[test]
    fn imports_stay_file_local_and_direct_and_records_are_explicitly_unavailable() {
        let fixture = Fixture::new();
        let root = fixture.package(
            "app",
            "[modules]\nlocal=\"local.svr\"\n",
            "[dependencies.library]\npath=\"../library\"\n",
        );
        let library = fixture.package("library", "[modules]\nmath=\"math.svr\"\n", "");
        std::fs::write(
            library.join("math.svr"),
            "mod math{export fn value()->Int{return 2;}}",
        )
        .unwrap();
        std::fs::write(
            root.join("main.svr"),
            "use library::math;fn main(){print(local::value());}",
        )
        .unwrap();
        std::fs::write(
            root.join("local.svr"),
            "mod local{export fn value()->Int{return library::math::value();}}",
        )
        .unwrap();
        let errors = packages::compile_registered_primitive_functions(&root)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors
            .items
            .iter()
            .any(|d| d.source_file.as_deref() == Some(root.join("local.svr").to_str().unwrap())));
        std::fs::write(root.join("main.svr"), "fn main(){}").unwrap();
        std::fs::write(root.join("local.svr"), "use missing::math;mod local{}").unwrap();
        assert!(packages::compile_registered_primitive_functions(&root)
            .unwrap_err()
            .into_diagnostics()
            .items
            .iter()
            .any(|d| d.code == "E4115"));
        std::fs::write(root.join("local.svr"), "mod local{struct Point{x:Int}}").unwrap();
        assert_eq!(
            packages::compile_registered_primitive_functions(&root)
                .unwrap_err()
                .into_diagnostics()
                .items[0]
                .code,
            "E4163"
        );
    }
    #[test]
    fn aliases_share_package_identity_and_float_coercions_survive_linking() {
        let fixture = Fixture::new();
        let root = fixture.package(
            "app",
            "",
            "[dependencies.left]\npath=\"../library\"\n[dependencies.right]\npath=\"../library\"\n",
        );
        let library = fixture.package("library", "[modules]\nmath=\"math.svr\"\n", "");
        std::fs::write(library.join("main.svr"), "fn main(){print(999);}").unwrap();
        std::fs::write(
            library.join("math.svr"),
            "mod math{type Scalar=Float;export fn widen(value:Scalar)->Scalar{return value;}}",
        )
        .unwrap();
        std::fs::write(root.join("main.svr"),"use left::math;use right::math;fn main(){print(left::math::widen(3)/2);print(right::math::widen(5)/2);}").unwrap();
        let package = packages::compile_registered_primitive_functions(&root).unwrap();
        assert_eq!(package.sources().inventory().graph.packages.len(), 2);
        assert_eq!(interpreter::run(package.ir()).unwrap(), ["1.5", "2.5"]);
        let output = std::process::Command::new("node")
            .arg("--eval")
            .arg(backend::render_javascript(package.ir()))
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["1.5", "2.5"]
        );
        let source = "use left::math;fn main(){left::math::widen(true);}";
        std::fs::write(root.join("main.svr"), source).unwrap();
        let errors = packages::compile_registered_primitive_functions(&root)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors
            .items
            .iter()
            .any(|d| &source[d.span.start..d.span.end] == "true"));
        assert!(errors
            .items
            .iter()
            .all(|d| d.source_file.as_deref() == Some(root.join("main.svr").to_str().unwrap())));
    }
    #[test]
    fn same_named_packages_and_modules_keep_distinct_link_targets() {
        let fixture = Fixture::new();
        let root = fixture.package(
            "app",
            "",
            "[dependencies.left]\npath=\"../left\"\n[dependencies.right]\npath=\"../right\"\n",
        );
        for (name, value) in [("left", 20), ("right", 30)] {
            let library = fixture.package(name, "[modules]\nmath=\"math.svr\"\n", "");
            std::fs::write(
                library.join("math.svr"),
                format!("mod math{{export fn value()->Int{{return {value};}}}}"),
            )
            .unwrap();
        }
        std::fs::write(root.join("main.svr"),"use left::math;use right::math;fn main(){print(left::math::value());print(right::math::value());}").unwrap();
        let package = packages::compile_registered_primitive_functions(root).unwrap();
        assert_eq!(interpreter::run(package.ir()).unwrap(), ["20", "30"]);
    }
    #[test]
    fn transitive_dependencies_do_not_become_direct_imports() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "", "[dependencies.middle]\npath=\"../middle\"\n");
        fixture.package("middle", "", "[dependencies.leaf]\npath=\"../leaf\"\n");
        let leaf = fixture.package("leaf", "[modules]\nmath=\"math.svr\"\n", "");
        std::fs::write(
            leaf.join("math.svr"),
            "mod math{export fn value()->Int{return 2;}}",
        )
        .unwrap();
        std::fs::write(root.join("main.svr"), "use leaf::math;fn main(){}").unwrap();
        let errors = packages::compile_registered_primitive_functions(root)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors
            .items
            .iter()
            .any(|d| d.code == "E4115" && d.message.contains("leaf")));
    }
    #[test]
    fn relocated_layouts_produce_the_same_linked_ir() {
        let first = Fixture::new();
        let second = Fixture::new();
        for fixture in [&first, &second] {
            for relative in [
                "app/sovra.toml",
                "app/main.svr",
                "app/application.svr",
                "utilities/sovra.toml",
                "utilities/main.svr",
                "utilities/arithmetic.svr",
            ] {
                let target = fixture.0.join(relative);
                std::fs::create_dir_all(target.parent().unwrap()).unwrap();
                std::fs::copy(
                    std::path::Path::new("examples/multi-file-packages").join(relative),
                    target,
                )
                .unwrap();
            }
        }
        let first = packages::compile_registered_primitive_functions(first.0.join("app")).unwrap();
        let second =
            packages::compile_registered_primitive_functions(second.0.join("app")).unwrap();
        assert_eq!(first.ir(), second.ir());
        assert_eq!(interpreter::run(second.ir()).unwrap(), ["49"]);
    }
    #[test]
    fn same_package_mutual_calls_keep_existing_recursion_rules() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "[modules]\neven=\"even.svr\"\nodd=\"odd.svr\"\n", "");
        std::fs::write(root.join("even.svr"),"mod even{export fn parity(n:Int)->Bool{if n==0{return true;}return odd::parity(n-1);}}").unwrap();
        std::fs::write(root.join("odd.svr"),"mod odd{export fn parity(n:Int)->Bool{if n==0{return false;}return even::parity(n-1);}}").unwrap();
        std::fs::write(root.join("main.svr"), "fn main(){print(even::parity(8));}").unwrap();
        let package = packages::compile_registered_primitive_functions(root).unwrap();
        assert_eq!(interpreter::run(package.ir()).unwrap(), ["true"]);
    }
}
