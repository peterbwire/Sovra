//! Strict graph/source image admission and startup rechecking without filesystem authority.
use sovra::compiler::v2::*;
fn num(out: &mut Vec<u8>, n: usize) {
    out.extend_from_slice(&(n as u32).to_le_bytes());
}
fn text(out: &mut Vec<u8>, s: &[u8]) {
    num(out, s.len());
    out.extend_from_slice(s);
}
fn image(
    packages: &[Vec<(&str, usize)>],
    sources: &[(usize, Option<&str>, &str, &[u8])],
) -> Vec<u8> {
    let mut out = b"SVRGP\0\x01\x02".to_vec();
    out.extend_from_slice(&511u32.to_le_bytes());
    num(&mut out, 0);
    num(&mut out, packages.len());
    for edges in packages {
        num(&mut out, edges.len());
        for (alias, target) in edges {
            text(&mut out, alias.as_bytes());
            num(&mut out, *target);
        }
    }
    num(&mut out, sources.len());
    for (owner, module, path, source) in sources {
        num(&mut out, *owner);
        out.push(u8::from(module.is_some()));
        if let Some(module) = module {
            text(&mut out, module.as_bytes());
        }
        text(&mut out, path.as_bytes());
        text(&mut out, source);
    }
    let length = (out.len() - 16) as u32;
    out[12..16].copy_from_slice(&length.to_le_bytes());
    out
}
fn simple() -> Vec<u8> {
    image(&[vec![]], &[(0, None, "main.svr", b"fn main(){}")])
}
#[test]
fn canonical_package_images_recheck_execute_and_preserve_memory_only_origin() {
    let bytes=image(&[vec![("helpers",1)],vec![]],&[(0,None,"main.svr",b"use helpers::values;fn main(){let x=helpers::values::id::<Int>(7);if x!=7{let bad=1/0;}}"),(1,None,"main.svr",b"fn main(){}"),(1,Some("values"),"src/values.svr",b"mod values {export fn id<T>(x:T)->T{return x;}}")]);
    let program = decode_generic_package_native_image(&bytes).unwrap();
    let inputs = program.bodies().types().declarations().inputs();
    assert!(inputs.is_embedded());
    assert!(inputs.captured().is_none());
    assert!(inputs.manifests().is_empty());
    assert!(inputs.root().starts_with("@sovra-image"));
    assert_eq!(
        program.encode_generic_package_native_image().unwrap(),
        bytes
    );
    assert!(execute_generic_package_native_image(&bytes)
        .unwrap()
        .value()
        .is_unit());
}
#[test]
fn old_and_new_image_families_do_not_silently_fall_back_or_accept_feature_downgrades() {
    let new = simple();
    assert!(is_generic_package_native_image(&new));
    assert!(!is_generic_native_image(&new));
    assert!(decode_generic_native_image(&new).is_err());
    let old = compile_generic_array_source(
        "fn main(){}",
        "main",
        GenericPropagationSourceLimits::DEFAULT,
    )
    .unwrap()
    .encode_generic_native_image()
    .unwrap();
    assert!(!is_generic_package_native_image(&old));
    assert!(decode_generic_package_native_image(&old).is_err());
    for position in [5, 6, 7, 8, 9, 10, 11] {
        let mut bad = new.clone();
        bad[position] ^= 1;
        assert!(
            decode_generic_package_native_image(&bad).is_err(),
            "header {position}"
        );
    }
    for features in [0u32, 1, 3, 7, 15, 31, 63, 127, 255, 1023, u32::MAX] {
        let mut bad = new.clone();
        bad[8..12].copy_from_slice(&features.to_le_bytes());
        assert!(decode_generic_package_native_image(&bad).is_err());
    }
}
#[test]
fn every_truncation_trailing_data_and_extreme_length_refuses() {
    let bytes = simple();
    for length in 0..bytes.len() {
        assert!(
            decode_generic_package_native_image(&bytes[..length]).is_err(),
            "{length}"
        );
    }
    let mut bad = bytes.clone();
    bad.push(0);
    assert!(decode_generic_package_native_image(&bad).is_err());
    let mut bad = bytes.clone();
    bad[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_generic_package_native_image(&bad).is_err());
    let mut bad = bytes;
    bad[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_generic_package_native_image(&bad).is_err());
    assert!(
        decode_generic_package_native_image(&vec![0; GENERIC_PACKAGE_NATIVE_IMAGE_LIMIT + 1])
            .is_err()
    );
}
#[test]
fn malformed_topology_cycles_unreachable_packages_and_alias_duplicates_refuse() {
    for packages in [
        vec![],
        vec![vec![("lib", 1)]],
        vec![vec![("self", 0)]],
        vec![vec![], vec![]],
        vec![vec![("lib", 1)], vec![("back", 0)]],
        vec![vec![("same", 1), ("same", 1)], vec![]],
        vec![vec![("z", 1), ("a", 1)], vec![]],
        vec![vec![("bad-name", 1)], vec![]],
    ] {
        let sources: Vec<_> = (0..packages.len())
            .map(|i| (i, None, "main.svr", b"fn main(){}".as_slice()))
            .collect();
        assert!(decode_generic_package_native_image(&image(&packages, &sources)).is_err());
    }
}
#[test]
fn source_owners_entry_module_order_and_paths_are_strict() {
    for sources in [
        vec![],
        vec![(1, None, "main.svr", b"fn main(){}".as_slice())],
        vec![
            (0, None, "main.svr", b"fn main(){}".as_slice()),
            (0, None, "other.svr", b"fn main(){}".as_slice()),
        ],
        vec![(0, Some("x"), "x.svr", b"mod x{}".as_slice())],
        vec![
            (0, None, "main.svr", b"fn main(){}".as_slice()),
            (0, Some("x"), "main.svr", b"mod x{}".as_slice()),
        ],
    ] {
        assert!(decode_generic_package_native_image(&image(&[vec![]], &sources)).is_err());
    }
    for path in [
        "/absolute.svr",
        "../outside.svr",
        "src/../main.svr",
        "./main.svr",
        "src\\main.svr",
        "C:/main.svr",
        "src//main.svr",
        "main.txt",
        "main\n.svr",
    ] {
        assert!(
            decode_generic_package_native_image(&image(
                &[vec![]],
                &[(0, None, path, b"fn main(){}")]
            ))
            .is_err(),
            "{path:?}"
        );
    }
    assert!(decode_generic_package_native_image(&image(
        &[vec![]],
        &[
            (0, None, "main.svr", b"fn main(){}"),
            (0, Some("x"), "x.svr", b"mod wrong{}")
        ]
    ))
    .is_err());
}
#[test]
fn utf8_source_bytes_and_aggregate_source_admission_are_independent() {
    assert!(decode_generic_package_native_image(&image(
        &[vec![]],
        &[(0, None, "main.svr", &[255])]
    ))
    .is_err());
    let prefix = "fn main(){}//";
    let source = format!(
        "{prefix}{}",
        "x".repeat(GENERIC_PACKAGE_NATIVE_SOURCE_LIMIT - prefix.len())
    );
    assert_eq!(source.len(), GENERIC_PACKAGE_NATIVE_SOURCE_LIMIT);
    let bytes = image(&[vec![]], &[(0, None, "main.svr", source.as_bytes())]);
    assert!(decode_generic_package_native_image(&bytes).is_ok());
    let more = format!("{source}x");
    assert!(decode_generic_package_native_image(&image(
        &[vec![]],
        &[(0, None, "main.svr", more.as_bytes())]
    ))
    .is_err());
}
#[test]
fn semantic_errors_and_runtime_failures_use_embedded_source_locations() {
    assert!(decode_generic_package_native_image(&image(
        &[vec![]],
        &[(0, None, "main.svr", b"fn main()->Int{return 1;}")]
    ))
    .is_err());
    assert!(decode_generic_package_native_image(&image(
        &[vec![]],
        &[(0, None, "main.svr", b"fn main(){}fn unused(){missing();}")]
    ))
    .is_err());
    let bytes = image(
        &[vec![("lib", 1)], vec![]],
        &[
            (
                0,
                None,
                "main.svr",
                b"use lib::ops;fn main(){lib::ops::fail();}",
            ),
            (1, None, "main.svr", b"fn main(){}"),
            (
                1,
                Some("ops"),
                "ops.svr",
                b"mod ops {export fn fail(){let bad=1/0;}}",
            ),
        ],
    );
    let program = decode_generic_package_native_image(&bytes).unwrap();
    let diagnostics = execute_generic_package(&program, GenericVmPolicy::DEFAULT).unwrap_err();
    assert!(
        std::path::Path::new(diagnostics.items[0].source_file.as_ref().unwrap())
            .ends_with(std::path::Path::new("p1").join("ops.svr"))
    );
    let error = execute_generic_package_native_image(&bytes).unwrap_err();
    assert!(error.contains("ops.svr"), "{error}");
    assert!(error.contains("E3125"));
}

