//! Nominal record values across source-owned registered module interfaces.
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
                "sovra-record-modules-{}-{}",
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
                format!("[project]\nname=\"same\"\nentry=\"main.svr\"\n{modules}{deps}"),
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
    fn standard_result_records_cross_registered_signatures_aliases_and_fields() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "[modules]\nvalues=\"values.svr\"\n", "");
        std::fs::write(root.join("values.svr"), "mod values{type Parsed=std::ParsedInt;export struct Box{result:Parsed} export fn parse(text:String)->Parsed{return std::parse_int(text);} export fn wrap(text:String)->Box{return Box{result:std::parse_int(text)};} export fn read(result:Parsed)->Int{return result.value;}}").unwrap();
        std::fs::write(root.join("main.svr"), "fn main(){print(values::read(values::parse(\"42\")));let boxed=values::wrap(\"bad\");print(boxed.result.ok);print(boxed.result.value);}").unwrap();
        let package = packages::compile_registered_records(&root).unwrap();
        assert_eq!(
            interpreter::run(package.ir()).unwrap(),
            ["42", "false", "0"]
        );
        let js = std::process::Command::new("node")
            .args(["--eval", &backend::render_javascript(package.ir())])
            .output()
            .unwrap();
        assert!(
            js.status.success(),
            "{}",
            String::from_utf8_lossy(&js.stderr)
        );
        assert_eq!(String::from_utf8(js.stdout).unwrap().trim(), "42\nfalse\n0");
    }
    #[test]
    fn imported_records_construct_pass_return_and_widen_in_both_engines() {
        let fixture = Fixture::new();
        let root = fixture.package(
            "app",
            "",
            "[dependencies.shapes]\npath=\"../shapes\"\n[dependencies.same]\npath=\"../shapes\"\n",
        );
        let shapes = fixture.package("shapes", "[modules]\ngeometry=\"geometry.svr\"\n", "");
        std::fs::write(shapes.join("geometry.svr"),"mod geometry{type Scalar=Float;export struct Point{x:Scalar} export fn origin()->Point{return Point{x:3};} export fn read(point:Point)->Scalar{return point.x;}}").unwrap();
        std::fs::write(root.join("main.svr"),"use shapes::geometry;use same::geometry;type Alias=shapes::geometry::Point;fn main(){let point=Alias{x:3};print(shapes::geometry::read(point)/2);print(same::geometry::read(shapes::geometry::origin())/2);}").unwrap();
        let package = packages::compile_registered_records(&root).unwrap();
        assert_eq!(interpreter::run(package.ir()).unwrap(), ["1.5", "1.5"]);
        let output = std::process::Command::new("node")
            .arg("--eval")
            .arg(backend::render_javascript(package.ir()))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["1.5", "1.5"]
        );
        assert!(packages::compile_registered_primitive_functions(&root).is_err());
    }
    #[test]
    fn same_shape_types_in_different_roots_do_not_unify() {
        let fixture = Fixture::new();
        let root = fixture.package(
            "app",
            "",
            "[dependencies.left]\npath=\"../left\"\n[dependencies.right]\npath=\"../right\"\n",
        );
        for name in ["left", "right"] {
            let library = fixture.package(name, "[modules]\ngeometry=\"geometry.svr\"\n", "");
            std::fs::write(library.join("geometry.svr"),"mod geometry{export struct Point{x:Int} export fn read(point:Point)->Int{return point.x;}}").unwrap();
        }
        let source="use left::geometry;use right::geometry;fn main(){let p=left::geometry::Point{x:1};right::geometry::read(p);}";
        std::fs::write(root.join("main.svr"), source).unwrap();
        let errors = packages::compile_registered_records(&root)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors.items.iter().any(|d| d.message.contains("argument")));
        assert!(errors
            .items
            .iter()
            .all(|d| d.source_file.as_deref() == Some(root.join("main.svr").to_str().unwrap())));
    }
    #[test]
    fn same_package_record_calls_and_local_aliases_keep_source_owned_fields() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "[modules]\ngeometry=\"geometry.svr\"\n", "");
        let source="// λ\r\nmod geometry{\r\nexport struct Point{x:Float}\r\nexport fn make()->Point{return Point{x:3};}\r\n}";
        std::fs::write(root.join("geometry.svr"), source).unwrap();
        std::fs::write(
            root.join("main.svr"),
            "type Local=geometry::Point;fn main(){let p:Local=geometry::make();print(p.x/2);}",
        )
        .unwrap();
        assert_eq!(
            interpreter::run(packages::compile_registered_records(&root).unwrap().ir()).unwrap(),
            ["1.5"]
        );
        std::fs::write(
            root.join("main.svr"),
            "fn main(){let p=geometry::Point{x:true};}",
        )
        .unwrap();
        let errors = packages::compile_registered_records(&root)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors
            .items
            .iter()
            .all(|d| d.source_file.as_deref() == Some(root.join("main.svr").to_str().unwrap())));
    }
    #[test]
    fn private_record_names_and_public_signature_leaks_are_rejected() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "", "[dependencies.shapes]\npath=\"../shapes\"\n");
        let shapes = fixture.package("shapes", "[modules]\ngeometry=\"geometry.svr\"\n", "");
        std::fs::write(
            shapes.join("geometry.svr"),
            "mod geometry{struct Secret{x:Int} export struct Point{x:Int}}",
        )
        .unwrap();
        std::fs::write(
            root.join("main.svr"),
            "use shapes::geometry;fn main(){let p=shapes::geometry::Secret{x:1};}",
        )
        .unwrap();
        let errors = packages::compile_registered_records(&root)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors
            .items
            .iter()
            .all(|d| d.source_file.as_deref() == Some(root.join("main.svr").to_str().unwrap())));
        std::fs::write(root.join("main.svr"), "fn main(){}").unwrap();
        let source="// λ\r\nmod geometry{struct Secret{x:Int} export fn leak()->Secret{return Secret{x:1};}}";
        std::fs::write(shapes.join("geometry.svr"), source).unwrap();
        let errors = packages::compile_registered_records(&root)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors
            .items
            .iter()
            .any(|d| d.code == "E4116" && &source[d.span.start..d.span.end] == "Secret"));
        assert!(errors.items.iter().all(
            |d| d.source_file.as_deref() == Some(shapes.join("geometry.svr").to_str().unwrap())
        ));
    }
    #[test]
    fn invalid_fields_and_record_valued_contexts_keep_provider_ownership() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "[modules]\ngeometry=\"geometry.svr\"\n", "");
        for (source, code) in [
            ("mod geometry{export struct Point{x:Int,x:Float}}", "E3008"),
            ("mod geometry{export struct Point{x:Missing}}", "E3017"),
            (
                "mod geometry{export struct Point{x:Int} export struct Point{x:Int}}",
                "E3008",
            ),
        ] {
            std::fs::write(root.join("geometry.svr"), source).unwrap();
            let errors = packages::compile_registered_records(&root)
                .unwrap_err()
                .into_diagnostics();
            assert!(errors.items.iter().any(|d| d.code == code), "{source}");
            assert!(errors
                .items
                .iter()
                .all(|d| d.source_file.as_deref()
                    == Some(root.join("geometry.svr").to_str().unwrap())));
        }
    }
    #[test]
    fn independent_record_example_runs_on_both_backends() {
        let package =
            packages::compile_registered_records("examples/multi-file-records/app").unwrap();
        assert_eq!(interpreter::run(package.ir()).unwrap(), ["25", "1.5"]);
        let output = std::process::Command::new("node")
            .arg("--eval")
            .arg(backend::render_javascript(package.ir()))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["25", "1.5"]
        );
    }
    #[test]
    fn root_records_and_a_module_named_root_have_distinct_identities() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "[modules]\nroot=\"root.svr\"\n", "");
        std::fs::write(
            root.join("root.svr"),
            "mod root{export struct Point{x:Int} export fn read(p:Point)->Int{return p.x;}}",
        )
        .unwrap();
        std::fs::write(root.join("main.svr"),"struct Point{x:Int} fn main(){let a=Point{x:1};let b=root::Point{x:2};print(a.x);print(root::read(b));}").unwrap();
        let package = packages::compile_registered_records(&root).unwrap();
        assert_eq!(interpreter::run(package.ir()).unwrap(), ["1", "2"]);
        std::fs::write(
            root.join("main.svr"),
            "struct Point{x:Int} fn main(){let a=Point{x:1};root::read(a);}",
        )
        .unwrap();
        let errors = packages::compile_registered_records(&root)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors.items.iter().any(|d| d.message.contains("argument")));
        let legacy = fixture.package("legacy", "", "");
        std::fs::write(legacy.join("main.svr"),"struct Point{x:Int} mod root{export struct Point{x:Int}} fn main(){let a=Point{x:1};let b=root::Point{x:2};print(a.x);print(b.x);}").unwrap();
        let ir = packages::compile(legacy).unwrap();
        let identities = ir
            .functions
            .iter()
            .flat_map(|function| &function.instructions)
            .filter_map(|instruction| match instruction {
                sovra::compiler::ir::Instruction::MakeStruct { type_name, .. } => {
                    Some(type_name.as_str())
                }
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(identities.len(), 2);
        assert_eq!(interpreter::run(&ir).unwrap(), ["1", "2"]);
    }
    #[test]
    fn nested_record_fields_follow_aliases_and_visible_imports_in_both_engines() {
        let fixture = Fixture::new();
        let root = fixture.package(
            "app",
            "",
            "[dependencies.shapes]\npath=\"../shapes\"\n[dependencies.boxes]\npath=\"../boxes\"\n",
        );
        let shapes = fixture.package("shapes", "[modules]\ngeometry=\"geometry.svr\"\n", "");
        std::fs::write(shapes.join("geometry.svr"),"mod geometry{export struct Point{x:Float} export fn origin()->Point{return Point{x:3};}}").unwrap();
        let boxes = fixture.package(
            "boxes",
            "[modules]\ncontainer=\"container.svr\"\n",
            "[dependencies.shapes]\npath=\"../shapes\"\n",
        );
        std::fs::write(boxes.join("container.svr"),"use shapes::geometry;mod container{type Item=shapes::geometry::Point;export struct Box{point:Item} export fn make()->Box{return Box{point:shapes::geometry::origin()};}}").unwrap();
        std::fs::write(root.join("main.svr"),"use boxes::container;use shapes::geometry;fn main(){let b=boxes::container::Box{point:shapes::geometry::Point{x:3}};print(b.point.x/2);print(boxes::container::make().point.x/2);}").unwrap();
        let package = packages::compile_registered_records(&root).unwrap();
        assert_eq!(interpreter::run(package.ir()).unwrap(), ["1.5", "1.5"]);
        let output = std::process::Command::new("node")
            .arg("--eval")
            .arg(backend::render_javascript(package.ir()))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["1.5", "1.5"]
        );
        let wrong="use boxes::container;struct Imposter{x:Float} fn main(){let b=boxes::container::Box{point:Imposter{x:3}};}";
        std::fs::write(root.join("main.svr"), wrong).unwrap();
        let errors = packages::compile_registered_records(&root)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors.items.iter().any(|d| d.message.contains("point")));
        assert!(errors
            .items
            .iter()
            .all(|d| d.source_file.as_deref() == Some(root.join("main.svr").to_str().unwrap())));
        // Returned public nested types support field access without granting a
        // transitive source import or constructor alias to the consumer.
        std::fs::write(root.join("sovra.toml"),"[project]\nname=\"app\"\nentry=\"main.svr\"\n[dependencies.boxes]\npath=\"../boxes\"\n").unwrap();
        std::fs::write(
            root.join("main.svr"),
            "use boxes::container;fn main(){print(boxes::container::make().point.x/2);}",
        )
        .unwrap();
        assert_eq!(
            interpreter::run(packages::compile_registered_records(&root).unwrap().ir()).unwrap(),
            ["1.5"]
        );
        std::fs::write(root.join("main.svr"), "use shapes::geometry;fn main(){}").unwrap();
        assert!(packages::compile_registered_records(&root)
            .unwrap_err()
            .into_diagnostics()
            .items
            .iter()
            .any(|d| d.code == "E4115"));
    }
    #[test]
    fn public_record_fields_cannot_leak_private_record_types() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "[modules]\ngeometry=\"geometry.svr\"\n", "");
        let source = "// λ\r\nmod geometry{struct Secret{x:Int} export struct Box{value:Secret}}";
        std::fs::write(root.join("geometry.svr"), source).unwrap();
        let errors = packages::compile_registered_records(root)
            .unwrap_err()
            .into_diagnostics();
        assert!(errors
            .items
            .iter()
            .any(|d| d.code == "E4116" && &source[d.span.start..d.span.end] == "Secret"));
    }
    #[test]
    fn recursive_record_declarations_keep_existing_core_behavior_without_layout_expansion() {
        let fixture = Fixture::new();
        let root = fixture.package("app", "[modules]\nnodes=\"nodes.svr\"\n", "");
        let source = "mod nodes{export struct Node{next:Node}}";
        std::fs::write(root.join("nodes.svr"), source).unwrap();
        let program = sovra::compiler::parser::Parser::new()
            .parse_source(source)
            .unwrap();
        assert!(sovra::compiler::semantic::SemanticAnalyzer::new()
            .analyze(&program)
            .is_ok());
        let package = packages::compile_registered_records(root).unwrap();
        assert!(interpreter::run(package.ir()).unwrap().is_empty());
    }
}
