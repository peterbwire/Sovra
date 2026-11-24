//! Checked snapshot array growth/slicing, explicit recursive widening and bounds.
use sovra::compiler::v2::*;
fn compile(source: &str) -> CompiledGenericSource {
    compile_generic_array_source(source, "main", GenericPropagationSourceLimits::DEFAULT).unwrap()
}
fn run(source: &str) -> GenericSourceOutcome {
    execute_generic_source(&compile(source), GenericVmPolicy::DEFAULT).unwrap()
}
#[test]
fn append_concat_and_slice_preserve_owned_snapshots_and_source_arrays() {
    let source="fn main()->[[Int]]{let mut a:[Int]=[1,2];let appended=std::array_append(a,3);let other:[Int]=[4,5];let joined=std::array_concat(appended,other);let part=std::array_slice(joined,1,4);a[0]=99;return [appended,joined,part,a];}";
    let out = run(source);
    let root = out.execution().value();
    for (i, expected) in [
        [1, 2, 3].as_slice(),
        [1, 2, 3, 4, 5].as_slice(),
        [2, 3, 4].as_slice(),
        [99, 2].as_slice(),
    ]
    .iter()
    .enumerate()
    {
        let row = root.array_item(i).unwrap();
        assert_eq!(row.array_len(), Some(expected.len()));
        for (j, n) in expected.iter().enumerate() {
            assert_eq!(row.array_item(j).unwrap().as_int(), Some(*n));
        }
    }
}
#[test]
fn append_and_concat_widen_int_to_float_including_nested_arrays() {
    let out=run("fn main()->[[Float]]{let a:[[Float]]=[[1.5]];let b:[Int]=[2,3];let x=std::array_append(a,b);let more:[[Int]]=[[4,5]];return std::array_concat(x,more);}");
    let v = out.execution().value();
    assert_eq!(v.array_len(), Some(3));
    assert_eq!(
        v.array_item(0).unwrap().array_item(0).unwrap().as_float(),
        Some(1.5)
    );
    assert_eq!(
        v.array_item(1).unwrap().array_item(1).unwrap().as_float(),
        Some(3.0)
    );
    assert_eq!(
        v.array_item(2).unwrap().array_item(0).unwrap().as_float(),
        Some(4.0)
    );
    let empty=run("fn main()->[Float]{let a:[Float]=[];let b:[Int]=[];return std::array_append(std::array_concat(a,b),7);}");
    assert_eq!(
        empty.execution().value().array_item(0).unwrap().as_float(),
        Some(7.0)
    );
}
#[test]
fn generic_growth_handles_nominal_and_opaque_element_types() {
    let source="struct Box<T>{value:T}fn add<T>(xs:[T],x:T)->[T]{return std::array_append(xs,x);}fn join<T>(a:[T],b:[T])->[T]{return std::array_concat(a,b);}fn part<T>(a:[T])->[T]{return std::array_slice(a,0,1);}fn main()->[Box<Int>]{let a:[Box<Int>]=[];let b=add::<Box<Int>>(a,Box::<Int>{value:8});let c:[Box<Int>]=[Box::<Int>{value:9}];return part::<Box<Int>>(join::<Box<Int>>(b,c));}";
    let out = run(source);
    assert_eq!(out.execution().value().array_len(), Some(1));
    assert_eq!(
        out.execution()
            .value()
            .array_item(0)
            .unwrap()
            .field(0)
            .unwrap()
            .as_int(),
        Some(8)
    );
}
#[test]
fn slice_half_open_bounds_allow_empty_edges_and_refuse_invalid_ranges() {
    for (start, end, size) in [(0, 0, 0), (0, 2, 2), (2, 2, 0), (1, 2, 1)] {
        let out = run(&format!(
            "fn main()->[Int]{{let a:[Int]=[1,2];return std::array_slice(a,{start},{end});}}"
        ));
        assert_eq!(out.execution().value().array_len(), Some(size));
    }
    for (start, end) in [
        ("0-1", "1"),
        ("0", "0-1"),
        ("2", "1"),
        ("0", "3"),
        ("9223372036854775807", "9223372036854775807"),
    ] {
        let source = format!(
            "// λ\nfn main()->[Int]{{let a:[Int]=[1,2];return std::array_slice(a,{start},{end});}}"
        );
        let p = compile(&source);
        let e = execute_generic_source(&p, GenericVmPolicy::DEFAULT).unwrap_err();
        assert!(matches!(e.error.kind, GenericVmErrorKind::Bounds));
        let span = e.span.unwrap();
        assert!(source[span.start..span.end].starts_with("std::array_slice("));
    }
}
#[test]
fn array_builtins_reject_incompatible_elements_wrong_bounds_and_type_arguments() {
    for body in [
        "let a:[Int]=[1];std::array_append(a,true);",
        "let a:[Int]=[1];let b:[Float]=[2.0];std::array_concat(a,b);",
        "std::array_append(1,2);",
        "let a:[Int]=[];std::array_slice(a,0.0,1);",
        "let a:[Int]=[];std::array_slice(a,0);",
        "let a:[Int]=[];std::array_concat::<Int>(a,a);",
    ] {
        let source = format!("fn main(){{}}fn unused(){{{body}}}");
        assert!(
            compile_generic_array_source(&source, "main", GenericPropagationSourceLimits::DEFAULT)
                .is_err(),
            "{body}"
        );
    }
}
#[test]
fn array_snapshot_conversion_and_temporary_storage_use_shared_exact_budgets() {
    let source="fn main()->[[Float]]{let a:[[Float]]=[[1.0]];let b:[[Int]]=[[2,3]];return std::array_concat(a,b);}";
    let p = compile(source);
    let out = execute_generic_source(&p, GenericVmPolicy::DEFAULT).unwrap();
    let d = GenericVmPolicy::DEFAULT;
    let rem = out.execution().remaining();
    let mut exact = d.clone();
    exact.max_work = out.execution().work();
    exact.runtime.execution.max_instructions = Some(out.execution().instructions());
    exact.runtime.materialization.max_value_nodes = Some(
        d.runtime.materialization.max_value_nodes.unwrap()
            - rem.materialization.max_value_nodes.unwrap(),
    );
    exact.runtime.materialization.max_text_bytes = Some(
        d.runtime.materialization.max_text_bytes.unwrap()
            - rem.materialization.max_text_bytes.unwrap(),
    );
    assert!(execute_generic_source(&p, exact.clone()).is_ok());
    for dimension in 0..3 {
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
            _ => *bad.runtime.execution.max_instructions.as_mut().unwrap() -= 1,
        };
        assert!(execute_generic_source(&p, bad).is_err());
    }
}
#[test]
fn native_array_profile_rechecks_targets_and_refuses_older_environment_downgrades() {
    let source =
        "fn main(){let a:[Float]=[];let b=std::array_append(a,1);if b[0]!=1.0{let bad=1/0;}}";
    let p = compile(source);
    let image = p.encode_generic_native_image().unwrap();
    assert_eq!(&image[8..12], &511u32.to_le_bytes());
    assert!(execute_generic_native_image(&image).is_ok());
    assert!(
        compile_generic_value_source(source, "main", GenericPropagationSourceLimits::DEFAULT)
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
        255,
        256,
        383,
        447,
        479,
        495,
        503,
        507,
        509,
        510,
        u32::MAX,
    ] {
        let mut bad = image.clone();
        bad[8..12].copy_from_slice(&word.to_le_bytes());
        assert!(decode_generic_native_image(&bad).is_err(), "{word}");
    }
    assert!(p.ir().functions()[0]
        .instructions()
        .iter()
        .any(|i| matches!(
            i.operation,
            GenericIrOperation::Builtin {
                builtin: GenericBuiltin::ArrayAppend,
                right_layout: Some(_),
                ..
            }
        )));
}

#[test]
fn nested_string_array_copies_preserve_text_and_enforce_shared_byte_limits() {
    let source="fn main()->[[String]]{let mut item:[String]=[\"old\"];let a:[[String]]=[];let b=std::array_append(a,item);item[0]=\"new\";return std::array_slice(b,0,1);}";
    let p = compile(source);
    let out = execute_generic_source(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(
        out.execution()
            .value()
            .array_item(0)
            .unwrap()
            .array_item(0)
            .unwrap()
            .as_str(),
        Some("old")
    );
    let consumed = GenericVmPolicy::DEFAULT
        .runtime
        .materialization
        .max_text_bytes
        .unwrap()
        - out
            .execution()
            .remaining()
            .materialization
            .max_text_bytes
            .unwrap();
    let mut exact = GenericVmPolicy::DEFAULT;
    exact.runtime.materialization.max_text_bytes = Some(consumed);
    assert!(execute_generic_source(&p, exact.clone()).is_ok());
    exact.runtime.materialization.max_text_bytes = Some(consumed - 1);
    assert!(execute_generic_source(&p, exact).is_err());
}