#[test]
fn graph_and_source_cardinality_caps_are_exact() {
    let packages: Vec<_> = (0..64)
        .map(|i| {
            if i < 63 {
                vec![("next", i + 1)]
            } else {
                vec![]
            }
        })
        .collect();
    let mut owned = Vec::new();
    for i in 0..64 {
        owned.push((i, None, "main.svr".to_owned(), "fn main(){}".to_owned()));
        for j in 0..3 {
            owned.push((
                i,
                Some(format!("m{j}")),
                format!("m{j}.svr"),
                format!("mod m{j}{{}}"),
            ));
        }
    }
    let as_sources = |owned: &Vec<(usize, Option<String>, String, String)>| {
        image(
            &packages,
            &owned
                .iter()
                .map(|(owner, module, path, text)| {
                    (*owner, module.as_deref(), path.as_str(), text.as_bytes())
                })
                .collect::<Vec<_>>(),
        )
    };
    assert!(decode_generic_package_native_image(&as_sources(&owned)).is_ok());
    owned.push((63, Some("z".into()), "z.svr".into(), "mod z{}".into()));
    assert!(decode_generic_package_native_image(&as_sources(&owned)).is_err());
    let too_many: Vec<_> = (0..65)
        .map(|i| {
            if i < 64 {
                vec![("next", i + 1)]
            } else {
                vec![]
            }
        })
        .collect();
    let sources: Vec<_> = (0..65)
        .map(|i| (i, None, "main.svr", b"fn main(){}".as_slice()))
        .collect();
    assert!(decode_generic_package_native_image(&image(&too_many, &sources)).is_err());
}

