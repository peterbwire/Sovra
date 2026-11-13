//! Approved G13 structural array annotations and snapshot behavior.
use sovra::compiler::{
    backend, compile_program, interpreter, parser::Parser, semantic::SemanticAnalyzer,
};
fn execute(source: &str, expected: &[&str]) {
    let program = Parser::new().parse_source(source).unwrap();
    let (ir, _) = compile_program(&program).unwrap();
    assert_eq!(interpreter::run(&ir).unwrap(), expected);
    let output = std::process::Command::new("node")
        .args(["--eval", &backend::render_javascript(&ir)])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        expected.join("\n")
    );
}
#[test]
fn typed_arrays_cross_aliases_parameters_returns_fields_and_empty_literals() {
    execute("type Numbers=[Float];struct Batch{values:Numbers} fn widen(values:[Int])->Numbers{return values;} fn empty()->[[Int]]{return [];} fn main(){let items:Numbers=[1,2];let batch=Batch{values:widen([3,4])};print(items[0]/2);print(batch.values[0]/2);let rows:[[Int]]=[[],[7]];print(rows[1][0]);print(std::array_len(empty()));}", &["0.5","1.5","7","0"]);
}
#[test]
fn module_local_array_aliases_and_record_elements_keep_snapshot_semantics() {
    execute("mod values{struct Point{x:Int} type Points=[Point];export fn first()->Int{let original:Points=[Point{x:7}];let mut copy:Points=original;copy[0]=Point{x:9};return original[0].x;}} fn main(){print(values::first());print(std::array_len([[],[1]]));}", &["7","2"]);
}
#[test]
fn invalid_element_types_alias_cycles_and_nonarray_lengths_fail_semantically() {
    for source in [
        "fn bad(items:[Missing]){}",
        "fn bad(items:[Int]){} fn main(){bad([true]);}",
        "fn main(){let items:[Int]=[1,true];}",
        "type A=[B];type B=[A];fn main(){}",
        "fn main(){std::array_len(1);}",
        "fn main(){std::array_len(\"text\");}",
    ] {
        let program = Parser::new().parse_source(source).unwrap();
        assert!(
            SemanticAnalyzer::new().analyze(&program).is_err(),
            "{source}"
        );
    }
}
#[test]
fn type_expression_depth_is_bounded_before_recursive_type_work() {
    for (layers, accepted) in [(127, true), (128, false), (10000, false)] {
        let source = format!(
            "fn accept(items:{}Int{}){{}}",
            "[".repeat(layers),
            "]".repeat(layers)
        );
        let parsed = Parser::new().parse_source(&source);
        assert_eq!(parsed.is_ok(), accepted, "{layers}");
        if let Ok(program) = parsed {
            assert!(SemanticAnalyzer::new().analyze(&program).is_ok());
        }
    }
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
fn independent_registered_library_exports_arrays_records_and_owned_results() {
    let package = sovra::compiler::project::packages::compile_registered_records(
        "examples/typed-array-packages/app",
    )
    .unwrap();
    let expected = ["9", "1.5", "0", "1", "4.5"];
    assert_eq!(interpreter::run(package.ir()).unwrap(), expected);
    let js = std::process::Command::new("node")
        .args(["--eval", &backend::render_javascript(package.ir())])
        .output()
        .unwrap();
    assert!(
        js.status.success(),
        "{}",
        String::from_utf8_lossy(&js.stderr)
    );
    assert_eq!(
        String::from_utf8(js.stdout).unwrap().trim(),
        expected.join("\n")
    );
}
#[test]
fn annotation_spans_and_array_nodes_keep_exact_admission() {
    let source = "fn echo(items:[[Int]])->[[Int]]{let copy:[[Int]]=items;return copy;}";
    let program = Parser::new().parse_source(source).unwrap();
    let function = &program.functions[0];
    for span in [
        function.parameters[0].type_span.unwrap(),
        function.return_type_span.unwrap(),
    ] {
        assert_eq!(&source[span.start..span.end], "[[Int]]");
    }
    let count = sovra::compiler::resources::check_ast_budget(&program, usize::MAX).unwrap();
    assert_eq!(count, 12);
    assert!(sovra::compiler::resources::check_ast_budget(&program, count).is_ok());
    assert!(sovra::compiler::resources::check_ast_budget(&program, count - 1).is_err());
}

#[test]
fn recursive_widening_at_function_boundaries_preserves_int_sources() {
    execute("fn convert(values:[[Float]])->[[Float]]{return values;} fn returned()->[Float]{return [3];} fn main(){let original=[[1]];let converted=convert(original);print(original[0][0]/2);print(converted[0][0]/2);print(returned()[0]/2);print(std::array_len([]));}", &["0","0.5","1.5","0"]);
}
#[test]
fn expanded_alias_depth_and_malformed_annotations_fail_without_partial_types() {
    for source in [
        "fn bad(value:[]){ }",
        "fn bad(value:[Int){ }",
        "fn bad(value:[Int,String]){}",
    ] {
        assert!(Parser::new().parse_source(source).is_err());
    }
    for (layers, accepted) in [(127, true), (128, false), (1000, false)] {
        let mut source = "type A0=Int;".to_owned();
        for index in 1..=layers {
            source.push_str(&format!("type A{index}=[A{}];", index - 1));
        }
        source.push_str(&format!("fn accept(values:A{layers}){{}}"));
        let program = Parser::new().parse_source(&source).unwrap();
        assert_eq!(
            SemanticAnalyzer::new().analyze(&program).is_ok(),
            accepted,
            "{layers}"
        );
    }
}
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
fn registered_array_record_elements_preserve_nominal_identity_and_private_boundaries() {
    use sovra::compiler::project::packages;
    let root = std::env::temp_dir().join(format!("sovra-g13-interfaces-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    struct Temp(std::path::PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _temp = Temp(root.clone());
    std::fs::write(
        root.join("sovra.toml"),
        "[project]\nname=\"arrays\"\nentry=\"main.svr\"\n[modules]\nvalues=\"values.svr\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("main.svr"),
        "fn main(){let items=values::make();print(items[0].x/2);}",
    )
    .unwrap();
    std::fs::write(
        root.join("values.svr"),
        "mod values{export struct Point{x:Float} export fn make()->[Point]{return [Point{x:3}];}}",
    )
    .unwrap();
    let package = packages::compile_registered_records(&root).unwrap();
    assert_eq!(interpreter::run(package.ir()).unwrap(), ["1.5"]);
    std::fs::write(root.join("values.svr"), "mod values{export struct Point{x:Float} export fn read(items:[Point])->Float{return items[0].x;}}").unwrap();
    std::fs::write(
        root.join("main.svr"),
        "struct Point{x:Float} fn main(){values::read([Point{x:3}]);}",
    )
    .unwrap();
    let nominal = packages::compile_registered_records(&root)
        .unwrap_err()
        .into_diagnostics();
    assert!(nominal.items.iter().any(|error| error.code == "E3007"));
    // The entry-only compiler uses the same normalized structural interface.
    std::fs::write(root.join("main.svr"), "fn main(){}").unwrap();
    for (source, code) in [
        (
            "mod values{struct Secret{x:Int} export fn leak()->[Secret]{return [];}}",
            "E4116",
        ),
        (
            "mod values{struct Secret{x:Int} export struct Leak{items:[[Secret]]}}",
            "E4116",
        ),
        ("mod values{export fn invalid(items:[Missing]){}}", "E3017"),
    ] {
        std::fs::write(root.join("values.svr"), source).unwrap();
        let errors = packages::compile_registered_records(&root)
            .unwrap_err()
            .into_diagnostics();
        assert!(
            errors.items.iter().any(|error| error.code == code),
            "{errors:?}"
        );
        assert!(errors
            .items
            .iter()
            .all(|error| error.source_file.as_deref() == root.join("values.svr").to_str()));
    }
}

#[test]
fn entry_only_package_interfaces_and_legacy_lowering_preserve_array_widening() {
    use sovra::compiler::project::packages;
    let root = std::env::temp_dir().join(format!("sovra-g13-entry-arrays-{}", std::process::id()));
    for name in ["app", "library"] {
        std::fs::create_dir_all(root.join(name)).unwrap();
    }
    struct Temp(std::path::PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _temp = Temp(root.clone());
    std::fs::write(root.join("app/sovra.toml"),"[project]\nname=\"app\"\nentry=\"main.svr\"\n[dependencies.library]\npath=\"../library\"\n").unwrap();
    std::fs::write(
        root.join("library/sovra.toml"),
        "[project]\nname=\"library\"\nentry=\"main.svr\"\n",
    )
    .unwrap();
    std::fs::write(root.join("library/main.svr"),"mod data{export struct Values{items:[Float]} export fn make(input:[Int])->Values{return Values{items:input};} export fn direct(input:[Int])->[Float]{return input;}}").unwrap();
    std::fs::write(root.join("app/main.svr"),"use library::data;fn main(){let result=library::data::make([3]);print(result.items[0]/2);print(library::data::direct([1])[0]/2);}").unwrap();
    for ir in [
        packages::compile(root.join("app")).unwrap(),
        packages::compile_with_legacy_lowering(
            root.join("app"),
            sovra::compiler::resources::PackageCompilationLimits::DEFAULT,
        )
        .unwrap(),
    ] {
        assert_eq!(interpreter::run(&ir).unwrap(), ["1.5", "0.5"]);
    }
}
