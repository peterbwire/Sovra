//! Polymorphic display and array length under explicit checked value profiles.
use sovra::compiler::v2::*;
fn compile(source: &str) -> CompiledGenericSource {
    compile_generic_value_source(source, "main", GenericPropagationSourceLimits::DEFAULT).unwrap()
}
fn run(source: &str) -> GenericSourceOutcome {
    execute_generic_source(&compile(source), GenericVmPolicy::DEFAULT).unwrap()
}
#[test]
fn primitive_display_preserves_existing_value_conventions() {
    for (value, expected) in [
        ("42", "42"),
        ("0-7", "-7"),
        ("1.5", "1.5"),
        ("true", "true"),
        ("false", "false"),
        ("\"é text\"", "é text"),
        ("unit()", ""),
    ] {
        let out = run(&format!(
            "fn unit(){{}}fn main()->String{{return std::to_string({value});}}"
        ));
        assert_eq!(out.execution().value().as_str(), Some(expected));
    }
}
#[test]
fn arrays_records_and_enums_have_iterative_versioned_display() {
    for (source,expected) in [
        ("fn main()->String{let x:[[Int]]=[[1,2],[],[3]];return std::to_string(x);}","[[1, 2], [], [3]]"),
        ("struct Pair<A,B>{left:A,right:B}fn main()->String{return std::to_string(Pair::<Int,String>{right:\"two\",left:1});}","Pair { left: 1, right: two }"),
        ("fn main()->String{return std::to_string(std::Option::<Int>::Some(7));}","std::Option::Some(7)"),
        ("fn main()->String{return std::to_string(std::Option::<Int>::None());}","std::Option::None()"),
        ("fn main()->String{return std::to_string(std::parse_int(\"7\"));}","std::ParsedInt { ok: true, value: 7 }")
    ] {let out=run(source);assert_eq!(out.execution().value().as_str(),Some(expected));assert_eq!(out.execution().value().footprint(),(1,expected.len() as u64));}
}
#[test]
fn unconstrained_generic_display_and_generic_array_length_recheck_each_instance() {
    let source="struct Box<T>{value:T}fn show<T>(x:T)->String{return std::to_string(x);}fn size<T>(xs:[T])->Int{return std::array_len(xs);}fn main()->String{let a:[Int]=[1,2];let b:[String]=[\"x\"];let first=size::<Int>(a);let second=size::<String>(b);if first+second!=3{let bad=1/0;}return show::<Box<[Int]>>(Box::<[Int]>{value:a});}";
    assert_eq!(
        run(source).execution().value().as_str(),
        Some("Box { value: [1, 2] }")
    );
    assert_eq!(
        run("fn main()->Int{let xs:[std::Option<Int>]=[];return std::array_len(xs);}")
            .execution()
            .value()
            .as_int(),
        Some(0)
    );
}
#[test]
fn display_admits_output_and_pending_work_before_allocation() {
    let p = compile("fn main()->String{let xs:[[Int]]=[[1,2],[3,4]];return std::to_string(xs);}");
    let out = execute_generic_source(&p, GenericVmPolicy::DEFAULT).unwrap();
    let d = GenericVmPolicy::DEFAULT;
    let remaining = out.execution().remaining();
    let mut exact = d.clone();
    exact.max_work = out.execution().work();
    exact.runtime.execution.max_instructions = Some(out.execution().instructions());
    exact.runtime.materialization.max_value_nodes = Some(
        d.runtime.materialization.max_value_nodes.unwrap()
            - remaining.materialization.max_value_nodes.unwrap(),
    );
    exact.runtime.materialization.max_text_bytes = Some(
        d.runtime.materialization.max_text_bytes.unwrap()
            - remaining.materialization.max_text_bytes.unwrap(),
    );
    assert!(execute_generic_source(&p, exact.clone()).is_ok());
    for dimension in 0..4 {
        let mut bad = exact.clone();
        match dimension {
            0 => bad.max_work -= 1,
            1 => {
                *bad.runtime
                    .materialization
                    .max_value_nodes
                    .as_mut()
                    .unwrap() -= 1
            }
            2 => *bad.runtime.materialization.max_text_bytes.as_mut().unwrap() -= 1,
            _ => *bad.runtime.execution.max_instructions.as_mut().unwrap() -= 1,
        };
        assert!(execute_generic_source(&p, bad).is_err());
    }
}
#[test]
fn value_builtin_profile_is_versioned_and_old_string_calls_remain_strict() {
    let source="fn main(){let xs:[Int]=[1,2];if std::array_len(xs)!=2{let bad=1/0;}if std::to_string(xs)!=\"[1, 2]\"{let bad=1/0;}}";
    let p = compile(source);
    assert!(p.has_standard_values());
    let image = p.encode_generic_native_image().unwrap();
    assert_eq!(&image[8..12], &255u32.to_le_bytes());
    assert!(execute_generic_native_image(&image).is_ok());
    assert!(
        compile_generic_string_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
            .is_err()
    );
    for word in [
        1u32,
        3,
        7,
        15,
        31,
        63,
        127,
        128,
        191,
        223,
        239,
        247,
        251,
        253,
        254,
        u32::MAX,
    ] {
        let mut bad = image.clone();
        bad[8..12].copy_from_slice(&word.to_le_bytes());
        assert!(decode_generic_native_image(&bad).is_err(), "{word}");
    }
}
#[test]
fn array_argument_arity_and_type_arguments_are_checked_even_in_unused_bodies() {
    for call in [
        "std::to_string()",
        "std::to_string(1,2)",
        "std::to_string::<Int>(1)",
        "std::array_len(1)",
        "std::array_len(\"text\")",
        "std::array_len()",
    ] {
        let source = format!("fn main(){{}}fn unused(){{{call};}}");
        assert!(
            compile_generic_value_source(&source, "main", GenericPropagationSourceLimits::DEFAULT)
                .is_err(),
            "{call}"
        );
    }
    assert!(compile_generic_value_source(
        "fn main(){}fn bad<T>(x:T)->Int{return std::array_len(x);}",
        "main",
        GenericPropagationSourceLimits::DEFAULT
    )
    .is_err());
}

#[test]
fn display_handles_runtime_recursive_value_depth_without_recursive_traversal() {
    let source="struct Node<T>{next:[Node<T>],value:T}fn main()->String{let mut xs:[Node<Int>]=[];let mut n=0;while n<200{let node=Node::<Int>{next:xs,value:n};xs=[node];n=n+1;}return std::to_string(xs);}";
    let out = run(source);
    let text = out.execution().value().as_str().unwrap();
    assert_eq!(text.matches("Node { next: ").count(), 200);
    assert!(text.starts_with("[Node { next: [Node"));
    assert!(text.ends_with("value: 199 }]"));
}