#[test]
fn metadata_allowance_and_unknown_source_kinds_refuse_before_compilation() {
    let names: Vec<_> = (0..80)
        .map(|i| {
            let prefix = format!("alias_{i:03}_");
            format!("{prefix}{}", "x".repeat(1024 - prefix.len()))
        })
        .collect();
    let edges = names.iter().map(|name| (name.as_str(), 1)).collect();
    let bytes = image(
        &[edges, vec![]],
        &[
            (0, None, "main.svr", b"fn main(){}"),
            (1, None, "main.svr", b"fn main(){}"),
        ],
    );
    assert!(bytes.len() < GENERIC_PACKAGE_NATIVE_IMAGE_LIMIT);
    assert!(decode_generic_package_native_image(&bytes).is_err());
    let mut bad = simple();
    bad[32] = 2;
    assert!(decode_generic_package_native_image(&bad).is_err());
}

#[test]
fn single_byte_corruption_of_a_small_image_never_panics() {
    let bytes = simple();
    for at in 0..bytes.len() {
        for value in [0, 1, 128, 255] {
            let mut changed = bytes.clone();
            changed[at] = value;
            assert!(
                std::panic::catch_unwind(|| decode_generic_package_native_image(&changed)).is_ok(),
                "byte {at} value {value}"
            );
        }
    }
}

