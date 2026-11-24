//! Compiler-owned Byte/Bytes annotations, bounds, profile isolation and data execution.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::{project::packages::*, v2::*};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture(PathBuf);
impl Fixture {
    fn new(source: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "sovra data types λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::create_dir_all(root.join("dep")).unwrap();
        let f = Self(root);
        f.write(
            "app/sovra.toml",
            "[project]\nname=\"app\"\nentry=\"main.svr\"\n[dependencies.dep]\npath=\"../dep\"\n",
        );
        f.write(
            "dep/sovra.toml",
            "[project]\nname=\"dep\"\nentry=\"main.svr\"\n[modules]\nops=\"ops.svr\"\n",
        );
        f.write("dep/main.svr", "fn main(){}");
        f.write("dep/ops.svr","mod ops{export struct Box<T>{value:T}export fn id<T>(value:T)->T{return value;}export fn key<K:std::Key>(value:K)->K{return value;}}");
        f.write("app/main.svr", source);
        f
    }
    fn write(&self, path: &str, text: &str) {
        std::fs::write(self.0.join(path), text).unwrap();
    }
    fn table(&self) -> GenericPackageDeclarations {
        bind_generic_package_declarations(
            capture_package_sources(self.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            GenericPackageDeclarationLimits::DEFAULT,
        )
        .unwrap()
    }
    fn types(&self) -> CheckedGenericPackageTypes {
        check_generic_data_package_types(self.table(), GenericPackageTypeLimits::DEFAULT).unwrap()
    }
    fn bodies(&self) -> CheckedGenericPackageBodies {
        check_generic_collection_package_bodies(
            self.types(),
            GenericCopyBodyLimits::PACKAGE,
            GenericPropagationLimits::DEFAULT,
        )
        .unwrap()
    }
    fn literals(&self) -> Result<CheckedGenericLiterals, GenericConcreteBodyError> {
        let bodies = self.bodies();
        let root = bodies
            .types()
            .signatures()
            .signatures()
            .iter()
            .find(|s| s.syntax().name() == "main")
            .unwrap()
            .id();
        let graph = discover_generic_propagation_specializations(
            bodies.bodies(),
            root,
            GenericSpecializationLimits::DEFAULT,
        )
        .unwrap();
        let concrete = recheck_generic_propagation_concrete_bodies(
            &graph,
            GenericConcreteBodyLimits::DEFAULT,
            GenericPropagationLimits::DEFAULT,
        )?;
        Ok(decode_generic_literals(&concrete, GenericLiteralLimits::DEFAULT).unwrap())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn annotations_are_sealed_opaque_zero_argument_types_with_distinct_ids() {
    let f=Fixture::new("use dep::ops;type B<T>=std::Byte;type Buffer<T>=std::Bytes;struct Out<T>{byte:T,buffer:std::Bytes,items:[std::Byte],keyed:std::Map<B<Int>,dep::ops::Box<Buffer<Int>>>}fn main(x:Out<Byte>){}struct Byte<T>{value:T}");
    // A source homonym is generic and cannot replace the qualified standard Byte.
    assert!(
        check_generic_data_package_types(f.table(), GenericPackageTypeLimits::DEFAULT).is_err()
    );
    f.write("app/main.svr","use dep::ops;type B<T>=std::Byte;type Buffer<T>=std::Bytes;struct Out<T>{byte:T,buffer:std::Bytes,items:[std::Byte],keyed:std::Map<B<Int>,dep::ops::Box<Buffer<Int>>>}struct Byte<T>{value:T}fn main(x:Out<std::Byte>){}");
    let types = f.types();
    let data = types
        .types()
        .definitions()
        .iter()
        .filter_map(|d| {
            if let CheckedGenericDefinition::Data(d) = d {
                Some(d)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(data.len(), 2);
    assert_ne!(data[0].id(), data[1].id());
    for d in data {
        assert!(d.syntax().parameters().parameters().is_empty());
        assert!(d.environment().registry().header(d.id()).is_some());
        assert!(matches!(
            d.syntax().qualified_name(),
            "std::Byte" | "std::Bytes"
        ));
    }
    f.bodies();
}
#[test]
fn byte_is_a_key_through_aliases_and_generic_calls_but_bytes_and_homonyms_are_not() {
    let f=Fixture::new("use dep::ops;type B<T>=std::Byte;fn main(x:B<Int>)->std::Byte{return dep::ops::key::<B<Int>>(dep::ops::id::<B<Int>>(x));}");
    assert!(f.literals().is_ok());
    for key in ["std::Bytes", "Float", "[std::Byte]", "Byte<Int>"] {
        f.write(
            "app/main.svr",
            &format!("struct Byte<T>{{value:T}}fn main(x:std::Map<{key},Int>){{}}"),
        );
        assert!(
            check_generic_data_package_types(f.table(), GenericPackageTypeLimits::DEFAULT).is_err(),
            "{key}"
        );
    }
    f.write(
        "app/main.svr",
        "use dep::ops;fn main(x:std::Bytes)->std::Bytes{return dep::ops::key::<std::Bytes>(x);}",
    );
    let types = f.types();
    assert!(check_generic_collection_package_bodies(
        types,
        GenericCopyBodyLimits::PACKAGE,
        GenericPropagationLimits::DEFAULT
    )
    .is_err());
}
#[test]
fn scalar_buffer_aliases_and_nominal_phantom_arguments_do_not_mix() {
    for source in [
        "fn main(x:std::Byte)->Int{return x;}",
        "fn main(x:Int)->std::Byte{return x;}",
        "fn main(x:std::Bytes)->[std::Byte]{return x;}",
        "fn main(x:[std::Byte])->std::Bytes{return x;}",
        "struct Empty<T>{}fn main(x:Empty<std::Byte>)->Empty<std::Bytes>{return x;}",
    ] {
        let f = Fixture::new(source);
        assert!(
            check_generic_collection_package_bodies(
                f.types(),
                GenericCopyBodyLimits::PACKAGE,
                GenericPropagationLimits::DEFAULT
            )
            .is_err(),
            "{source}"
        );
    }
}
#[test]
fn opaque_data_cannot_be_forged_with_record_construction_or_field_projection() {
    for source in [
        "fn main()->std::Byte{return std::Byte{};}",
        "fn main()->std::Bytes{return std::Bytes{};}",
        "fn main(x:std::Byte)->Int{return x.value;}",
        "fn main(x:std::Bytes)->Int{return x.length;}",
    ] {
        let f = Fixture::new(source);
        assert!(
            check_generic_collection_package_bodies(
                f.types(),
                GenericCopyBodyLimits::PACKAGE,
                GenericPropagationLimits::DEFAULT
            )
            .is_err(),
            "{source}"
        );
    }
    for ty in ["std::Byte<Int>", "std::Bytes<std::Byte>"] {
        let f = Fixture::new(&format!("fn main(x:{ty}){{}}"));
        assert!(
            check_generic_data_package_types(f.table(), GenericPackageTypeLimits::DEFAULT).is_err()
        );
    }
}
#[test]
fn imported_generic_byte_forwarding_rechecks_then_executable_layouts_explicitly_refuse() {
    for source in [
        "use dep::ops;fn main(x:std::Byte)->std::Byte{return dep::ops::key::<std::Byte>(x);}",
        "use dep::ops;fn main(x:std::Bytes)->std::Bytes{return dep::ops::id::<std::Bytes>(x);}",
        "fn main(x:std::Map<std::Byte,std::Bytes>){}",
        "fn main(x:std::MapEntry<std::Byte,std::Bytes>)->std::Bytes{return x.value;}",
        "fn main(x:std::Byte,y:std::Bytes)->std::MapEntry<std::Byte,std::Bytes>{return std::MapEntry::<std::Byte,std::Bytes>{key:x,value:y};}",
        "struct Empty<T>{}fn main(x:Empty<std::Byte>){}",
    ] {
        let f = Fixture::new(source);
        let literals = f.literals().unwrap();
        assert!(matches!(
            plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT),
            Err(GenericLayoutError::Type(
                "intrinsic data execution is not enabled"
                    | "intrinsic collection execution is not enabled"
            ))
        ));
        let result = plan_generic_collection_layouts(&literals, GenericLayoutLimits::DEFAULT);
        assert!(
            matches!(
                result,
                Err(GenericLayoutError::Type(
                    "intrinsic data execution is not enabled"
                ))
            ),
            "{source}: {result:?}"
        );
    }
}
#[test]
fn old_semantic_compile_and_native_profiles_do_not_inject_data_types() {
    let f = Fixture::new("fn unused(value:std::Byte){}fn main(){}");
    f.types();
    assert!(check_generic_package_types(f.table(), GenericPackageTypeLimits::DEFAULT).is_err());
    assert!(
        check_generic_collection_package_types(f.table(), GenericPackageTypeLimits::DEFAULT)
            .is_err()
    );
    for compile in [compile_generic_package, compile_generic_collection_package] {
        assert!(compile(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT
        )
        .is_err());
    }
}
#[test]
fn two_data_headers_consume_exact_cumulative_declaration_and_scope_allowances() {
    let f = Fixture::new("fn main(){}");
    let old = check_generic_collection_package_types(f.table(), GenericPackageTypeLimits::DEFAULT)
        .unwrap();
    let n = old.types().environment().registry().headers().len();
    let types = f.types();
    assert_eq!(
        types.types().environment().registry().headers().len(),
        n + 3
    );
    let mut limits = GenericPackageTypeLimits::DEFAULT;
    limits.registry.max_declarations = n + 3;
    assert!(check_generic_data_package_types(f.table(), limits).is_ok());
    limits.registry.max_declarations = n + 2;
    assert!(check_generic_data_package_types(f.table(), limits).is_err());
    assert!(check_generic_collection_package_types(f.table(), limits).is_ok());
    let mut limits = GenericPackageTypeLimits::DEFAULT;
    limits.max_scopes = n + old.signatures().signatures().len() + 3;
    assert!(check_generic_data_package_types(f.table(), limits).is_ok());
    limits.max_scopes -= 1;
    assert!(check_generic_data_package_types(f.table(), limits).is_err());
    assert!(check_generic_collection_package_types(f.table(), limits).is_ok());
}
#[test]
fn fresh_data_definitions_cannot_attach_to_another_headers_empty_parameter_scope() {
    let data = standard_generic_data_types().unwrap();
    let inputs = data
        .iter()
        .map(GenericTypeDeclaration::Data)
        .collect::<Vec<_>>();
    let registry = register_generic_types(&inputs, GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    check_generic_declarations(&env, &inputs, GenericDeclarationLimits::DEFAULT).unwrap();
    let foreign = standard_generic_data_types().unwrap();
    let inputs = foreign
        .iter()
        .map(GenericTypeDeclaration::Data)
        .collect::<Vec<_>>();
    assert!(matches!(
        check_generic_declarations(&env, &inputs, GenericDeclarationLimits::DEFAULT),
        Err(GenericDeclarationError::Ownership { .. })
    ));
}

#[test]
fn data_layouts_keep_byte_bytes_int_array_and_nominal_identities_distinct() {
    let f=Fixture::new("struct Byte<T>{value:T}fn main(a:std::Byte,b:std::Bytes,c:[std::Byte],d:Int,e:Byte<Int>){}");
    let literals = f.literals().unwrap();
    let layouts = plan_generic_data_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap();
    assert!(layouts.has_data());
    assert!(!layouts.has_collections());
    let all = layouts.layouts();
    let byte = all
        .iter()
        .position(|l| matches!(l.kind(), GenericRuntimeLayoutKind::Byte { .. }))
        .unwrap();
    let buffer = all
        .iter()
        .position(|l| matches!(l.kind(), GenericRuntimeLayoutKind::Bytes { .. }))
        .unwrap();
    let int = all
        .iter()
        .position(|l| {
            matches!(
                l.kind(),
                GenericRuntimeLayoutKind::Primitive(GenericPrimitive::Int)
            )
        })
        .unwrap();
    let array = all
        .iter()
        .position(|l| matches!(l.kind(), GenericRuntimeLayoutKind::Array { .. }))
        .unwrap();
    let record = all
        .iter()
        .position(|l| matches!(l.kind(), GenericRuntimeLayoutKind::Record { .. }))
        .unwrap();
    assert!(matches!(all[buffer].kind(),GenericRuntimeLayoutKind::Bytes{byte:b,..} if *b==byte));
    assert!(matches!(all[array].kind(),GenericRuntimeLayoutKind::Array{element} if *element==byte));
    assert_ne!(byte, int);
    assert_ne!(buffer, array);
    let byte = GenericCollectionType::from_layout(&layouts, byte).unwrap();
    let buffer = GenericCollectionType::from_layout(&layouts, buffer).unwrap();
    let int = GenericCollectionType::from_layout(&layouts, int).unwrap();
    let record = GenericCollectionType::from_layout(&layouts, record).unwrap();
    assert!(byte.is_key());
    assert!(!buffer.is_key());
    assert!(!record.is_key());
    assert_ne!(byte.declaration(), buffer.declaration());
    assert!(byte.declaration().is_some());
    assert!(!byte
        .equivalent(&int, GenericTypeEquivalenceLimits::DEFAULT)
        .unwrap());
    assert!(!byte
        .equivalent(&record, GenericTypeEquivalenceLimits::DEFAULT)
        .unwrap());
}
#[test]
fn buffer_only_annotations_discover_exact_sealed_byte_edge_and_intern_aliases() {
    for source in ["fn main(value:std::Bytes){}","type B<T>=std::Byte;type Buffer<T>=std::Bytes;fn main(a:std::Bytes,b:Buffer<Int>,c:std::Byte,d:B<String>){}"]{
  let f=Fixture::new(source);let layouts=plan_generic_data_layouts(&f.literals().unwrap(),GenericLayoutLimits::DEFAULT).unwrap();let all=layouts.layouts();let bytes=all.iter().filter(|l|matches!(l.kind(),GenericRuntimeLayoutKind::Byte{..})).collect::<Vec<_>>();assert_eq!(bytes.len(),1);assert_eq!(all.iter().filter(|l|matches!(l.kind(),GenericRuntimeLayoutKind::Bytes{..})).count(),1);
  let GenericRuntimeLayoutKind::Byte{declaration}=bytes[0].kind() else{unreachable!()};let Some(CheckedGenericDefinition::Data(def))=layouts.literals().concrete().graph().bodies().signatures().types().definition(*declaration) else{panic!("missing data proof")};assert_eq!(def.syntax().kind(),GenericDataTypeKind::Byte);assert_eq!(def.syntax().qualified_name(),"std::Byte");
 }
}
#[test]
fn recursive_map_byte_graphs_close_without_erasing_data_edges() {
    let f=Fixture::new("struct Node<T>{buffer:std::Bytes,next:std::Map<std::Byte,Node<T>>,value:T}fn main(value:Node<std::Byte>){}");
    let layouts =
        plan_generic_data_layouts(&f.literals().unwrap(), GenericLayoutLimits::DEFAULT).unwrap();
    assert!(layouts.has_data());
    assert!(layouts.has_collections());
    assert!(layouts.layouts().len() < 16);
    let all = layouts.layouts();
    let root = all
        .iter()
        .position(|l| matches!(l.kind(), GenericRuntimeLayoutKind::Record { .. }))
        .unwrap();
    let GenericRuntimeLayoutKind::Record { fields, .. } = all[root].kind() else {
        unreachable!()
    };
    assert!(matches!(
        all[fields[0]].kind(),
        GenericRuntimeLayoutKind::Bytes { .. }
    ));
    let GenericRuntimeLayoutKind::Map { key, value, .. } = all[fields[1]].kind() else {
        panic!("missing map")
    };
    assert_eq!(*value, root);
    assert!(matches!(
        all[*key].kind(),
        GenericRuntimeLayoutKind::Byte { .. }
    ));
    assert_eq!(*key, fields[2]);
    assert!(matches!(
        GenericCollectionLayout::from_checked_layout(
            &layouts,
            fields[1],
            GenericTypeEquivalenceLimits::DEFAULT
        ),
        Err(GenericCollectionLayoutError::DataOperationsUnavailable)
    ));
}
#[test]
fn data_and_phantom_layouts_cannot_escape_existing_construction_or_ir_gates() {
    for source in [
        "fn main(value:std::Bytes){}",
        "struct Empty<T>{}fn main(value:Empty<std::Byte>){}",
        "fn main(){let value=std::Option::<std::Bytes>::None();}",
        "fn main(value:std::Map<std::Byte,std::Bytes>){}",
    ] {
        let f = Fixture::new(source);
        let literals = f.literals().unwrap();
        assert!(plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT).is_err());
        assert!(plan_generic_collection_layouts(&literals, GenericLayoutLimits::DEFAULT).is_err());
        let layouts = plan_generic_data_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap();
        assert!(layouts.has_data());
        for result in [
            plan_generic_construction(&layouts, GenericConstructionLimits::DEFAULT),
            plan_generic_collection_construction(&layouts, GenericConstructionLimits::DEFAULT),
        ] {
            assert!(
                matches!(
                    result,
                    Err(GenericConstructionError::DataOperationsUnavailable)
                ),
                "{source}"
            );
        }
        let operations =
            plan_generic_propagation_layouts(&layouts, GenericPropagationLayoutLimits::DEFAULT)
                .unwrap();
        assert!(matches!(
            lower_generic_collection_propagation_ir(
                &operations,
                GenericConstructionLimits::DEFAULT,
                GenericIrLimits::DEFAULT
            ),
            Err(GenericIrError::Construction(
                GenericConstructionError::DataOperationsUnavailable
            ))
        ));
    }
}
#[test]
fn exact_data_layout_budget_counters_pass_and_each_one_under_refuses() {
    let f=Fixture::new("struct Wrap<T>{value:T}fn main(a:std::Bytes,b:[std::Byte],c:std::Map<std::Byte,Wrap<std::Bytes>>){}");
    let literals = f.literals().unwrap();
    let layouts = plan_generic_data_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap();
    let u = layouts.usage();
    assert!(u.generated_nodes >= 2);
    let mut exact = GenericLayoutLimits::DEFAULT;
    exact.max_instantiations = u.instantiations;
    exact.max_layouts = u.layouts;
    exact.max_members = u.members;
    exact.max_edges = u.edges;
    exact.max_generated_nodes = u.generated_nodes;
    exact.max_generated_edges = u.generated_edges;
    exact.max_key_nodes = u.key_nodes;
    exact.max_key_edges = u.key_edges;
    exact.max_work = u.work;
    assert!(plan_generic_data_layouts(&literals, exact).is_ok());
    for dimension in 0..9 {
        let mut small = exact;
        let counter = match dimension {
            0 => &mut small.max_instantiations,
            1 => &mut small.max_layouts,
            2 => &mut small.max_members,
            3 => &mut small.max_edges,
            4 => &mut small.max_generated_nodes,
            5 => &mut small.max_generated_edges,
            6 => &mut small.max_key_nodes,
            7 => &mut small.max_key_edges,
            _ => &mut small.max_work,
        };
        if *counter > 0 {
            *counter -= 1;
            assert!(
                plan_generic_data_layouts(&literals, small).is_err(),
                "dimension {dimension}"
            );
        }
    }
}
#[test]
fn data_free_graphs_keep_original_layout_usage_and_vm_behavior() {
    let f = Fixture::new("fn main()->Int{return 7;}");
    let literals = f.literals().unwrap();
    let old = plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap();
    let data = plan_generic_data_layouts(&literals, GenericLayoutLimits::DEFAULT).unwrap();
    assert!(!data.has_data());
    assert!(!data.has_collections());
    assert_eq!(old.usage(), data.usage());
    let construction =
        plan_generic_construction(&data, GenericConstructionLimits::DEFAULT).unwrap();
    let ir = lower_generic_ir(&construction, GenericIrLimits::DEFAULT).unwrap();
    assert_eq!(
        execute_generic_ir(&ir, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .as_int(),
        Some(7)
    );
}

#[test]
fn bytes_layout_does_not_guess_a_missing_byte_schema_from_user_record_names() {
    let data = standard_generic_data_types().unwrap();
    let bytes = data
        .iter()
        .find(|d| d.kind() == GenericDataTypeKind::Bytes)
        .unwrap();
    let user =
        parse_generic_record("struct Byte<T>{value:T}", GenericRecordLimits::DEFAULT).unwrap();
    let inputs = [
        GenericTypeDeclaration::Data(bytes),
        GenericTypeDeclaration::Record(&user),
    ];
    let registry = register_generic_types(&inputs, GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    let types =
        check_generic_declarations(&env, &inputs, GenericDeclarationLimits::DEFAULT).unwrap();
    let root = parse_specialization_root(
        "fn root(value:std::Bytes,fake:Byte<Int>){}",
        GenericFunctionLimits::DEFAULT,
    )
    .unwrap();
    let signatures =
        check_generic_signatures(&types, &[&root], GenericSignatureLimits::DEFAULT).unwrap();
    let bodies = check_generic_copy_bodies(&signatures, GenericCopyBodyLimits::DEFAULT).unwrap();
    let graph = discover_generic_specializations(
        &bodies,
        signatures.find("root").unwrap(),
        GenericSpecializationLimits::DEFAULT,
    )
    .unwrap();
    let concrete =
        recheck_generic_concrete_bodies(&graph, GenericConcreteBodyLimits::DEFAULT).unwrap();
    let literals = decode_generic_literals(&concrete, GenericLiteralLimits::DEFAULT).unwrap();
    assert!(matches!(
        plan_generic_data_layouts(&literals, GenericLayoutLimits::DEFAULT),
        Err(GenericLayoutError::Type("Bytes element schema unavailable"))
    ));
}

fn compile_byte_source(source: &str) -> CompiledGenericPackage {
    let f = Fixture::new(source);
    compile_generic_data_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap()
}
#[test]
fn byte_source_executes_all_values_and_owned_range_errors() {
    for n in 0..=256 {
        let p = compile_byte_source(&format!(
            "fn main()->std::Result<std::Byte,std::Error>{{return std::byte_from_int({n});}}"
        ));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        let value = out.value();
        if n <= 255 {
            assert_eq!(value.enum_tag(), Some(0));
            assert_eq!(value.payload().unwrap().as_byte(), Some(n as u8));
            assert_eq!(value.payload().unwrap().as_int(), None);
            assert_eq!(value.footprint(), (2, 13));
        } else {
            assert_eq!(value.enum_tag(), Some(1));
            let e = value.payload().unwrap();
            assert_eq!(e.field(0).unwrap().as_str(), Some("byte_range"));
            assert_eq!(
                e.field(1).unwrap().as_str(),
                Some("byte value must be in 0..255")
            );
            assert_eq!(e.field(2).unwrap().enum_tag(), Some(1));
            assert_eq!(value.footprint(), (5, 94));
        }
    }
}
#[test]
fn byte_source_roundtrip_and_profile_refusal() {
    let p = compile_byte_source("fn main()->std::Result<Int,std::Error>{let b=std::byte_from_int(255)?;return std::Result::<Int,std::Error>::Ok(std::byte_to_int(b));}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().payload().unwrap().as_int(), Some(255));
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).is_err());
}
#[test]
fn byte_conversion_profiles_and_materialization_are_explicit() {
    for (n, nodes, text) in [(0, 2, 13), (256, 5, 94), (i64::MAX, 5, 94)] {
        let p = compile_byte_source(&format!(
            "fn main()->std::Result<std::Byte,std::Error>{{return std::byte_from_int({n});}}"
        ));
        // The argument literal also consumes one materialized node.
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.runtime.materialization.max_value_nodes = Some(nodes + 1);
        policy.runtime.materialization.max_text_bytes = Some(text);
        assert!(execute_generic_data_package(&p, policy.clone()).is_ok());
        policy.runtime.materialization.max_value_nodes = Some(nodes);
        assert!(execute_generic_data_package(&p, policy.clone()).is_err());
        policy.runtime.materialization.max_value_nodes = Some(nodes + 1);
        policy.runtime.materialization.max_text_bytes = Some(text - 1);
        assert!(execute_generic_data_package(&p, policy.clone()).is_err());
        assert!(p.encode_generic_package_native_image().is_err());
        assert!(p.encode_generic_collection_package_native_image().is_err());
    }
}
#[test]
fn byte_conversions_reject_implicit_types_and_invalid_buffer_operands() {
    for source in [
        "fn main()->Int{return std::byte_to_int(1);}",
        "fn main()->std::Byte{return 1;}",
        "fn main()->std::Result<std::Byte,std::Error>{return std::byte_from_int(true);}",
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_array([1,2]);}",
    ] {
        let f = Fixture::new(source);
        assert!(
            compile_generic_data_package(
                capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
                "main",
                GenericPackageCompilationLimits::DEFAULT
            )
            .is_err(),
            "{source}"
        );
    }
}
#[test]
fn byte_keys_execute_with_generic_forwarding_and_collection_snapshots() {
    let p = compile_byte_source("use dep::ops;fn main()->std::Result<Int,std::Error>{let byte=dep::ops::key::<std::Byte>(std::byte_from_int(7)?);let map:std::Map<std::Byte,Int>=std::map_new();let map=std::map_insert(map,byte,11);let map=std::map_insert(map,byte,12);let keys=std::map_keys(map);return std::Result::<Int,std::Error>::Ok(std::byte_to_int(keys[0]));}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().payload().unwrap().as_int(), Some(7));
}
#[test]
fn negative_bytes_errors_propagate_and_data_free_profiles_stay_explicit() {
    let p = compile_byte_source("fn main()->std::Result<Int,std::Error>{let b=std::byte_from_int(0-1)?;return std::Result::<Int,std::Error>::Ok(std::byte_to_int(b));}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    assert_eq!(
        out.value().payload().unwrap().field(0).unwrap().as_str(),
        Some("byte_range")
    );
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_work = 0;
    assert!(execute_generic_data_package(&p, policy).is_err());
    let p = compile_byte_source("fn main(){}");
    assert!(execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert_eq!(
        p.encode_generic_collection_package_native_image()
            .unwrap_err(),
        "native data feature is not enabled"
    );
}
#[test]
fn byte_arrays_sets_equality_and_display_preserve_scalar_identity() {
    let p = compile_byte_source("fn main()->std::Result<Bool,std::Error>{let a=std::byte_from_int(9)?;let b=std::byte_from_int(9)?;let values:[std::Byte]=[a,b];let set:std::Set<std::Byte>=std::set_new();let set=std::set_insert(std::set_insert(set,a),b);return std::Result::<Bool,std::Error>::Ok(values[0]==values[1]&&std::set_len(set)==1&&std::set_contains(set,b)&&std::to_string(a)==\"9\");}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().payload().unwrap().as_bool(), Some(true));
}

#[test]
fn bytes_source_utf8_is_exact_and_distinct_from_arrays() {
    for text in ["", "ascii", "λ🦀", "é"] {
        let p = compile_byte_source(&format!(
            "fn main()->std::Result<std::Bytes,std::Error>{{return std::bytes_from_text(\"{text}\");}}"
        ));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        let value = out.value().payload().unwrap();
        assert_eq!(value.bytes_len(), Some(text.len()));
        assert_eq!(value.array_len(), None);
        assert_eq!(value.footprint(), (text.len() + 1, 0));
        for (i, byte) in text.bytes().enumerate() {
            assert_eq!(value.byte_item(i).unwrap().as_byte(), Some(byte));
        }
        assert!(value.byte_item(text.len()).is_none());
    }
}
#[test]
fn bytes_source_array_copy_is_independent_and_generic() {
    let p = compile_byte_source("use dep::ops;fn main()->std::Result<std::Bytes,std::Error>{let a=std::byte_from_int(0)?;let b=std::byte_from_int(255)?;let mut items:[std::Byte]=[a,b];let bytes=dep::ops::id::<std::Bytes>(std::bytes_from_array(items)?);items[0]=b;return std::Result::<std::Bytes,std::Error>::Ok(bytes);}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    let bytes = out.value().payload().unwrap();
    assert_eq!(bytes.bytes_len(), Some(2));
    assert_eq!(bytes.byte_item(0).unwrap().as_byte(), Some(0));
    assert_eq!(bytes.byte_item(1).unwrap().as_byte(), Some(255));
    assert_eq!(bytes.footprint(), (3, 0));
}
#[test]
fn bytes_source_len_equality_display_and_empty_nominal_payloads() {
    let p=compile_byte_source("fn main()->std::Result<Bool,std::Error>{let a=std::bytes_from_text(\"λ\")?;let b=std::bytes_from_text(\"λ\")?;let c=std::bytes_from_text(\"x\")?;let empty:std::Option<std::Bytes>=std::Option::<std::Bytes>::None();return std::Result::<Bool,std::Error>::Ok(a==b&&a!=c&&std::bytes_len(a)==2&&std::to_string(a)==\"std::Bytes [206, 187]\");}");
    assert_eq!(
        execute_generic_data_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .payload()
            .unwrap()
            .as_bool(),
        Some(true)
    );
}
#[test]
fn bytes_source_whole_output_costs_and_old_profiles_refuse() {
    let p = compile_byte_source(
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_text(\"λ\");}",
    );
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.runtime.materialization.max_value_nodes = Some(5); // String + Result + buffer + two Bytes
    policy.runtime.materialization.max_text_bytes = Some(15); // source String and Result labels
    assert!(execute_generic_data_package(&p, policy.clone()).is_ok());
    policy.runtime.materialization.max_value_nodes = Some(4);
    assert!(execute_generic_data_package(&p, policy.clone()).is_err());
    policy.runtime.materialization.max_value_nodes = Some(5);
    policy.runtime.materialization.max_text_bytes = Some(14);
    assert!(execute_generic_data_package(&p, policy).is_err());
    assert!(execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(p.encode_generic_package_native_image().is_err());
    assert!(p.encode_generic_collection_package_native_image().is_err());
}
#[test]
fn bytes_snapshots_keep_full_costs_and_checked_byte_selection() {
    use sovra::compiler::execution::MaterializationLimits;
    let p = compile_byte_source(
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_text(\"ab\");}",
    );
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    let mut fuel = RuntimeValueFuel::new(MaterializationLimits {
        max_value_nodes: Some(3),
        max_text_bytes: Some(0),
    });
    let mut work = RuntimeSetWork::new(10000);
    let snapshot = out
        .collection_snapshot(&[GenericCollectionSelector::Payload], &mut fuel, &mut work)
        .unwrap();
    assert_eq!(snapshot.value().footprint(), (3, 0));
    assert_eq!(fuel.remaining().max_value_nodes, Some(0));
    let mut short = RuntimeValueFuel::new(MaterializationLimits {
        max_value_nodes: Some(2),
        max_text_bytes: Some(0),
    });
    assert!(snapshot.snapshot(&mut short, &mut work).is_err());
    assert_eq!(short.remaining().max_value_nodes, Some(2));
    let mut fuel = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
    let copy = snapshot.snapshot(&mut fuel, &mut work).unwrap();
    assert!(snapshot
        .equal(&copy, GenericTypeEquivalenceLimits::DEFAULT, &mut work)
        .unwrap());
    let item = copy
        .select(&[GenericCollectionSelector::Item(1)], &mut fuel, &mut work)
        .unwrap();
    assert_eq!(item.value().as_byte(), Some(b'b'));
    assert_eq!(item.value().footprint(), (1, 0));
}
#[test]
fn bytes_controls_and_invalid_operand_types_refuse() {
    use sovra::compiler::execution::{CancellationToken, ExecutionControl};
    let p = compile_byte_source(
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_text(\"ab\");}",
    );
    let token = CancellationToken::new();
    token.cancel();
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.control = ExecutionControl {
        cancellation: Some(token),
        max_elapsed: None,
    };
    assert!(execute_generic_data_package(&p, policy).is_err());
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.control.max_elapsed = Some(std::time::Duration::ZERO);
    assert!(execute_generic_data_package(&p, policy).is_err());
    for source in [
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_text(1);}",
        "fn main()->Int{return std::bytes_len(\"ab\");}",
        "fn main()->std::Result<[std::Byte],std::Error>{return std::Result::<[std::Byte],std::Error>::Ok(std::bytes_from_text(\"ab\"))?;}",
        "fn main()->std::Result<Bool,std::Error>{return std::Result::<Bool,std::Error>::Ok(std::bytes_from_text(\"ab\")?==[1,2]);}",
    ] {
        let f = Fixture::new(source);
        assert!(
            compile_generic_data_package(
                capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
                "main",
                GenericPackageCompilationLimits::DEFAULT
            )
            .is_err(),
            "{source}"
        );
    }
}
#[test]
fn empty_byte_array_and_nested_byte_buffers_execute() {
    let p=compile_byte_source("struct Wrap<T>{value:T}fn main()->std::Result<Wrap<[std::Bytes]>,std::Error>{let empty:[std::Byte]=[];return std::Result::<Wrap<[std::Bytes]>,std::Error>::Ok(Wrap::<[std::Bytes]>{value:[std::bytes_from_array(empty)?,std::bytes_from_text(\"\")?]});}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    let items = out.value().payload().unwrap().field(0).unwrap();
    assert_eq!(items.array_len(), Some(2));
    assert_eq!(items.array_item(0).unwrap().bytes_len(), Some(0));
    assert_eq!(items.array_item(1).unwrap().footprint(), (1, 0));
}

#[test]
fn bytes_get_returns_checked_result_at_boundaries_and_utf8_byte_offsets() {
    for (index, expected) in [
        ("0", Some(206)),
        ("1", Some(187)),
        ("2", None),
        ("0-1", None),
        ("9223372036854775807", None),
    ] {
        let p=compile_byte_source(&format!("fn main()->std::Result<std::Byte,std::Error>{{return std::bytes_get(std::bytes_from_text(\"λ\")?,{index});}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(
            out.value().enum_tag(),
            Some(usize::from(expected.is_none()))
        );
        assert_eq!(out.value().payload().and_then(|p| p.as_byte()), expected);
        if expected.is_none() {
            let error = out.value().payload().unwrap();
            assert_eq!(error.field(0).unwrap().as_str(), Some("bounds"));
            assert_eq!(
                error.field(1).unwrap().as_str(),
                Some("byte index out of bounds")
            );
            let attempted = if index == "0-1" {
                -1
            } else {
                index.parse::<i64>().unwrap()
            };
            assert_eq!(
                error.field(2).unwrap().payload().unwrap().as_int(),
                Some(attempted)
            );
        }
        assert_eq!(
            out.value().footprint(),
            (
                if expected.is_some() { 2 } else { 6 },
                if expected.is_some() { 13 } else { 86 }
            )
        );
    }
    let p = compile_byte_source(
        "fn main()->std::Result<std::Byte,std::Error>{return std::bytes_get(std::bytes_from_text(\"\")?,0);}",
    );
    assert_eq!(
        execute_generic_data_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .enum_tag(),
        Some(1)
    );
}
#[test]
fn bytes_get_preserves_full_byte_range_and_imported_generic_ownership() {
    let p=compile_byte_source("use dep::ops;fn main()->std::Result<std::Byte,std::Error>{let x=std::byte_from_int(255)?;let values:[std::Byte]=[x];let bytes=dep::ops::id::<std::Bytes>(std::bytes_from_array(values)?);return std::bytes_get(bytes,0);}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    let byte = out.value().payload().unwrap();
    assert_eq!(byte.as_byte(), Some(255));
    assert_eq!(byte.as_int(), None);
}
#[test]
fn bytes_concat_preserves_order_inputs_and_empty_buffers() {
    for (left, right) in [("", ""), ("a", ""), ("", "λ"), ("λ", "🦀")] {
        let p=compile_byte_source(&format!("struct Triple<T>{{left:T,right:T,joined:T}}fn main()->std::Result<Triple<std::Bytes>,std::Error>{{let a=std::bytes_from_text(\"{left}\")?;let b=std::bytes_from_text(\"{right}\")?;let joined=std::bytes_concat(a,b)?;return std::Result::<Triple<std::Bytes>,std::Error>::Ok(Triple::<std::Bytes>{{left:a,right:b,joined:joined}});}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        for (field, expected) in [left.to_owned(), right.to_owned(), format!("{left}{right}")]
            .iter()
            .enumerate()
        {
            let value = out.value().payload().unwrap().field(field).unwrap();
            assert_eq!(value.bytes_len(), Some(expected.len()));
            assert_eq!(value.footprint(), (expected.len() + 1, 0));
            for (i, byte) in expected.bytes().enumerate() {
                assert_eq!(value.byte_item(i).unwrap().as_byte(), Some(byte));
            }
        }
    }
}
#[test]
fn bytes_get_and_concat_charge_complete_outputs() {
    for (source,nodes,text) in [
        ("fn main()->std::Result<std::Byte,std::Error>{return std::bytes_get(std::bytes_from_text(\"ab\")?,0);}",11,28),
        ("fn main()->std::Result<std::Byte,std::Error>{return std::bytes_get(std::bytes_from_text(\"ab\")?,2);}",15,101),
        ("fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_concat(std::bytes_from_text(\"a\")?,std::bytes_from_text(\"b\")?);}",16,41),
    ] {
        let p=compile_byte_source(source);
        let mut policy=GenericVmPolicy::DEFAULT;
        policy.runtime.materialization.max_value_nodes=Some(nodes);
        policy.runtime.materialization.max_text_bytes=Some(text);
        assert!(execute_generic_data_package(&p,policy.clone()).is_ok(),"{source}");
        policy.runtime.materialization.max_value_nodes=Some(nodes-1);
        assert!(execute_generic_data_package(&p,policy.clone()).is_err());
        policy.runtime.materialization.max_value_nodes=Some(nodes);
        policy.runtime.materialization.max_text_bytes=Some(text-1);
        assert!(execute_generic_data_package(&p,policy).is_err());
        assert!(execute_generic_collection_package(&p,GenericVmPolicy::DEFAULT).is_err());
        assert!(p.encode_generic_collection_package_native_image().is_err());
    }
}
#[test]
fn bytes_access_and_concat_reject_implicit_types() {
    for source in [
        "fn main()->std::Result<std::Byte,std::Error>{return std::bytes_get(\"a\",0);}",
        "fn main()->std::Result<std::Byte,std::Error>{return std::bytes_get(std::bytes_from_text(\"a\")?,true);}",
        "fn main()->std::Result<Int,std::Error>{return std::bytes_get(std::bytes_from_text(\"a\")?,0);}",
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_concat(std::bytes_from_text(\"a\")?,\"b\");}",
    ] {
        let f=Fixture::new(source);
        assert!(compile_generic_data_package(capture_package_sources(f.0.join("app"),PackageCaptureLimits::DEFAULT).unwrap(),"main",GenericPackageCompilationLimits::DEFAULT).is_err(),"{source}");
    }
}
#[test]
fn bytes_access_and_concat_obey_exact_shared_work_and_cancellation() {
    use sovra::compiler::execution::CancellationToken;
    for source in [
        "fn main()->std::Result<std::Byte,std::Error>{return std::bytes_get(std::bytes_from_text(\"a\")?,0);}",
        "fn main()->std::Result<std::Byte,std::Error>{return std::bytes_get(std::bytes_from_text(\"a\")?,1);}",
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_concat(std::bytes_from_text(\"ab\")?,std::bytes_from_text(\"cd\")?);}",
    ] {
        let p=compile_byte_source(source);
        let out=execute_generic_data_package(&p,GenericVmPolicy::DEFAULT).unwrap();
        let mut policy=GenericVmPolicy::DEFAULT;
        policy.max_work=out.work();
        assert!(execute_generic_data_package(&p,policy.clone()).is_ok());
        policy.max_work-=1;
        assert!(execute_generic_data_package(&p,policy).is_err());
        let token=CancellationToken::new();token.cancel();
        let mut policy=GenericVmPolicy::DEFAULT;policy.control.cancellation=Some(token);
        assert!(execute_generic_data_package(&p,policy).is_err());
    }
}
#[test]
fn byte_access_structured_bounds_errors_propagate_without_fabricated_bytes() {
    let p=compile_byte_source("fn main()->std::Result<Int,std::Error>{let byte=std::bytes_get(std::bytes_from_text(\"a\")?,0-1)?;return std::Result::<Int,std::Error>::Ok(std::byte_to_int(byte));}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    let error = out.value().payload().unwrap();
    assert_eq!(error.field(0).unwrap().as_str(), Some("bounds"));
    assert_eq!(
        error.field(2).unwrap().payload().unwrap().as_int(),
        Some(-1)
    );
}

#[test]
fn buffer_constructor_capacity_is_a_structured_language_error() {
    let p = compile_byte_source(
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_text(\"λ\");}",
    );
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_byte_buffer_bytes = 2;
    let out = execute_generic_data_package(&p, policy.clone()).unwrap();
    assert_eq!(out.value().enum_tag(), Some(0));
    assert_eq!(out.value().footprint(), (4, 13));
    policy.max_byte_buffer_bytes = 1;
    let out = execute_generic_data_package(&p, policy).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    assert_eq!(out.value().footprint(), (5, 93));
    let error = out.value().payload().unwrap();
    assert_eq!(error.field(0).unwrap().as_str(), Some("capacity"));
    assert_eq!(
        error.field(1).unwrap().as_str(),
        Some("byte buffer capacity exceeded")
    );
    assert_eq!(error.field(2).unwrap().enum_tag(), Some(1));
}
#[test]
fn array_constructor_capacity_and_propagation_preserve_error_ownership() {
    let p=compile_byte_source("fn main()->std::Result<Int,std::Error>{let hi=std::byte_from_int(255)?;let lo=std::byte_from_int(0)?;let items:[std::Byte]=[hi,lo];let bytes=std::bytes_from_array(items)?;return std::Result::<Int,std::Error>::Ok(std::bytes_len(bytes));}");
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_byte_buffer_bytes = 2;
    assert_eq!(
        execute_generic_data_package(&p, policy.clone())
            .unwrap()
            .value()
            .payload()
            .unwrap()
            .as_int(),
        Some(2)
    );
    policy.max_byte_buffer_bytes = 1;
    let out = execute_generic_data_package(&p, policy).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    let error = out.value().payload().unwrap();
    assert_eq!(error.field(0).unwrap().as_str(), Some("capacity"));
    assert_eq!(error.field(2).unwrap().enum_tag(), Some(1));
}
#[test]
fn constructor_error_preflights_whole_owned_result_and_obeys_exact_work() {
    let p = compile_byte_source(
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_text(\"ab\");}",
    );
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_byte_buffer_bytes = 1;
    policy.runtime.materialization.max_value_nodes = Some(6); // String + complete five-node Err
    policy.runtime.materialization.max_text_bytes = Some(95); // input + complete owned error labels/text
    let out = execute_generic_data_package(&p, policy.clone()).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    policy.max_work = out.work();
    assert!(execute_generic_data_package(&p, policy.clone()).is_ok());
    policy.max_work -= 1;
    assert!(execute_generic_data_package(&p, policy.clone()).is_err());
    policy.max_work = GenericVmPolicy::DEFAULT.max_work;
    policy.runtime.materialization.max_value_nodes = Some(5);
    assert!(execute_generic_data_package(&p, policy.clone()).is_err());
    policy.runtime.materialization.max_value_nodes = Some(6);
    policy.runtime.materialization.max_text_bytes = Some(94);
    assert!(execute_generic_data_package(&p, policy).is_err());
}
#[test]
fn zero_buffer_capacity_allows_empty_constructors_and_checks_concat_capacity() {
    for source in [
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_text(\"\");}",
        "fn main()->std::Result<std::Bytes,std::Error>{let items:[std::Byte]=[];return std::bytes_from_array(items);}",
    ] {
        let p=compile_byte_source(source);
        let mut policy=GenericVmPolicy::DEFAULT;policy.max_byte_buffer_bytes=0;
        let out=execute_generic_data_package(&p,policy).unwrap();
        assert_eq!(out.value().enum_tag(),Some(0));
        assert_eq!(out.value().footprint(),(2,13));
        assert_eq!(out.value().payload().unwrap().bytes_len(),Some(0));
    }
    let p=compile_byte_source("fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_concat(std::bytes_from_text(\"a\")?,std::bytes_from_text(\"b\")?);}");
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_byte_buffer_bytes = 1;
    let out = execute_generic_data_package(&p, policy).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    assert_eq!(
        out.value().payload().unwrap().field(0).unwrap().as_str(),
        Some("capacity")
    );
}
#[test]
fn constructor_results_and_capacity_policy_cannot_be_implicitly_erased() {
    for source in [
        "fn main()->std::Bytes{return std::bytes_from_text(\"a\");}",
        "fn main()->std::Result<Int,std::Error>{return std::bytes_from_text(\"a\");}",
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_array([1]);}",
    ] {
        let f = Fixture::new(source);
        assert!(
            compile_generic_data_package(
                capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
                "main",
                GenericPackageCompilationLimits::DEFAULT
            )
            .is_err(),
            "{source}"
        );
    }
    let p = compile_byte_source("fn main(){}");
    for cap in [RuntimeBytesLimits::DEFAULT.max_bytes + 1, usize::MAX] {
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.max_byte_buffer_bytes = cap;
        assert!(execute_generic_data_package(&p, policy).is_err());
    }
}

#[test]
fn byte_slices_cover_every_half_open_utf8_byte_range_and_bounds_failure() {
    let text = "λ🦀";
    for start in 0..=text.len() + 1 {
        for end in 0..=text.len() + 1 {
            let p=compile_byte_source(&format!("fn main()->std::Result<std::Bytes,std::Error>{{return std::bytes_slice(std::bytes_from_text(\"{text}\")?,{start},{end});}}"));
            let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
            if start <= end && end <= text.len() {
                assert_eq!(out.value().enum_tag(), Some(0));
                let bytes = out.value().payload().unwrap();
                assert_eq!(bytes.bytes_len(), Some(end - start));
                assert_eq!(out.value().footprint(), (end - start + 2, 13));
                for (i, byte) in text.as_bytes()[start..end].iter().enumerate() {
                    assert_eq!(bytes.byte_item(i).unwrap().as_byte(), Some(*byte));
                }
            } else {
                assert_eq!(out.value().enum_tag(), Some(1));
                assert_eq!(out.value().footprint(), (6, 86));
                let error = out.value().payload().unwrap();
                assert_eq!(error.field(0).unwrap().as_str(), Some("bounds"));
                assert_eq!(
                    error.field(1).unwrap().as_str(),
                    Some("byte slice out of bounds")
                );
                assert_eq!(
                    error.field(2).unwrap().payload().unwrap().as_int(),
                    Some(if start > end {
                        start as i64
                    } else {
                        end as i64
                    })
                );
            }
        }
    }
}
#[test]
fn byte_slice_signed_extremes_return_original_indices_without_wrapping() {
    for (start, end, offset) in [
        ("0-1", "0", -1),
        ("0", "0-1", -1),
        ("0-1", "0-2", -1),
        ("0-9223372036854775807-1", "0", i64::MIN),
        ("0", "9223372036854775807", i64::MAX),
        ("9223372036854775807", "0", i64::MAX),
    ] {
        let p=compile_byte_source(&format!("fn main()->std::Result<std::Bytes,std::Error>{{return std::bytes_slice(std::bytes_from_text(\"a\")?,{start},{end});}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(out.value().enum_tag(), Some(1));
        assert_eq!(
            out.value()
                .payload()
                .unwrap()
                .field(2)
                .unwrap()
                .payload()
                .unwrap()
                .as_int(),
            Some(offset)
        );
    }
}
#[test]
fn byte_slice_owns_raw_bytes_and_preserves_original_and_generic_identity() {
    let p=compile_byte_source("use dep::ops;struct Pair<T>{original:T,slice:T}fn main()->std::Result<Pair<std::Bytes>,std::Error>{let original=std::bytes_from_text(\"λ\")?;let slice=dep::ops::id::<std::Bytes>(std::bytes_slice(original,1,2)?);return std::Result::<Pair<std::Bytes>,std::Error>::Ok(Pair::<std::Bytes>{original:original,slice:slice});}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    let pair = out.value().payload().unwrap();
    assert_eq!(pair.field(0).unwrap().bytes_len(), Some(2));
    assert_eq!(
        pair.field(0).unwrap().byte_item(0).unwrap().as_byte(),
        Some(206)
    );
    let slice = pair.field(1).unwrap();
    assert_eq!(slice.bytes_len(), Some(1));
    assert_eq!(slice.byte_item(0).unwrap().as_byte(), Some(187)); // raw invalid UTF-8 is a legal buffer
    assert_eq!(slice.footprint(), (2, 0));
}
#[test]
fn byte_slice_complete_success_error_and_empty_output_budgets_are_exact() {
    for (start, end, nodes, text) in [(0, 1, 13, 28), (2, 2, 12, 28), (0, 3, 16, 101)] {
        let p=compile_byte_source(&format!("fn main()->std::Result<std::Bytes,std::Error>{{return std::bytes_slice(std::bytes_from_text(\"ab\")?,{start},{end});}}"));
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.runtime.materialization.max_value_nodes = Some(nodes);
        policy.runtime.materialization.max_text_bytes = Some(text);
        let out = execute_generic_data_package(&p, policy.clone()).unwrap();
        policy.max_work = out.work();
        assert!(execute_generic_data_package(&p, policy.clone()).is_ok());
        policy.max_work -= 1;
        assert!(execute_generic_data_package(&p, policy.clone()).is_err());
        policy.max_work = GenericVmPolicy::DEFAULT.max_work;
        policy.runtime.materialization.max_value_nodes = Some(nodes - 1);
        assert!(execute_generic_data_package(&p, policy.clone()).is_err());
        policy.runtime.materialization.max_value_nodes = Some(nodes);
        policy.runtime.materialization.max_text_bytes = Some(text - 1);
        assert!(execute_generic_data_package(&p, policy).is_err());
    }
}
#[test]
fn byte_slice_types_arity_and_older_execution_profiles_remain_strict() {
    for source in [
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_slice(\"a\",0,1);}",
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_slice(std::bytes_from_text(\"a\")?,true,1);}",
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_slice(std::bytes_from_text(\"a\")?,0,false);}",
        "fn main()->std::Result<std::Byte,std::Error>{return std::bytes_slice(std::bytes_from_text(\"a\")?,0,1);}",
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_slice(std::bytes_from_text(\"a\")?,0);}",
    ] {
        let f=Fixture::new(source);
        assert!(compile_generic_data_package(capture_package_sources(f.0.join("app"),PackageCaptureLimits::DEFAULT).unwrap(),"main",GenericPackageCompilationLimits::DEFAULT).is_err(),"{source}");
    }
    let p=compile_byte_source("fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_slice(std::bytes_from_text(\"\")?,0,0);}");
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(p.encode_generic_package_native_image().is_err());
    assert!(p.encode_generic_collection_package_native_image().is_err());
}
#[test]
fn byte_slice_bounds_errors_propagate_and_cancellation_refuses() {
    use sovra::compiler::execution::CancellationToken;
    let p=compile_byte_source("fn main()->std::Result<Int,std::Error>{let bytes=std::bytes_slice(std::bytes_from_text(\"a\")?,1,0)?;return std::Result::<Int,std::Error>::Ok(std::bytes_len(bytes));}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    assert_eq!(
        out.value().payload().unwrap().field(0).unwrap().as_str(),
        Some("bounds")
    );
    assert_eq!(
        out.value()
            .payload()
            .unwrap()
            .field(2)
            .unwrap()
            .payload()
            .unwrap()
            .as_int(),
        Some(1)
    );
    let token = CancellationToken::new();
    token.cancel();
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.control.cancellation = Some(token);
    assert!(execute_generic_data_package(&p, policy).is_err());
}

#[test]
fn byte_array_conversion_roundtrips_every_byte_with_distinct_element_identity() {
    for n in 0..=255 {
        let p=compile_byte_source(&format!("fn main()->std::Result<[std::Byte],std::Error>{{let b=std::byte_from_int({n})?;let items:[std::Byte]=[b];return std::bytes_to_array(std::bytes_from_array(items)?);}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(out.value().enum_tag(), Some(0));
        assert_eq!(out.value().footprint(), (3, 13));
        let array = out.value().payload().unwrap();
        assert_eq!(array.array_len(), Some(1));
        assert_eq!(array.bytes_len(), None);
        let byte = array.array_item(0).unwrap();
        assert_eq!(byte.as_byte(), Some(n as u8));
        assert_eq!(byte.as_int(), None);
    }
}
#[test]
fn byte_array_conversion_copies_utf8_and_raw_slice_bytes_without_decoding() {
    for text in ["", "ascii", "λ🦀", "é"] {
        let p=compile_byte_source(&format!("fn main()->std::Result<[std::Byte],std::Error>{{return std::bytes_to_array(std::bytes_from_text(\"{text}\")?);}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(out.value().footprint(), (text.len() + 2, 13));
        let array = out.value().payload().unwrap();
        assert_eq!(array.array_len(), Some(text.len()));
        for (i, byte) in text.bytes().enumerate() {
            assert_eq!(array.array_item(i).unwrap().as_byte(), Some(byte));
        }
    }
    let p=compile_byte_source("fn main()->std::Result<[std::Byte],std::Error>{return std::bytes_to_array(std::bytes_slice(std::bytes_from_text(\"λ\")?,1,2)?);}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(
        out.value()
            .payload()
            .unwrap()
            .array_item(0)
            .unwrap()
            .as_byte(),
        Some(187)
    );
}
#[test]
fn byte_array_mutation_and_imported_forwarding_preserve_original_buffer() {
    let p=compile_byte_source("use dep::ops;struct Pair<T>{buffer:std::Bytes,array:T}fn main()->std::Result<Pair<[std::Byte]>,std::Error>{let buffer=std::bytes_from_text(\"λ\")?;let mut items=dep::ops::id::<[std::Byte]>(std::bytes_to_array(buffer)?);items[0]=std::byte_from_int(255)?;return std::Result::<Pair<[std::Byte]>,std::Error>::Ok(Pair::<[std::Byte]>{buffer:buffer,array:items});}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    let pair = out.value().payload().unwrap();
    let original = pair.field(0).unwrap();
    assert_eq!(original.byte_item(0).unwrap().as_byte(), Some(206));
    assert_eq!(original.byte_item(1).unwrap().as_byte(), Some(187));
    let array = pair.field(1).unwrap();
    assert_eq!(array.array_item(0).unwrap().as_byte(), Some(255));
    assert_eq!(array.array_item(1).unwrap().as_byte(), Some(187));
}
#[test]
fn byte_array_conversion_whole_result_budgets_and_shared_work_are_exact() {
    for (text, nodes, bytes) in [("ab", 12, 28), ("", 6, 26)] {
        let p=compile_byte_source(&format!("fn main()->std::Result<[std::Byte],std::Error>{{return std::bytes_to_array(std::bytes_from_text(\"{text}\")?);}}"));
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.runtime.materialization.max_value_nodes = Some(nodes);
        policy.runtime.materialization.max_text_bytes = Some(bytes);
        let out = execute_generic_data_package(&p, policy.clone()).unwrap();
        policy.max_work = out.work();
        assert!(execute_generic_data_package(&p, policy.clone()).is_ok());
        policy.max_work -= 1;
        assert!(execute_generic_data_package(&p, policy.clone()).is_err());
        policy.max_work = GenericVmPolicy::DEFAULT.max_work;
        policy.runtime.materialization.max_value_nodes = Some(nodes - 1);
        assert!(execute_generic_data_package(&p, policy.clone()).is_err());
        policy.runtime.materialization.max_value_nodes = Some(nodes);
        policy.runtime.materialization.max_text_bytes = Some(bytes - 1);
        assert!(execute_generic_data_package(&p, policy).is_err());
    }
}
#[test]
fn byte_array_conversion_rejects_implicit_array_elements_and_result_erasure() {
    for source in [
        "fn main()->std::Result<[std::Byte],std::Error>{return std::bytes_to_array(\"a\");}",
        "fn main()->std::Result<[std::Byte],std::Error>{return std::bytes_to_array([1]);}",
        "fn main()->std::Result<[Int],std::Error>{return std::bytes_to_array(std::bytes_from_text(\"a\")?);}",
        "struct Byte<T>{value:T}fn main()->std::Result<[Byte<Int>],std::Error>{return std::bytes_to_array(std::bytes_from_text(\"a\")?);}",
        "fn main()->[std::Byte]{return std::bytes_to_array(std::bytes_from_text(\"a\"));}",
        "fn main()->std::Result<[std::Byte],std::Error>{return std::bytes_to_array();}",
    ] {
        let f=Fixture::new(source);
        assert!(compile_generic_data_package(capture_package_sources(f.0.join("app"),PackageCaptureLimits::DEFAULT).unwrap(),"main",GenericPackageCompilationLimits::DEFAULT).is_err(),"{source}");
    }
}
#[test]
fn byte_array_conversion_empty_capacity_cancellation_and_profile_admission() {
    use sovra::compiler::execution::CancellationToken;
    let p=compile_byte_source("fn main()->std::Result<[std::Byte],std::Error>{return std::bytes_to_array(std::bytes_from_text(\"\")?);}");
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_byte_buffer_bytes = 0;
    let out = execute_generic_data_package(&p, policy.clone()).unwrap();
    assert_eq!(out.value().payload().unwrap().array_len(), Some(0));
    let failure=compile_byte_source("fn main()->std::Result<[std::Byte],std::Error>{return std::bytes_to_array(std::bytes_from_text(\"a\")?);}");
    let failed = execute_generic_data_package(&failure, policy.clone()).unwrap();
    assert_eq!(failed.value().enum_tag(), Some(1));
    let error = failed.value().payload().unwrap();
    assert_eq!(error.array_len(), None);
    assert_eq!(error.field(0).unwrap().as_str(), Some("capacity"));
    let token = CancellationToken::new();
    token.cancel();
    policy.control.cancellation = Some(token);
    assert!(execute_generic_data_package(&p, policy).is_err());
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(p.encode_generic_package_native_image().is_err());
    assert!(p.encode_generic_collection_package_native_image().is_err());
}

#[test]
fn source_hex_roundtrips_every_byte_and_emits_lowercase() {
    for n in 0..=255 {
        let p=compile_byte_source(&format!("fn main()->std::Result<String,std::Error>{{return std::bytes_to_hex(std::bytes_from_hex(\"{n:02X}\")?);}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(out.value().enum_tag(), Some(0));
        assert_eq!(
            out.value().payload().unwrap().as_str(),
            Some(format!("{n:02x}").as_str())
        );
        assert_eq!(out.value().footprint(), (2, 15));
    }
}
#[test]
fn source_hex_decodes_empty_mixed_case_binary_and_utf8_bytes_exactly() {
    for (text, expected) in [
        ("", &[][..]),
        ("00ff", &[0, 255][..]),
        ("AbCd", &[171, 205][..]),
        ("cebbf09fa680", &[206, 187, 240, 159, 166, 128][..]),
    ] {
        let p=compile_byte_source(&format!("fn main()->std::Result<std::Bytes,std::Error>{{return std::bytes_from_hex(\"{text}\");}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(out.value().enum_tag(), Some(0));
        assert_eq!(out.value().footprint(), (expected.len() + 2, 13));
        let bytes = out.value().payload().unwrap();
        assert_eq!(bytes.bytes_len(), Some(expected.len()));
        for (i, byte) in expected.iter().enumerate() {
            assert_eq!(bytes.byte_item(i).unwrap().as_byte(), Some(*byte));
        }
    }
}
#[test]
fn source_hex_rejects_malformed_input_with_original_byte_offsets() {
    for (text, offset) in [
        ("a", 0),
        ("abc", 2),
        ("0g", 1),
        ("0g0", 1),
        (" 00", 0),
        ("00 ", 2),
        ("0x00", 1),
        ("00\n", 2),
        ("00λ", 2),
        ("λ", 0),
    ] {
        let p=compile_byte_source(&format!("fn main()->std::Result<std::Bytes,std::Error>{{return std::bytes_from_hex({text:?});}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(out.value().enum_tag(), Some(1), "{text:?}");
        assert_eq!(out.value().footprint(), (6, 84));
        let error = out.value().payload().unwrap();
        assert_eq!(error.field(0).unwrap().as_str(), Some("hex"));
        assert_eq!(
            error.field(1).unwrap().as_str(),
            Some("invalid hexadecimal input")
        );
        assert_eq!(
            error.field(2).unwrap().payload().unwrap().as_int(),
            Some(offset)
        );
    }
}
#[test]
fn source_hex_result_payloads_forward_and_errors_propagate() {
    let p=compile_byte_source("use dep::ops;fn main()->std::Result<String,std::Error>{let bytes=dep::ops::id::<std::Bytes>(std::bytes_from_hex(\"00FF\")?);let text=dep::ops::id::<String>(std::bytes_to_hex(bytes)?);return std::Result::<String,std::Error>::Ok(text);}");
    assert_eq!(
        execute_generic_data_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .payload()
            .unwrap()
            .as_str(),
        Some("00ff")
    );
    let p=compile_byte_source("fn main()->std::Result<String,std::Error>{return std::bytes_to_hex(std::bytes_from_hex(\"zz\")?);}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    assert_eq!(
        out.value().payload().unwrap().field(0).unwrap().as_str(),
        Some("hex")
    );
}
#[test]
fn source_hex_whole_owned_results_and_shared_work_have_exact_boundaries() {
    for (source,nodes,text) in [
        ("fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_hex(\"ff\");}",4,15),
        ("fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_hex(\"0g\");}",7,86),
        ("fn main()->std::Result<String,std::Error>{return std::bytes_to_hex(std::bytes_from_hex(\"ff\")?);}",8,30),
        ("fn main()->std::Result<String,std::Error>{return std::bytes_to_hex(std::bytes_from_hex(\"\")?);}",6,26),
    ] {
        let p=compile_byte_source(source);
        let mut policy=GenericVmPolicy::DEFAULT;
        policy.runtime.materialization.max_value_nodes=Some(nodes);
        policy.runtime.materialization.max_text_bytes=Some(text);
        let out=execute_generic_data_package(&p,policy.clone()).unwrap();
        policy.max_work=out.work();
        assert!(execute_generic_data_package(&p,policy.clone()).is_ok());
        policy.max_work-=1;
        assert!(execute_generic_data_package(&p,policy.clone()).is_err());
        policy.max_work=GenericVmPolicy::DEFAULT.max_work;
        policy.runtime.materialization.max_value_nodes=Some(nodes-1);
        assert!(execute_generic_data_package(&p,policy.clone()).is_err());
        policy.runtime.materialization.max_value_nodes=Some(nodes);
        policy.runtime.materialization.max_text_bytes=Some(text-1);
        assert!(execute_generic_data_package(&p,policy).is_err());
    }
}
#[test]
fn source_hex_capacity_priority_empty_limits_and_cancellation_are_explicit() {
    use sovra::compiler::execution::CancellationToken;
    let p = compile_byte_source(
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_hex(\"zz\");}",
    );
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_byte_buffer_bytes = 0;
    let out = execute_generic_data_package(&p, policy.clone()).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    assert_eq!(out.value().footprint(), (5, 87));
    assert_eq!(
        out.value().payload().unwrap().field(0).unwrap().as_str(),
        Some("capacity")
    );
    assert_eq!(
        out.value().payload().unwrap().field(2).unwrap().enum_tag(),
        Some(1)
    );
    let p=compile_byte_source("fn main()->std::Result<String,std::Error>{return std::bytes_to_hex(std::bytes_from_hex(\"\")?);}");
    assert_eq!(
        execute_generic_data_package(&p, policy.clone())
            .unwrap()
            .value()
            .payload()
            .unwrap()
            .as_str(),
        Some("")
    );
    let token = CancellationToken::new();
    token.cancel();
    policy.control.cancellation = Some(token);
    assert!(execute_generic_data_package(&p, policy).is_err());
}
#[test]
fn source_hex_exact_types_arity_and_profile_admission_stay_strict() {
    for source in [
        "fn main()->std::Result<String,std::Error>{return std::bytes_to_hex(\"a\");}",
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_hex(std::bytes_from_text(\"ff\")?);}",
        "fn main()->std::Result<Int,std::Error>{return std::bytes_to_hex(std::bytes_from_text(\"a\")?);}",
        "fn main()->std::Result<std::Byte,std::Error>{return std::bytes_from_hex(\"ff\");}",
        "fn main()->String{return std::bytes_to_hex(std::bytes_from_text(\"a\"));}",
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_hex();}",
    ] {
        let f=Fixture::new(source);
        assert!(compile_generic_data_package(capture_package_sources(f.0.join("app"),PackageCaptureLimits::DEFAULT).unwrap(),"main",GenericPackageCompilationLimits::DEFAULT).is_err(),"{source}");
    }
    let p = compile_byte_source(
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_hex(\"ff\");}",
    );
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(p.encode_generic_package_native_image().is_err());
    assert!(p.encode_generic_collection_package_native_image().is_err());
}

#[test]
fn source_base64_matches_rfc4648_vectors_and_all_padding_lengths() {
    for (raw, encoded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        let p=compile_byte_source(&format!("fn main()->std::Result<String,std::Error>{{return std::bytes_to_base64(std::bytes_from_text(\"{raw}\")?);}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(out.value().payload().unwrap().as_str(), Some(encoded));
        assert_eq!(out.value().footprint(), (2, 13 + encoded.len() as u64));
        let p=compile_byte_source(&format!("fn main()->std::Result<std::Bytes,std::Error>{{return std::bytes_from_base64(\"{encoded}\");}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(out.value().footprint(), (raw.len() + 2, 13));
        let bytes = out.value().payload().unwrap();
        assert_eq!(bytes.bytes_len(), Some(raw.len()));
        for (i, byte) in raw.bytes().enumerate() {
            assert_eq!(bytes.byte_item(i).unwrap().as_byte(), Some(byte));
        }
    }
}
#[test]
fn source_base64_all_single_bytes_match_runtime_reference_and_decode_exactly() {
    use sovra::compiler::execution::MaterializationLimits;
    for byte in 0..=255u8 {
        let mut fuel = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
        let mut work = RuntimeDataWork::new(10000);
        let reference =
            RuntimeBytesV2::from_slice(&[byte], RuntimeBytesLimits::DEFAULT, &mut fuel, &mut work)
                .unwrap()
                .to_base64(RuntimeBytesLimits::DEFAULT, &mut fuel, &mut work)
                .unwrap();
        let p=compile_byte_source(&format!("fn main()->std::Result<String,std::Error>{{return std::bytes_to_base64(std::bytes_from_hex(\"{byte:02x}\")?);}}"));
        assert_eq!(
            execute_generic_data_package(&p, GenericVmPolicy::DEFAULT)
                .unwrap()
                .value()
                .payload()
                .unwrap()
                .as_str(),
            Some(reference.as_str())
        );
        let p=compile_byte_source(&format!("fn main()->std::Result<std::Bytes,std::Error>{{return std::bytes_from_base64(\"{reference}\");}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(
            out.value()
                .payload()
                .unwrap()
                .byte_item(0)
                .unwrap()
                .as_byte(),
            Some(byte)
        );
    }
}
#[test]
fn source_base64_malformed_inputs_match_runtime_original_byte_offsets() {
    use sovra::compiler::execution::MaterializationLimits;
    for text in [
        "A", "AA", "AAA", "AA=", "AA===", "====", "=AAA", "A=AA", "AA=A", "AAA=AAAA", "AB==",
        "AAB=", "_w==", "-w==", "Z g==", "Zg==\n", "Zg===", "00λ", "Zg==x",
    ] {
        let mut fuel = RuntimeValueFuel::new(MaterializationLimits::UNLIMITED);
        let mut work = RuntimeDataWork::new(10000);
        let reference =
            RuntimeBytesV2::from_base64(text, RuntimeBytesLimits::DEFAULT, &mut fuel, &mut work)
                .unwrap_err();
        assert_eq!(reference.kind, RuntimeDataErrorKind::Base64);
        let p=compile_byte_source(&format!("fn main()->std::Result<std::Bytes,std::Error>{{return std::bytes_from_base64({text:?});}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(out.value().enum_tag(), Some(1), "{text:?}");
        assert_eq!(out.value().footprint(), (6, 82));
        let error = out.value().payload().unwrap();
        assert_eq!(error.field(0).unwrap().as_str(), Some("base64"));
        assert_eq!(
            error.field(1).unwrap().as_str(),
            Some("invalid base64 input")
        );
        assert_eq!(
            error.field(2).unwrap().payload().unwrap().as_int(),
            reference.offset.map(|n| n as i64)
        );
    }
}
#[test]
fn source_base64_generics_and_structured_failure_propagation_preserve_identity() {
    let p=compile_byte_source("use dep::ops;fn main()->std::Result<String,std::Error>{let bytes=dep::ops::id::<std::Bytes>(std::bytes_from_base64(\"Zg==\")?);let text=dep::ops::id::<String>(std::bytes_to_base64(bytes)?);return std::Result::<String,std::Error>::Ok(text);}");
    assert_eq!(
        execute_generic_data_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .payload()
            .unwrap()
            .as_str(),
        Some("Zg==")
    );
    let p=compile_byte_source("fn main()->std::Result<Int,std::Error>{let bytes=std::bytes_from_base64(\"AB==\")?;return std::Result::<Int,std::Error>::Ok(std::bytes_len(bytes));}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    assert_eq!(
        out.value().payload().unwrap().field(0).unwrap().as_str(),
        Some("base64")
    );
    assert_eq!(
        out.value()
            .payload()
            .unwrap()
            .field(2)
            .unwrap()
            .payload()
            .unwrap()
            .as_int(),
        Some(1)
    );
}
#[test]
fn source_base64_full_result_and_error_costs_obey_exact_work_node_text_limits() {
    for (source,nodes,text) in [
        ("fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_base64(\"Zg==\");}",4,17),
        ("fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_base64(\"AB==\");}",7,86),
        ("fn main()->std::Result<String,std::Error>{return std::bytes_to_base64(std::bytes_from_text(\"f\")?);}",8,31),
        ("fn main()->std::Result<String,std::Error>{return std::bytes_to_base64(std::bytes_from_text(\"\")?);}",6,26),
    ] {
        let p=compile_byte_source(source);
        let mut policy=GenericVmPolicy::DEFAULT;
        policy.runtime.materialization.max_value_nodes=Some(nodes);
        policy.runtime.materialization.max_text_bytes=Some(text);
        let out=execute_generic_data_package(&p,policy.clone()).unwrap();
        policy.max_work=out.work();
        assert!(execute_generic_data_package(&p,policy.clone()).is_ok());
        policy.max_work-=1;
        assert!(execute_generic_data_package(&p,policy.clone()).is_err());
        policy.max_work=GenericVmPolicy::DEFAULT.max_work;
        policy.runtime.materialization.max_value_nodes=Some(nodes-1);
        assert!(execute_generic_data_package(&p,policy.clone()).is_err());
        policy.runtime.materialization.max_value_nodes=Some(nodes);
        policy.runtime.materialization.max_text_bytes=Some(text-1);
        assert!(execute_generic_data_package(&p,policy).is_err());
    }
}
#[test]
fn source_base64_capacity_validation_order_zero_limits_and_controls_are_preserved() {
    use sovra::compiler::execution::CancellationToken;
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_byte_buffer_bytes = 0;
    for (text, code) in [("Zg==", "capacity"), ("AB==", "base64")] {
        let p=compile_byte_source(&format!("fn main()->std::Result<std::Bytes,std::Error>{{return std::bytes_from_base64(\"{text}\");}}"));
        let out = execute_generic_data_package(&p, policy.clone()).unwrap();
        assert_eq!(out.value().enum_tag(), Some(1));
        assert_eq!(
            out.value().payload().unwrap().field(0).unwrap().as_str(),
            Some(code)
        );
    }
    let p=compile_byte_source("fn main()->std::Result<String,std::Error>{return std::bytes_to_base64(std::bytes_from_base64(\"\")?);}");
    assert_eq!(
        execute_generic_data_package(&p, policy.clone())
            .unwrap()
            .value()
            .payload()
            .unwrap()
            .as_str(),
        Some("")
    );
    let token = CancellationToken::new();
    token.cancel();
    policy.control.cancellation = Some(token);
    assert!(execute_generic_data_package(&p, policy).is_err());
}
#[test]
fn source_base64_exact_types_arity_and_old_profiles_remain_closed() {
    for source in [
        "fn main()->std::Result<String,std::Error>{return std::bytes_to_base64(\"f\");}",
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_base64(std::bytes_from_text(\"f\")?);}",
        "fn main()->std::Result<Int,std::Error>{return std::bytes_to_base64(std::bytes_from_text(\"f\")?);}",
        "fn main()->std::Result<std::Byte,std::Error>{return std::bytes_from_base64(\"Zg==\");}",
        "fn main()->String{return std::bytes_to_base64(std::bytes_from_text(\"f\"));}",
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_base64();}",
    ] {
        let f=Fixture::new(source);
        assert!(compile_generic_data_package(capture_package_sources(f.0.join("app"),PackageCaptureLimits::DEFAULT).unwrap(),"main",GenericPackageCompilationLimits::DEFAULT).is_err(),"{source}");
    }
    let p = compile_byte_source(
        "fn main()->std::Result<std::Bytes,std::Error>{return std::bytes_from_base64(\"Zg==\");}",
    );
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(p.encode_generic_package_native_image().is_err());
    assert!(p.encode_generic_collection_package_native_image().is_err());
}

#[test]
fn source_utf8_all_single_bytes_match_standard_validation_without_replacement() {
    for byte in 0..=255u8 {
        let input = [byte];
        let reference = std::str::from_utf8(&input);
        let p=compile_byte_source(&format!("fn main()->std::Result<String,std::Error>{{return std::bytes_to_utf8(std::bytes_from_hex(\"{byte:02x}\")?);}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        match reference {
            Ok(text) => {
                assert_eq!(out.value().enum_tag(), Some(0));
                assert_eq!(out.value().payload().unwrap().as_str(), Some(text));
                assert_eq!(out.value().footprint(), (2, 14));
            }
            Err(error) => {
                assert_eq!(out.value().enum_tag(), Some(1));
                assert_eq!(out.value().footprint(), (6, 79));
                let result = out.value().payload().unwrap();
                assert_eq!(result.field(0).unwrap().as_str(), Some("utf8"));
                assert_eq!(
                    result.field(1).unwrap().as_str(),
                    Some("invalid UTF-8 input")
                );
                assert_eq!(
                    result.field(2).unwrap().payload().unwrap().as_int(),
                    Some(error.valid_up_to() as i64)
                );
            }
        }
    }
}
#[test]
fn source_utf8_rfc3629_boundaries_and_original_offsets_match_standard_library() {
    for hex in [
        "c280",
        "dfbf",
        "e0a080",
        "ed9fbf",
        "ee8080",
        "efbfbf",
        "f0908080",
        "f48fbfbf",
        "c080",
        "c1bf",
        "e08080",
        "eda080",
        "f0808080",
        "f4908080",
        "f5808080",
        "8080",
        "c2",
        "e282",
        "f09f98",
        "c241",
        "e28241",
        "f0908041",
        "61c2c2a2",
        "616263e282",
        "efbfbdff",
    ] {
        let input: Vec<u8> = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
            .collect();
        let reference = std::str::from_utf8(&input);
        let p=compile_byte_source(&format!("fn main()->std::Result<String,std::Error>{{return std::bytes_to_utf8(std::bytes_from_hex(\"{hex}\")?);}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        match reference {
            Ok(text) => {
                assert_eq!(out.value().enum_tag(), Some(0));
                assert_eq!(out.value().payload().unwrap().as_str(), Some(text));
                assert_eq!(out.value().footprint(), (2, 13 + input.len() as u64));
            }
            Err(error) => {
                assert_eq!(out.value().enum_tag(), Some(1), "{hex}");
                assert_eq!(
                    out.value()
                        .payload()
                        .unwrap()
                        .field(2)
                        .unwrap()
                        .payload()
                        .unwrap()
                        .as_int(),
                    Some(error.valid_up_to() as i64),
                    "{hex}"
                );
            }
        }
    }
}
#[test]
fn source_utf8_preserves_bom_nul_combining_characters_and_valid_replacement_scalar() {
    for (text, hex) in [
        ("", ""),
        ("a\0b", "610062"),
        ("\u{feff}hello", "efbbbf68656c6c6f"),
        ("é", "65cc81"),
        ("é", "c3a9"),
        ("λ🦀", "cebbf09fa680"),
        ("�", "efbfbd"),
    ] {
        let p=compile_byte_source(&format!("fn main()->std::Result<String,std::Error>{{return std::bytes_to_utf8(std::bytes_from_hex(\"{hex}\")?);}}"));
        let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(out.value().payload().unwrap().as_str(), Some(text));
        assert_eq!(out.value().footprint(), (2, 13 + text.len() as u64));
    }
}
#[test]
fn source_utf8_roundtrip_imported_generics_and_failure_propagation_retain_types() {
    let p=compile_byte_source("use dep::ops;fn main()->std::Result<Bool,std::Error>{let original=std::bytes_from_hex(\"efbbbf61006265cc81\")?;let text=dep::ops::id::<String>(std::bytes_to_utf8(original)?);let copy=std::bytes_from_text(text)?;return std::Result::<Bool,std::Error>::Ok(original==copy);}");
    assert_eq!(
        execute_generic_data_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .payload()
            .unwrap()
            .as_bool(),
        Some(true)
    );
    let p=compile_byte_source("fn main()->std::Result<Int,std::Error>{let text=std::bytes_to_utf8(std::bytes_from_hex(\"61ff\")?)?;return std::Result::<Int,std::Error>::Ok(std::len(text));}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    assert_eq!(
        out.value().payload().unwrap().field(0).unwrap().as_str(),
        Some("utf8")
    );
    assert_eq!(
        out.value()
            .payload()
            .unwrap()
            .field(2)
            .unwrap()
            .payload()
            .unwrap()
            .as_int(),
        Some(1)
    );
}
#[test]
fn source_utf8_complete_success_and_error_costs_obey_exact_shared_limits() {
    for (hex, nodes, text) in [("61", 8, 29), ("", 6, 26), ("ff", 12, 94), ("cebb", 10, 32)] {
        let p=compile_byte_source(&format!("fn main()->std::Result<String,std::Error>{{return std::bytes_to_utf8(std::bytes_from_hex(\"{hex}\")?);}}"));
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.runtime.materialization.max_value_nodes = Some(nodes);
        policy.runtime.materialization.max_text_bytes = Some(text);
        let out = execute_generic_data_package(&p, policy.clone()).unwrap();
        policy.max_work = out.work();
        assert!(execute_generic_data_package(&p, policy.clone()).is_ok());
        policy.max_work -= 1;
        assert!(execute_generic_data_package(&p, policy.clone()).is_err());
        policy.max_work = GenericVmPolicy::DEFAULT.max_work;
        policy.runtime.materialization.max_value_nodes = Some(nodes - 1);
        assert!(execute_generic_data_package(&p, policy.clone()).is_err());
        policy.runtime.materialization.max_value_nodes = Some(nodes);
        policy.runtime.materialization.max_text_bytes = Some(text - 1);
        assert!(execute_generic_data_package(&p, policy).is_err());
    }
}
#[test]
fn source_utf8_zero_capacity_controls_and_profile_admission_remain_explicit() {
    use sovra::compiler::execution::CancellationToken;
    let p=compile_byte_source("fn main()->std::Result<String,std::Error>{return std::bytes_to_utf8(std::bytes_from_hex(\"\")?);}");
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_byte_buffer_bytes = 0;
    assert_eq!(
        execute_generic_data_package(&p, policy.clone())
            .unwrap()
            .value()
            .payload()
            .unwrap()
            .as_str(),
        Some("")
    );
    let token = CancellationToken::new();
    token.cancel();
    policy.control.cancellation = Some(token);
    assert!(execute_generic_data_package(&p, policy).is_err());
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(p.encode_generic_package_native_image().is_err());
    assert!(p.encode_generic_collection_package_native_image().is_err());
}
#[test]
fn source_utf8_operands_and_result_string_identity_cannot_be_implicitly_coerced() {
    for source in [
        "fn main()->std::Result<String,std::Error>{return std::bytes_to_utf8(\"a\");}",
        "fn main()->std::Result<String,std::Error>{return std::bytes_to_utf8([1]);}",
        "fn main()->std::Result<Int,std::Error>{return std::bytes_to_utf8(std::bytes_from_hex(\"61\")?);}",
        "fn main()->String{return std::bytes_to_utf8(std::bytes_from_text(\"a\"));}",
        "fn main()->std::Result<String,std::Error>{return std::bytes_to_utf8();}",
    ] {
        let f=Fixture::new(source);assert!(compile_generic_data_package(capture_package_sources(f.0.join("app"),PackageCaptureLimits::DEFAULT).unwrap(),"main",GenericPackageCompilationLimits::DEFAULT).is_err(),"{source}");
    }
}

#[test]
fn source_json_values_parse_stringify_and_preserve_numeric_kind_and_order() {
    let p=compile_byte_source("fn main()->std::Result<String,std::Error>{let value=std::json_parse(\"{\\\"z\\\":1,\\\"a\\\":1.0,\\\"items\\\":[true,null,\\\"λ\\\"]}\")?;return std::json_stringify(value);}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(
        out.value().payload().unwrap().as_str(),
        Some("{\"z\":1,\"a\":1.0,\"items\":[true,null,\"λ\"]}")
    );
}
#[test]
fn source_json_errors_preserve_original_offsets_and_do_not_return_partial_values() {
    let p=compile_byte_source("fn main()->std::Result<std::JsonValue,std::Error>{return std::json_parse(\"{\\\"a\\\":1,\\\"a\\\":2}\");}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    let error = out.value().payload().unwrap();
    assert_eq!(error.field(0).unwrap().as_str(), Some("json_duplicate"));
    assert_eq!(error.field(2).unwrap().payload().unwrap().as_int(), Some(7));
}
#[test]
fn source_json_explicit_enum_construction_and_shared_limits() {
    let p=compile_byte_source("fn main()->std::Result<String,std::Error>{let value=std::JsonValue::Float(1.0);return std::json_stringify(value);}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().payload().unwrap().as_str(), Some("1.0"));
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.max_work = out.work();
    assert!(execute_generic_data_package(&p, policy.clone()).is_ok());
    policy.max_work -= 1;
    assert!(execute_generic_data_package(&p, policy).is_err());
    let p = compile_byte_source(
        "fn main()->std::Result<std::JsonValue,std::Error>{return std::json_parse(\"[1,2]\");}",
    );
    let mut policy = GenericVmPolicy::DEFAULT;
    policy.json.values = 2;
    let out = execute_generic_data_package(&p, policy).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
}

#[test]
fn source_host_date_math_is_pure_and_privileged_services_deny_by_default() {
    let p=compile_byte_source("fn main()->std::Result<String,std::Error>{let request=std::json_parse(\"{\\\"version\\\":3,\\\"op\\\":\\\"date.parse\\\",\\\"text\\\":\\\"1970-01-01T00:00:00Z\\\"}\")?;let value=std::host_call(request)?;return std::json_stringify(value);}");
    assert_eq!(
        execute_generic_data_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .payload()
            .unwrap()
            .as_str(),
        Some("0")
    );
    let p=compile_byte_source("fn main()->std::Result<std::JsonValue,std::Error>{let request=std::json_parse(\"{\\\"version\\\":3,\\\"op\\\":\\\"clock.wall\\\"}\")?;return std::host_call(request);}");
    let out = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(out.value().enum_tag(), Some(1));
    assert_eq!(
        out.value().payload().unwrap().field(0).unwrap().as_str(),
        Some("authority")
    );
}
#[test]
fn source_host_file_authority_is_explicit_and_returns_captured_binary() {
    let f=Fixture::new("fn main()->std::Result<std::JsonValue,std::Error>{let request=std::json_parse(\"{\\\"version\\\":3,\\\"op\\\":\\\"fs.read\\\",\\\"root\\\":\\\"files\\\",\\\"path\\\":\\\"input.bin\\\"}\")?;return std::host_call(request);}");
    f.write("input.bin", "abc");
    let p = compile_generic_data_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    let denied = execute_generic_data_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    assert_eq!(denied.value().enum_tag(), Some(1));
    let mut host = HostV3::default();
    host.grant_read("files", &f.0).unwrap();
    let out = execute_generic_data_package_with_host(&p, GenericVmPolicy::DEFAULT, &host).unwrap();
    assert_eq!(out.value().enum_tag(), Some(0));
    assert_eq!(
        out.value().payload().unwrap().payload().unwrap().as_str(),
        Some("YWJj")
    );
}