fn collection_image(source: &str) -> Vec<u8> {
    let mut bytes = image(&[vec![]], &[(0, None, "main.svr", source.as_bytes())]);
    bytes[8..12].copy_from_slice(&1023u32.to_le_bytes());
    bytes
}
#[test]
fn collection_images_roundtrip_with_exact_profile_and_memory_only_ownership() {
    let bytes=collection_image("fn main(){let m:std::Map<Int,String>=std::map_new();let full=std::map_insert(m,7,\"λ😀\");let items=std::map_entries(full);let first=items[0];if first.key!=7||first.value!=\"λ😀\"{let bad=1/0;}}");
    let program = decode_generic_collection_package_native_image(&bytes).unwrap();
    assert!(execute_generic_package(&program, GenericVmPolicy::DEFAULT).is_err());
    assert!(program
        .bodies()
        .types()
        .declarations()
        .inputs()
        .is_embedded());
    assert!(program
        .bodies()
        .types()
        .declarations()
        .inputs()
        .captured()
        .is_none());
    assert!(program
        .bodies()
        .types()
        .declarations()
        .inputs()
        .manifests()
        .is_empty());
    assert_eq!(
        program
            .encode_generic_collection_package_native_image()
            .unwrap(),
        bytes
    );
    assert_eq!(
        program.encode_generic_package_native_image().unwrap_err(),
        "native collection feature is not enabled"
    );
    assert!(execute_generic_collection_package_native_image(&bytes)
        .unwrap()
        .value()
        .is_unit());
    assert!(decode_generic_package_native_image(&bytes).is_err());
    assert!(execute_generic_package_native_image(&bytes).is_err());
    assert!(decode_generic_native_image(&bytes).is_err());
}
#[test]
fn collection_examples_execute_after_embedded_rechecking() {
    for name in [
        "insertion",
        "lookup",
        "removal",
        "iteration",
        "entries",
        "display",
    ] {
        let source =
            std::fs::read_to_string(format!("docs/examples/intrinsic-collection-{name}.svr"))
                .unwrap();
        let source = source
            .replace("fn main()->Bool", "fn verify()->Bool")
            .replace("fn main() -> Bool", "fn verify()->Bool");
        assert!(source.contains("fn verify()->Bool"), "{name}");
        let source = format!("{source}\nfn main(){{if (verify()==false){{let bad=1/0;}}}}");
        let bytes = collection_image(&source);
        assert!(
            execute_generic_collection_package_native_image(&bytes)
                .unwrap()
                .value()
                .is_unit(),
            "{name}"
        );
        assert_eq!(
            decode_generic_collection_package_native_image(&bytes)
                .unwrap()
                .encode_generic_collection_package_native_image()
                .unwrap(),
            bytes
        );
    }
}
#[test]
fn collection_package_modules_and_generic_imports_keep_original_owner_boundaries() {
    let mut bytes=image(&[vec![("helpers",1)],vec![]],&[
        (0,None,"main.svr",b"use helpers::maps;fn main(){let m=helpers::maps::make();let pairs=helpers::maps::entries::<Int,String>(m);let first=pairs[0];if first.key!=7||first.value!=\"value\"{let bad=1/0;}if std::to_string(m)!=\"std::Map { 7: value }\"{let bad=1/0;}}"),
        (1,None,"main.svr",b"fn main(){}"),
        (1,Some("maps"),"src/λ.svr",b"mod maps{export fn make()->std::Map<Int,String>{let m:std::Map<Int,String>=std::map_new();return std::map_insert(m,7,\"value\");}export fn entries<K:std::Key,V>(m:std::Map<K,V>)->[std::MapEntry<K,V>]{return std::map_entries(m);}}"),
    ]);
    bytes[8..12].copy_from_slice(&1023u32.to_le_bytes());
    assert!(execute_generic_collection_package_native_image(&bytes).is_ok());
    let program = decode_generic_collection_package_native_image(&bytes).unwrap();
    assert_eq!(
        program
            .encode_generic_collection_package_native_image()
            .unwrap(),
        bytes
    );
}
#[test]
fn collection_feature_words_are_exact_and_source_downgrades_do_not_bypass_rechecking() {
    let bytes = collection_image("fn main(){let m:std::Map<Int,Int>=std::map_new();}");
    for features in [
        0u32,
        1,
        3,
        7,
        15,
        31,
        63,
        127,
        255,
        511,
        512,
        1022,
        2047,
        u32::MAX,
    ] {
        let mut bad = bytes.clone();
        bad[8..12].copy_from_slice(&features.to_le_bytes());
        assert!(
            decode_generic_collection_package_native_image(&bad).is_err(),
            "{features}"
        );
        assert!(execute_generic_collection_package_native_image(&bad).is_err());
        assert!(decode_generic_package_native_image(&bad).is_err());
    }
    assert!(decode_generic_collection_package_native_image(&simple()).is_err());
    let promoted = collection_image("fn main(){}");
    let program = decode_generic_collection_package_native_image(&promoted).unwrap();
    assert!(program.encode_generic_package_native_image().is_err());
    assert!(execute_generic_package(&program, GenericVmPolicy::DEFAULT).is_err());
    assert!(execute_generic_collection_package_native_image(&promoted).is_ok());
}
#[test]
fn collection_image_truncations_trailing_bytes_and_corrupt_utf8_refuse() {
    let bytes = collection_image("fn main(){}");
    for n in 0..bytes.len() {
        assert!(
            decode_generic_collection_package_native_image(&bytes[..n]).is_err(),
            "{n}"
        );
    }
    let mut bad = bytes.clone();
    bad.push(0);
    assert!(decode_generic_collection_package_native_image(&bad).is_err());
    let length = (bad.len() - 16) as u32;
    bad[12..16].copy_from_slice(&length.to_le_bytes());
    assert!(decode_generic_collection_package_native_image(&bad).is_err());
    let mut bad = bytes.clone();
    *bad.last_mut().unwrap() = 255;
    assert!(decode_generic_collection_package_native_image(&bad).is_err());
    let mut bad = bytes;
    bad[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_generic_collection_package_native_image(&bad).is_err());
}
#[test]
fn collection_image_validation_checks_all_bodies_key_constraints_and_command_entry() {
    for source in [
        "fn main()->Int{return 7;}",
        "fn main(value:Int){}",
        "fn unused()->std::Option<Float>{let m:std::Map<Int,Int>=std::map_new();return std::map_get(m,1);}fn main(){}",
        "fn unused(value:std::Map<Float,Int>){}fn main(){}",
        "fn main(){let m:std::Map<Int,Int>=std::map_new();let bad=std::set_values(m);}",
        "fn main(){std::read_text(\"not-authorized\");}",
    ]{assert!(decode_generic_collection_package_native_image(&collection_image(source)).is_err(),"{source}");}
    let bytes=collection_image("fn main(){let m:std::Map<Int,Int>=std::map_new();if std::map_contains(m,1)==false{let bad=1/0;}}");
    assert!(decode_generic_collection_package_native_image(&bytes).is_ok());
    assert!(execute_generic_collection_package_native_image(&bytes).is_err());
}
#[test]
fn collection_image_size_and_runtime_budgets_have_no_unlimited_fallback() {
    let source = " ".repeat(GENERIC_PACKAGE_NATIVE_SOURCE_LIMIT + 1);
    assert!(decode_generic_collection_package_native_image(&collection_image(&source)).is_err());
    let bytes=collection_image("fn main(){let m:std::Map<Int,Int>=std::map_new();let mut i=0;while i<1000000{let text=std::to_string(m);i=i+1;}}");
    assert!(decode_generic_collection_package_native_image(&bytes).is_ok());
    assert!(execute_generic_collection_package_native_image(&bytes).is_err());
}
#[test]
fn single_byte_collection_image_corruption_never_panics() {
    let bytes = collection_image("fn main(){}");
    for at in 0..bytes.len() {
        for value in [0, 1, 128, 255] {
            let mut changed = bytes.clone();
            changed[at] = value;
            assert!(
                std::panic::catch_unwind(|| decode_generic_collection_package_native_image(
                    &changed
                ))
                .is_ok(),
                "byte {at} value {value}"
            );
        }
    }
}
