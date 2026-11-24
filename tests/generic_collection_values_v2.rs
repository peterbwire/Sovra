//! Actual package outputs retain compiler-owned declaration/argument proofs in snapshots.
#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")
))]
use sovra::compiler::{execution::MaterializationLimits as Limits, project::packages::*, v2::*};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
use GenericCollectionSelector::{Field, Item, Payload};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "sovra generic collection λ {} {}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for p in ["app", "dep"] {
            std::fs::create_dir_all(root.join(p)).unwrap()
        }
        let f = Self(root);
        f.write(
            "app/sovra.toml",
            "[project]\nname=\"app\"\nentry=\"main.svr\"\n[dependencies.dep]\npath=\"../dep\"\n",
        );
        f.write(
            "dep/sovra.toml",
            "[project]\nname=\"dep\"\nentry=\"main.svr\"\n[modules]\ntypes=\"types.svr\"\n",
        );
        f.write("dep/main.svr", "fn main(){}");
        f.write("dep/types.svr","mod types{export struct Box<T>{value:T}export struct Empty<T>{}export enum Choice<T>{One(T),End}}");
        f.write("app/main.svr","use dep::types;struct Box<T>{value:T}type Alias<T>=dep::types::Box<T>;struct Holder<T>{local:Box<Int>,imported:dep::types::Box<Int>,alias:Alias<Int>,phantom_int:dep::types::Empty<Int>,phantom_string:dep::types::Empty<String>,choice:dep::types::Choice<Int>,items:[dep::types::Box<Int>]}fn main()->Holder<Int>{let imported=dep::types::Box::<Int>{value:7};let items:[dep::types::Box<Int>]=[imported];return Holder::<Int>{items:items,choice:dep::types::Choice::<Int>::One(9),phantom_string:dep::types::Empty::<String>{},phantom_int:dep::types::Empty::<Int>{},alias:imported,imported:imported,local:Box::<Int>{value:7}};}");
        f
    }
    fn write(&self, p: &str, s: &str) {
        std::fs::write(self.0.join(p), s).unwrap()
    }
    fn compile(&self) -> CompiledGenericPackage {
        compile_generic_package(
            capture_package_sources(self.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
        .unwrap()
    }
    fn run(&self) -> GenericVmOutcome {
        execute_generic_package(&self.compile(), GenericVmPolicy::DEFAULT).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn unlimited() -> RuntimeValueFuel {
    RuntimeValueFuel::new(Limits::UNLIMITED)
}
fn fuel(n: u64, t: u64) -> RuntimeValueFuel {
    RuntimeValueFuel::new(Limits {
        max_value_nodes: Some(n),
        max_text_bytes: Some(t),
    })
}
fn work() -> RuntimeSetWork {
    RuntimeSetWork::new(1_000_000)
}
fn snap(out: &GenericVmOutcome, path: &[GenericCollectionSelector]) -> GenericCollectionSnapshot {
    out.collection_snapshot(path, &mut unlimited(), &mut work())
        .unwrap()
}
fn eq(a: &GenericCollectionSnapshot, b: &GenericCollectionSnapshot) -> bool {
    a.equal(b, GenericTypeEquivalenceLimits::DEFAULT, &mut work())
        .unwrap()
}
#[test]
fn imported_record_owners_aliases_and_generic_arguments_remain_exact() {
    let f = Fixture::new();
    let out = f.run();
    let local = snap(&out, &[Field(0)]);
    let imported = snap(&out, &[Field(1)]);
    let alias = snap(&out, &[Field(2)]);
    assert_ne!(local.ty().declaration(), imported.ty().declaration());
    assert!(!eq(&local, &imported));
    assert!(eq(&imported, &alias));
    assert_eq!(imported.value().field(0).unwrap().as_int(), Some(7));
    assert!(!imported.ty().is_key());
    let a = snap(&out, &[Field(3)]);
    let b = snap(&out, &[Field(4)]);
    assert_eq!(a.ty().declaration(), b.ty().declaration());
    assert!(!eq(&a, &b));
    assert!(!a
        .ty()
        .equivalent(&b.ty(), GenericTypeEquivalenceLimits::DEFAULT)
        .unwrap());
}
#[test]
fn outcome_and_selected_subtrees_remain_usable_after_program_and_source_deletion() {
    let f = Fixture::new();
    let program = f.compile();
    let out = execute_generic_package(&program, GenericVmPolicy::DEFAULT).unwrap();
    let kept = snap(&out, &[]);
    let imported = snap(&out, &[Field(1)]);
    drop(out);
    drop(program);
    std::fs::remove_dir_all(&f.0).unwrap();
    let nested = kept
        .select(&[Field(6), Item(0)], &mut unlimited(), &mut work())
        .unwrap();
    assert!(eq(&nested, &imported));
    let payload = kept
        .select(&[Field(5), Payload], &mut unlimited(), &mut work())
        .unwrap();
    assert_eq!(payload.value().as_int(), Some(9));
    assert!(payload.ty().is_key());
    let clone = kept.snapshot(&mut unlimited(), &mut work()).unwrap();
    assert!(eq(&kept, &clone));
}
#[test]
fn repeated_execution_keeps_owners_but_fresh_checks_do_not_share_nominal_authority() {
    let f = Fixture::new();
    let program = f.compile();
    let a = execute_generic_package(&program, GenericVmPolicy::DEFAULT).unwrap();
    let b = execute_generic_package(&program, GenericVmPolicy::DEFAULT).unwrap();
    let c = f.run();
    let x = snap(&a, &[Field(1)]);
    let y = snap(&b, &[Field(1)]);
    let z = snap(&c, &[Field(1)]);
    assert!(eq(&x, &y));
    assert!(!eq(&x, &z));
    let x = snap(&a, &[Field(1), Field(0)]);
    let y = snap(&c, &[Field(1), Field(0)]);
    assert!(eq(&x, &y));
}
#[test]
fn full_result_costs_are_atomic_and_selection_charges_only_the_subtree() {
    let out = Fixture::new().run();
    let (n, t) = out.value().footprint();
    let mut f = fuel(n as u64, t);
    let copy = out.collection_snapshot(&[], &mut f, &mut work()).unwrap();
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    for (nodes, text, error) in [
        (n as u64 - 1, t, RuntimeValueError::NodeLimit),
        (n as u64, t - 1, RuntimeValueError::TextLimit),
    ] {
        let mut f = fuel(nodes, text);
        let before = f.remaining();
        assert_eq!(
            out.collection_snapshot(&[], &mut f, &mut work())
                .unwrap_err(),
            GenericCollectionError::Materialization(error)
        );
        assert_eq!(f.remaining(), before);
    }
    let imported = out.value().field(1).unwrap();
    let (n, t) = imported.footprint();
    let mut f = fuel(n as u64, t);
    let selected = copy.select(&[Field(1)], &mut f, &mut work()).unwrap();
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    assert_eq!(selected.value().footprint(), (n, t));
    drop(copy);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
}
#[test]
fn invalid_selectors_and_work_refusal_return_no_partial_value_or_fuel_commit() {
    let out = Fixture::new().run();
    for path in [
        vec![Item(0)],
        vec![Field(99)],
        vec![Field(5), Field(0)],
        vec![Field(6), Item(99)],
    ] {
        let mut f = fuel(0, 0);
        assert_eq!(
            out.collection_snapshot(&path, &mut f, &mut work())
                .unwrap_err(),
            GenericCollectionError::Missing
        );
        assert_eq!(f.remaining(), fuel(0, 0).remaining());
    }
    let (n, t) = out.value().field(1).unwrap().footprint();
    let cost = 1 + 2 * n as u64 + t;
    let mut f = fuel(n as u64, t);
    let mut w = RuntimeSetWork::new(cost - 1);
    assert!(matches!(
        out.collection_snapshot(&[Field(1)], &mut f, &mut w),
        Err(GenericCollectionError::Work(RuntimeSetError::WorkLimit))
    ));
    assert_eq!(f.remaining(), fuel(n as u64, t).remaining());
    let mut w = RuntimeSetWork::new(cost);
    out.collection_snapshot(&[Field(1)], &mut f, &mut w)
        .unwrap();
    assert_eq!(w.remaining(), 0);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
}
#[test]
fn key_admission_and_primitive_equality_use_types_instead_of_layout_numbers() {
    let f = Fixture::new();
    let mut values = Vec::new();
    for (source, key) in [
        ("fn main()->Int{return 1;}", true),
        ("fn main()->Bool{return true;}", true),
        ("fn main()->String{return \"λ\";}", true),
        ("fn main()->Float{return 1.0;}", false),
        ("fn main(){}", false),
    ] {
        f.write("app/main.svr", source);
        let v = snap(&f.run(), &[]);
        assert_eq!(v.ty().is_key(), key);
        values.push(v);
    }
    assert!(!eq(&values[0], &values[3]));
    assert!(!eq(&values[0], &values[1]));
    assert!(eq(&values[2], &values[2]));
}
#[test]
fn type_and_value_comparison_limits_refuse_without_fabricating_false() {
    let out = Fixture::new().run();
    let a = snap(&out, &[Field(1)]);
    let b = a.snapshot(&mut unlimited(), &mut work()).unwrap();
    let mut tiny = GenericTypeEquivalenceLimits::DEFAULT;
    tiny.max_pairs = 0;
    assert!(matches!(
        a.ty().equivalent(&b.ty(), tiny),
        Err(GenericCollectionError::Type(_))
    ));
    tiny = GenericTypeEquivalenceLimits::DEFAULT;
    tiny.max_work = 0;
    assert!(matches!(
        a.equal(&b, tiny, &mut work()),
        Err(GenericCollectionError::Type(_))
    ));
    let (n, t) = a.value().footprint();
    let cost = 1 + 2 * n as u64 + 2 * t;
    let mut w = RuntimeSetWork::new(cost - 1);
    assert_eq!(
        a.equal(&b, GenericTypeEquivalenceLimits::DEFAULT, &mut w),
        Err(GenericCollectionError::Work(RuntimeSetError::WorkLimit))
    );
    let mut w = RuntimeSetWork::new(cost);
    assert!(a
        .equal(&b, GenericTypeEquivalenceLimits::DEFAULT, &mut w)
        .unwrap());
    assert_eq!(w.remaining(), 0);
}
#[test]
fn enum_tags_and_nullary_payload_absence_remain_sealed() {
    let f = Fixture::new();
    f.write("app/main.svr","use dep::types;struct Pair<T>{one:dep::types::Choice<Int>,end:dep::types::Choice<Int>}fn main()->Pair<Int>{return Pair::<Int>{end:dep::types::Choice::<Int>::End(),one:dep::types::Choice::<Int>::One(7)};}");
    let out = f.run();
    let a = snap(&out, &[Field(0)]);
    let b = snap(&out, &[Field(1)]);
    assert!(a
        .ty()
        .equivalent(&b.ty(), GenericTypeEquivalenceLimits::DEFAULT)
        .unwrap());
    assert!(!eq(&a, &b));
    assert_eq!(a.value().enum_tag(), Some(0));
    assert_eq!(b.value().enum_tag(), Some(1));
    assert_eq!(
        b.select(&[Payload], &mut fuel(0, 0), &mut work())
            .unwrap_err(),
        GenericCollectionError::Missing
    );
}

#[test]
fn equal_types_do_not_hide_changed_fields_or_reordered_array_values() {
    let f = Fixture::new();
    f.write("app/main.svr","use dep::types;struct Values<T>{a:dep::types::Box<Int>,b:dep::types::Box<Int>,forward:[dep::types::Box<Int>],reverse:[dep::types::Box<Int>]}fn main()->Values<Int>{let a=dep::types::Box::<Int>{value:7};let b=dep::types::Box::<Int>{value:8};let forward:[dep::types::Box<Int>]=[a,b];let reverse:[dep::types::Box<Int>]=[b,a];return Values::<Int>{a:a,b:b,forward:forward,reverse:reverse};}");
    let out = f.run();
    let a = snap(&out, &[Field(0)]);
    let b = snap(&out, &[Field(1)]);
    assert!(a
        .ty()
        .equivalent(&b.ty(), GenericTypeEquivalenceLimits::DEFAULT)
        .unwrap());
    assert!(!eq(&a, &b));
    assert!(!eq(&snap(&out, &[Field(2)]), &snap(&out, &[Field(3)])));
    assert!(eq(
        &snap(&out, &[Field(2), Item(0)]),
        &snap(&out, &[Field(3), Item(1)])
    ));
}
#[test]
fn recursive_generic_records_select_copy_compare_and_drop_as_flat_snapshots() {
    let f = Fixture::new();
    f.write("app/main.svr","struct Node<T>{next:[Node<T>],value:T}fn main()->Node<Int>{let initial:[Node<Int>]=[];let mut n=Node::<Int>{next:initial,value:0};let mut i=0;while i<128{let next:[Node<Int>]=[n];n=Node::<Int>{next:next,value:i};i=i+1;}return n;}");
    let out = f.run();
    let kept = snap(&out, &[]);
    let copied = kept.snapshot(&mut unlimited(), &mut work()).unwrap();
    assert!(eq(&kept, &copied));
    let mut path = Vec::new();
    for _ in 0..128 {
        path.push(Field(0));
        path.push(Item(0));
    }
    path.push(Field(1));
    let mut budget = fuel(1, 0);
    let mut w = RuntimeSetWork::new(path.len() as u64 + 2);
    let leaf = copied.select(&path, &mut budget, &mut w).unwrap();
    assert_eq!(leaf.value().as_int(), Some(0));
    assert_eq!(w.remaining(), 0);
    assert_eq!(budget.remaining(), fuel(0, 0).remaining());
    drop(out);
    drop(kept);
    drop(copied);
    assert!(leaf.ty().is_key());
}

mod map_storage {
    use super::*;
    fn empty(value: &GenericCollectionSnapshot, key: RuntimeKeyType) -> GenericCollectionMap {
        GenericCollectionMap::new(
            key,
            value.ty(),
            GenericTypeEquivalenceLimits::DEFAULT,
            &mut unlimited(),
        )
        .unwrap()
    }
    fn put(
        m: &GenericCollectionMap,
        key: i64,
        value: &GenericCollectionSnapshot,
    ) -> GenericCollectionMap {
        m.insert(&RuntimeKey::Int(key), value, &mut unlimited(), &mut work())
            .unwrap()
    }
    fn cost(
        n: u64,
        t: u64,
        op: impl Fn(&mut RuntimeValueFuel) -> Result<(), GenericCollectionMapError>,
    ) {
        let mut f = fuel(n, t);
        op(&mut f).unwrap();
        assert_eq!(f.remaining(), fuel(0, 0).remaining());
        let mut f = fuel(n - 1, t);
        let before = f.remaining();
        assert_eq!(
            op(&mut f),
            Err(GenericCollectionMapError::Value(
                GenericCollectionError::Materialization(RuntimeValueError::NodeLimit)
            ))
        );
        assert_eq!(f.remaining(), before);
        if t > 0 {
            let mut f = fuel(n, t - 1);
            let before = f.remaining();
            assert_eq!(
                op(&mut f),
                Err(GenericCollectionMapError::Value(
                    GenericCollectionError::Materialization(RuntimeValueError::TextLimit)
                ))
            );
            assert_eq!(f.remaining(), before);
        }
    }
    #[test]
    fn imported_generic_values_keep_owners_through_every_operation() {
        let fixture = Fixture::new();
        let out = fixture.run();
        let v = snap(&out, &[Field(1)]);
        let alias = snap(&out, &[Field(2)]);
        let m = put(&empty(&v, RuntimeKeyType::Int), 1, &alias);
        drop(out);
        std::fs::remove_dir_all(&fixture.0).unwrap();
        let got = m
            .get(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
            .unwrap()
            .unwrap();
        assert!(eq(&v, &got));
        let copy = m.snapshot(&mut unlimited(), &mut work()).unwrap();
        assert!(copy.equal(&m, &mut work()).unwrap());
        let mut entries = m.entries(&mut unlimited(), &mut work()).unwrap();
        entries[0].key = RuntimeKey::Int(99);
        entries.clear();
        assert!(m.contains(&RuntimeKey::Int(1), &mut work()).unwrap());
        let removed = m
            .remove(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
            .unwrap()
            .unwrap();
        assert!(removed.map.is_empty());
        assert!(eq(&v, &removed.value));
        assert!(removed
            .map
            .value_type()
            .equivalent(&v.ty(), GenericTypeEquivalenceLimits::DEFAULT)
            .unwrap());
    }
    #[test]
    fn same_shape_foreign_owner_and_phantom_argument_values_cannot_be_inserted() {
        let f = Fixture::new();
        let out = f.run();
        let imported = snap(&out, &[Field(1)]);
        let local = snap(&out, &[Field(0)]);
        let phantom_int = snap(&out, &[Field(3)]);
        let phantom_string = snap(&out, &[Field(4)]);
        for (m, value) in [
            (empty(&imported, RuntimeKeyType::Int), local),
            (empty(&phantom_int, RuntimeKeyType::Int), phantom_string),
            (
                empty(&imported, RuntimeKeyType::Int),
                snap(&f.run(), &[Field(1)]),
            ),
        ] {
            let mut f = fuel(0, 0);
            assert_eq!(
                m.insert(&RuntimeKey::Int(1), &value, &mut f, &mut work())
                    .unwrap_err(),
                GenericCollectionMapError::ValueType
            );
            assert_eq!(f.remaining(), fuel(0, 0).remaining());
            assert!(m.is_empty());
        }
    }
    #[test]
    fn replacement_and_remove_reinsert_preserve_snapshot_order() {
        let out = Fixture::new().run();
        let a = snap(&out, &[Field(1), Field(0)]);
        let b = snap(&out, &[Field(5), Payload]);
        let e = empty(&a, RuntimeKeyType::Int);
        let first = put(&e, 1, &a);
        let two = put(&first, 2, &a);
        let replaced = put(&two, 1, &b);
        assert_eq!(
            replaced.keys(&mut unlimited(), &mut work()).unwrap(),
            vec![RuntimeKey::Int(1), RuntimeKey::Int(2)]
        );
        assert_eq!(
            two.get(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
                .unwrap()
                .unwrap()
                .value()
                .as_int(),
            Some(7)
        );
        let removed = replaced
            .remove(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
            .unwrap()
            .unwrap();
        assert_eq!(removed.value.value().as_int(), Some(9));
        let again = put(&removed.map, 1, &b);
        assert_eq!(
            again.keys(&mut unlimited(), &mut work()).unwrap(),
            vec![RuntimeKey::Int(2), RuntimeKey::Int(1)]
        );
        assert!(again.equal(&replaced, &mut work()).unwrap());
        assert!(!again.equal(&two, &mut work()).unwrap());
        assert!(e.is_empty());
    }
    #[test]
    fn every_copy_cost_admits_generic_nodes_and_labels_together() {
        let out = Fixture::new().run();
        let v = snap(&out, &[Field(1)]);
        let (n, t) = v.value().footprint();
        assert_eq!(n, 2);
        assert!(t > 0);
        let n = n as u64;
        let key = RuntimeKey::String("λ".into());
        let e = empty(&v, RuntimeKeyType::String);
        cost(1, 0, |f| {
            GenericCollectionMap::new(
                RuntimeKeyType::String,
                v.ty(),
                GenericTypeEquivalenceLimits::DEFAULT,
                f,
            )
            .map(|_| ())
        });
        cost(n + 3, t + 2, |f| {
            e.insert(&key, &v, f, &mut work()).map(|_| ())
        });
        let m = e.insert(&key, &v, &mut unlimited(), &mut work()).unwrap();
        cost(n + 3, t + 2, |f| m.snapshot(f, &mut work()).map(|_| ()));
        cost(n + 3, t + 2, |f| m.entries(f, &mut work()).map(|_| ()));
        cost(n + 3, t + 2, |f| {
            m.insert(&key, &v, f, &mut work()).map(|_| ())
        });
        cost(n + 1, t, |f| m.values(f, &mut work()).map(|_| ()));
        cost(2, 2, |f| m.keys(f, &mut work()).map(|_| ()));
        cost(n, t, |f| m.get(&key, f, &mut work()).map(|_| ()));
        cost(n + 1, t, |f| m.remove(&key, f, &mut work()).map(|_| ()));
    }
    #[test]
    fn missing_keys_copy_nothing_and_declared_domains_cannot_mix() {
        let out = Fixture::new().run();
        let v = snap(&out, &[Field(1), Field(0)]);
        let m = put(&empty(&v, RuntimeKeyType::Int), 1, &v);
        let mut f = fuel(0, 0);
        assert!(m
            .get(&RuntimeKey::Int(9), &mut f, &mut work())
            .unwrap()
            .is_none());
        assert!(m
            .remove(&RuntimeKey::Int(9), &mut f, &mut work())
            .unwrap()
            .is_none());
        assert!(!m.contains(&RuntimeKey::Int(9), &mut work()).unwrap());
        assert_eq!(
            m.get(&RuntimeKey::Bool(true), &mut f, &mut work())
                .unwrap_err(),
            GenericCollectionMapError::KeyType
        );
        assert_eq!(
            m.equal(&empty(&v, RuntimeKeyType::Bool), &mut work()),
            Err(GenericCollectionMapError::KeyType)
        );
        assert_eq!(
            m.equal(
                &empty(&snap(&out, &[Field(1)]), RuntimeKeyType::Int),
                &mut work()
            ),
            Err(GenericCollectionMapError::ValueType)
        );
        assert_eq!(f.remaining(), fuel(0, 0).remaining());
    }
    #[test]
    fn exact_generic_copy_work_and_equality_refusal_do_not_commit_fuel() {
        let out = Fixture::new().run();
        let v = snap(&out, &[Field(1), Field(0)]);
        assert_eq!(v.value().footprint(), (1, 0));
        let e = empty(&v, RuntimeKeyType::Int);
        let mut f = fuel(4, 0);
        let mut w = RuntimeSetWork::new(4);
        assert_eq!(
            e.insert(&RuntimeKey::Int(1), &v, &mut f, &mut w)
                .unwrap_err(),
            GenericCollectionMapError::Value(GenericCollectionError::Work(
                RuntimeSetError::WorkLimit
            ))
        );
        assert_eq!(f.remaining(), fuel(4, 0).remaining());
        let mut w = RuntimeSetWork::new(5);
        let m = e.insert(&RuntimeKey::Int(1), &v, &mut f, &mut w).unwrap();
        assert_eq!(w.remaining(), 0);
        assert_eq!(f.remaining(), fuel(0, 0).remaining());
        let mut f = fuel(4, 0);
        let mut w = RuntimeSetWork::new(5);
        assert!(m.snapshot(&mut f, &mut w).is_err());
        assert_eq!(f.remaining(), fuel(4, 0).remaining());
        let mut w = RuntimeSetWork::new(6);
        m.snapshot(&mut f, &mut w).unwrap();
        assert_eq!(w.remaining(), 0);
        let mut w = RuntimeSetWork::new(3);
        assert!(m.equal(&m, &mut w).is_err());
        let mut w = RuntimeSetWork::new(4);
        assert!(m.equal(&m, &mut w).unwrap());
        assert_eq!(w.remaining(), 0);
    }
    #[test]
    fn unicode_string_keys_and_bool_keys_stay_exact() {
        let out = Fixture::new().run();
        let v = snap(&out, &[Field(1), Field(0)]);
        let mut m = empty(&v, RuntimeKeyType::String);
        for key in ["é", "e\u{301}", ""] {
            m = m
                .insert(
                    &RuntimeKey::String(key.into()),
                    &v,
                    &mut unlimited(),
                    &mut work(),
                )
                .unwrap();
        }
        assert_eq!(m.len(), 3);
        assert_eq!(m.key_type(), RuntimeKeyType::String);
        let m = empty(&v, RuntimeKeyType::Bool);
        let m = m
            .insert(&RuntimeKey::Bool(true), &v, &mut unlimited(), &mut work())
            .unwrap();
        let m = m
            .insert(&RuntimeKey::Bool(false), &v, &mut unlimited(), &mut work())
            .unwrap();
        assert_eq!(m.len(), 2);
    }
    #[test]
    fn declared_type_comparison_exhaustion_is_an_error_before_empty_map_fuel() {
        let out = Fixture::new().run();
        let v = snap(&out, &[Field(1)]);
        let mut limits = GenericTypeEquivalenceLimits::DEFAULT;
        limits.max_pairs = 0;
        let mut f = fuel(1, 0);
        assert!(matches!(
            GenericCollectionMap::new(RuntimeKeyType::Int, v.ty(), limits, &mut f),
            Err(GenericCollectionMapError::Value(
                GenericCollectionError::Type(_)
            ))
        ));
        assert_eq!(f.remaining(), fuel(1, 0).remaining());
    }
    #[test]
    fn enum_and_array_values_retain_their_original_generic_proofs() {
        let out = Fixture::new().run();
        for field in [5, 6] {
            let v = snap(&out, &[Field(field)]);
            let m = put(&empty(&v, RuntimeKeyType::Int), 1, &v);
            let got = m
                .get(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
                .unwrap()
                .unwrap();
            assert!(eq(&v, &got));
            let values = m.values(&mut unlimited(), &mut work()).unwrap();
            assert!(eq(&v, &values[0]));
        }
    }
    #[test]
    fn exhaustive_short_updates_match_an_ordered_association_oracle() {
        let out = Fixture::new().run();
        let values = [
            snap(&out, &[Field(1), Field(0)]),
            snap(&out, &[Field(5), Payload]),
        ];
        for trace in 0..6usize.pow(4) {
            let mut code = trace;
            let mut expected: Vec<(i64, usize)> = Vec::new();
            let mut m = empty(&values[0], RuntimeKeyType::Int);
            for _ in 0..4 {
                let op = code % 6;
                code /= 6;
                let key = (op % 2) as i64;
                if op < 4 {
                    let value = op / 2;
                    if let Some(pair) = expected.iter_mut().find(|p| p.0 == key) {
                        pair.1 = value
                    } else {
                        expected.push((key, value))
                    }
                    m = put(&m, key, &values[value]);
                } else {
                    let pos = expected.iter().position(|p| p.0 == key);
                    let removed = m
                        .remove(&RuntimeKey::Int(key), &mut unlimited(), &mut work())
                        .unwrap();
                    assert_eq!(removed.is_some(), pos.is_some());
                    if let Some(i) = pos {
                        let (_, value) = expected.remove(i);
                        let removed = removed.unwrap();
                        assert!(eq(&removed.value, &values[value]));
                        m = removed.map;
                    }
                }
                let entries = m.entries(&mut unlimited(), &mut work()).unwrap();
                assert_eq!(entries.len(), expected.len());
                for (e, (key, value)) in entries.iter().zip(&expected) {
                    assert_eq!(e.key, RuntimeKey::Int(*key));
                    assert!(eq(&e.value, &values[*value]), "trace {trace}");
                }
                let mut reverse = empty(&values[0], RuntimeKeyType::Int);
                for (key, value) in expected.iter().rev() {
                    reverse = put(&reverse, *key, &values[*value]);
                }
                assert!(m.equal(&reverse, &mut work()).unwrap());
            }
        }
    }
}

mod collection_layouts {
    use super::*;
    #[test]
    fn opaque_constructor_arity_and_checked_argument_owners_are_retained() {
        let out = Fixture::new().run();
        let key = snap(&out, &[Field(1), Field(0)]);
        let value = snap(&out, &[Field(1)]);
        let map = GenericCollectionLayout::map(
            key.ty(),
            value.ty(),
            GenericTypeEquivalenceLimits::DEFAULT,
        )
        .unwrap();
        assert_eq!(map.kind(), GenericCollectionLayoutKind::Map);
        assert_eq!(map.qualified_name(), "std::Map");
        assert_eq!(map.argument_count(), 2);
        assert_eq!(map.key_domain(), RuntimeKeyType::Int);
        assert!(map.argument(0).unwrap().is_key());
        assert_eq!(
            map.argument(1).unwrap().declaration(),
            value.ty().declaration()
        );
        assert!(map.argument(2).is_none());
        assert_eq!(map.type_limits(), GenericTypeEquivalenceLimits::DEFAULT);
        let set =
            GenericCollectionLayout::set(key.ty(), GenericTypeEquivalenceLimits::DEFAULT).unwrap();
        assert_eq!(set.kind(), GenericCollectionLayoutKind::Set);
        assert_eq!(set.qualified_name(), "std::Set");
        assert_eq!(set.argument_count(), 1);
        assert!(set.argument(1).is_none());
        assert!(!map.equivalent(&set).unwrap());
    }
    #[test]
    fn layout_identity_preserves_aliases_nominal_owners_and_phantom_arguments() {
        let f = Fixture::new();
        let out = f.run();
        let key = snap(&out, &[Field(1), Field(0)]);
        let layout = |value: GenericCollectionSnapshot| {
            GenericCollectionLayout::map(
                key.ty(),
                value.ty(),
                GenericTypeEquivalenceLimits::DEFAULT,
            )
            .unwrap()
        };
        let imported = layout(snap(&out, &[Field(1)]));
        let alias = layout(snap(&out, &[Field(2)]));
        let local = layout(snap(&out, &[Field(0)]));
        assert!(imported.equivalent(&alias).unwrap());
        assert!(!imported.equivalent(&local).unwrap());
        assert!(!layout(snap(&out, &[Field(3)]))
            .equivalent(&layout(snap(&out, &[Field(4)])))
            .unwrap());
        assert!(!imported
            .equivalent(&layout(snap(&f.run(), &[Field(1)])))
            .unwrap());
    }
    #[test]
    fn float_unit_and_compound_key_layouts_are_rejected_without_shape_coercion() {
        let f = Fixture::new();
        let out = f.run();
        let value = snap(&out, &[Field(1)]);
        for path in [
            vec![Field(0)],
            vec![Field(3)],
            vec![Field(5)],
            vec![Field(6)],
        ] {
            let key = snap(&out, &path);
            assert!(matches!(
                GenericCollectionLayout::set(key.ty(), GenericTypeEquivalenceLimits::DEFAULT),
                Err(GenericCollectionLayoutError::KeyBound)
            ));
            assert!(matches!(
                GenericCollectionLayout::map(
                    key.ty(),
                    value.ty(),
                    GenericTypeEquivalenceLimits::DEFAULT
                ),
                Err(GenericCollectionLayoutError::KeyBound)
            ));
        }
        for source in ["fn main()->Float{return 1.0;}", "fn main(){}"] {
            f.write("app/main.svr", source);
            let key = snap(&f.run(), &[]);
            assert!(matches!(
                GenericCollectionLayout::set(key.ty(), GenericTypeEquivalenceLimits::DEFAULT),
                Err(GenericCollectionLayoutError::KeyBound)
            ));
        }
    }
    #[test]
    fn layout_factories_create_real_typed_storage_after_source_deletion() {
        let fixture = Fixture::new();
        let out = fixture.run();
        let key = snap(&out, &[Field(1), Field(0)]);
        let value = snap(&out, &[Field(1)]);
        let map = GenericCollectionLayout::map(
            key.ty(),
            value.ty(),
            GenericTypeEquivalenceLimits::DEFAULT,
        )
        .unwrap();
        let set =
            GenericCollectionLayout::set(key.ty(), GenericTypeEquivalenceLimits::DEFAULT).unwrap();
        drop(out);
        std::fs::remove_dir_all(&fixture.0).unwrap();
        let mut f = fuel(1, 0);
        let empty = map.instantiate_map(&mut f).unwrap();
        assert!(empty.is_empty());
        assert_eq!(f.remaining(), fuel(0, 0).remaining());
        assert!(empty
            .value_type()
            .equivalent(&value.ty(), GenericTypeEquivalenceLimits::DEFAULT)
            .unwrap());
        let filled = empty
            .insert(&RuntimeKey::Int(1), &value, &mut unlimited(), &mut work())
            .unwrap();
        let got = filled
            .get(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
            .unwrap()
            .unwrap();
        assert!(eq(&got, &value));
        let mut f = fuel(1, 0);
        let empty = set.instantiate_set(&mut f).unwrap();
        assert_eq!(empty.key_type(), RuntimeKeyType::Int);
        assert_eq!(f.remaining(), fuel(0, 0).remaining());
        let filled = empty
            .insert(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
            .unwrap();
        assert!(filled.contains(&RuntimeKey::Int(1), &mut work()).unwrap());
    }
    #[test]
    fn constructor_kind_and_materialization_refusals_do_not_commit_fuel() {
        let out = Fixture::new().run();
        let key = snap(&out, &[Field(1), Field(0)]);
        let value = snap(&out, &[Field(1)]);
        let map = GenericCollectionLayout::map(
            key.ty(),
            value.ty(),
            GenericTypeEquivalenceLimits::DEFAULT,
        )
        .unwrap();
        let set =
            GenericCollectionLayout::set(key.ty(), GenericTypeEquivalenceLimits::DEFAULT).unwrap();
        let mut f = fuel(1, 0);
        assert!(matches!(
            map.instantiate_set(&mut f),
            Err(GenericCollectionLayoutError::Kind)
        ));
        assert!(matches!(
            set.instantiate_map(&mut f),
            Err(GenericCollectionLayoutError::Kind)
        ));
        assert_eq!(f.remaining(), fuel(1, 0).remaining());
        let mut f = fuel(0, 0);
        assert!(matches!(
            map.instantiate_map(&mut f),
            Err(GenericCollectionLayoutError::Map(
                GenericCollectionMapError::Value(GenericCollectionError::Materialization(
                    RuntimeValueError::NodeLimit
                ))
            ))
        ));
        assert!(matches!(
            set.instantiate_set(&mut f),
            Err(GenericCollectionLayoutError::Set(
                RuntimeSetError::Materialization(RuntimeValueError::NodeLimit)
            ))
        ));
        assert_eq!(f.remaining(), fuel(0, 0).remaining());
    }
    #[test]
    fn retained_type_profiles_refuse_exhaustion_instead_of_publishing_partial_plans() {
        let out = Fixture::new().run();
        let key = snap(&out, &[Field(1), Field(0)]);
        let value = snap(&out, &[Field(1)]);
        let mut limits = GenericTypeEquivalenceLimits::DEFAULT;
        limits.max_pairs = 0;
        assert!(matches!(
            GenericCollectionLayout::set(key.ty(), limits),
            Err(GenericCollectionLayoutError::Type(_))
        ));
        limits = GenericTypeEquivalenceLimits::DEFAULT;
        limits.max_edges = 0;
        assert!(GenericCollectionLayout::set(key.ty(), limits).is_ok());
        assert!(matches!(
            GenericCollectionLayout::map(key.ty(), value.ty(), limits),
            Err(GenericCollectionLayoutError::Type(_))
        ));
    }
    #[test]
    fn primitive_key_domains_are_exact_and_portable_across_fresh_checks() {
        let f = Fixture::new();
        let mut plans = Vec::new();
        for source in [
            "fn main()->Int{return 1;}",
            "fn main()->Bool{return true;}",
            "fn main()->String{return \"λ\";}",
        ] {
            f.write("app/main.svr", source);
            let key = snap(&f.run(), &[]);
            let plan =
                GenericCollectionLayout::set(key.ty(), GenericTypeEquivalenceLimits::DEFAULT)
                    .unwrap();
            let fresh = snap(&f.run(), &[]);
            assert!(plan
                .equivalent(
                    &GenericCollectionLayout::set(
                        fresh.ty(),
                        GenericTypeEquivalenceLimits::DEFAULT
                    )
                    .unwrap()
                )
                .unwrap());
            plans.push(plan);
        }
        assert_eq!(plans[0].key_domain(), RuntimeKeyType::Int);
        assert_eq!(plans[1].key_domain(), RuntimeKeyType::Bool);
        assert_eq!(plans[2].key_domain(), RuntimeKeyType::String);
        assert!(!plans[0].equivalent(&plans[1]).unwrap());
        assert!(!plans[1].equivalent(&plans[2]).unwrap());
    }
    #[test]
    fn source_intrinsics_stay_refused_until_executable_layout_and_ir_are_qualified() {
        let fixture = Fixture::new();
        for name in ["std::Map<Int,Int>", "std::Set<Int>"] {
            fixture.write(
                "app/main.svr",
                &format!("fn unused(value:{name}){{}}fn main(){{}}"),
            );
            assert!(compile_generic_package(
                capture_package_sources(fixture.0.join("app"), PackageCaptureLimits::DEFAULT)
                    .unwrap(),
                "main",
                GenericPackageCompilationLimits::DEFAULT
            )
            .is_err());
        }
    }
}

#[test]
fn same_spelled_user_map_records_keep_ordinary_record_layouts() {
    let f = Fixture::new();
    f.write("app/main.svr","struct Map<K,V>{key:K,value:V}fn main()->Map<Int,String>{return Map::<Int,String>{value:\"ordinary\",key:1};}");
    let out = f.run();
    let ordinary = snap(&out, &[]);
    let key = snap(&out, &[Field(0)]);
    assert!(matches!(
        ordinary.ty().layout().kind(),
        GenericRuntimeLayoutKind::Record { .. }
    ));
    assert_eq!(
        ordinary.value().field(1).unwrap().as_str(),
        Some("ordinary")
    );
    assert!(matches!(
        GenericCollectionLayout::set(ordinary.ty(), GenericTypeEquivalenceLimits::DEFAULT),
        Err(GenericCollectionLayoutError::KeyBound)
    ));
    let plan = GenericCollectionLayout::map(
        key.ty(),
        ordinary.ty(),
        GenericTypeEquivalenceLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        plan.argument(1).unwrap().declaration(),
        ordinary.ty().declaration()
    );
    let empty = plan.instantiate_map(&mut unlimited()).unwrap();
    let map = empty
        .insert(
            &RuntimeKey::Int(1),
            &ordinary,
            &mut unlimited(),
            &mut work(),
        )
        .unwrap();
    let value = map
        .get(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
        .unwrap()
        .unwrap();
    assert!(eq(&value, &ordinary));
}

mod collection_semantics {
    use super::*;
    fn table(f: &Fixture) -> GenericPackageDeclarations {
        bind_generic_package_declarations(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            GenericPackageDeclarationLimits::DEFAULT,
        )
        .unwrap()
    }
    fn check(
        f: &Fixture,
    ) -> Result<CheckedGenericPackageTypes, sovra::compiler::diagnostics::Diagnostics> {
        check_generic_collection_package_types(table(f), GenericPackageTypeLimits::DEFAULT)
    }
    #[test]
    fn opt_in_source_annotations_register_opaque_headers_not_fake_records() {
        let f = Fixture::new();
        f.write("app/main.svr","use dep::types;type Lookup<K:std::Key,V>=std::Map<K,V>;struct Envelope<K:std::Key,V>{map:Lookup<K,V>,set:std::Set<K>}enum Choice<T>{Maps(std::Map<String,T>),Empty}fn unused(x:Envelope<Int,dep::types::Box<Int>>)->std::Set<Bool>{}fn main(){}");
        let checked = check(&f).unwrap();
        let definitions = checked.types();
        let headers = definitions.environment().registry().headers();
        for (name, kind, count) in [
            ("std::Map", GenericCollectionLayoutKind::Map, 2),
            ("std::Set", GenericCollectionLayoutKind::Set, 1),
        ] {
            let header = headers.iter().find(|h| h.name.as_ref() == name).unwrap();
            assert_eq!(header.kind, GenericTypeDeclarationKind::Collection);
            assert_eq!(header.parameters.parameters().len(), count);
            assert_eq!(
                header.parameters.parameters()[0].bound,
                Some(GenericParameterBound::Key)
            );
            let Some(CheckedGenericDefinition::Collection(c)) = definitions.definition(header.id)
            else {
                panic!("opaque collection definition missing")
            };
            assert_eq!(c.id(), header.id);
            assert_eq!(c.syntax().kind(), kind);
            assert_eq!(c.syntax().qualified_name(), name);
        }
    }
    #[test]
    fn aliases_nested_collections_and_symbolic_key_bounds_use_existing_resolution() {
        let f = Fixture::new();
        for key in ["Int", "Bool", "String"] {
            f.write("app/main.svr",&format!("type Key<T>=T;fn unused(x:std::Map<Key<{key}>,[std::Set<String>]>){{}}fn main(){{}}"));
            check(&f).unwrap();
        }
        f.write("app/main.svr","type Lookup<K:std::Key,V>=std::Map<K,V>;fn pass<K:std::Key,V>(x:Lookup<K,V>)->std::Map<K,V>{return x;}fn main(){}");
        let bodies = check_generic_package_bodies(
            check(&f).unwrap(),
            GenericCopyBodyLimits::PACKAGE,
            GenericPropagationLimits::DEFAULT,
        );
        assert!(bodies.is_ok(), "{bodies:?}");
        f.write(
            "app/main.svr",
            "fn unbounded<K,V>(x:std::Map<K,V>){}fn main(){}",
        );
        assert!(check(&f).is_err());
    }
    #[test]
    fn invalid_keys_and_wrong_collection_arities_fail_in_original_source() {
        let f = Fixture::new();
        for annotation in [
            "std::Map<Int>",
            "std::Map<Int,Int,Int>",
            "std::Set<Int,Int>",
            "std::Map",
            "std::Set",
            "std::Map<Float,Int>",
            "std::Set<[Int]>",
            "std::Map<dep::types::Box<Int>,Int>",
            "std::Set<Unit>",
            "std::Set<std::Set<Int>>",
        ] {
            f.write(
                "app/main.svr",
                &format!("use dep::types;fn unused(x:{annotation}){{}}fn main(){{}}"),
            );
            let errors = check(&f).unwrap_err();
            assert!(
                errors.items.iter().any(|d| d
                    .source_file
                    .as_ref()
                    .is_some_and(|p| p.ends_with("app/main.svr"))),
                "{annotation}: {errors:?}"
            );
        }
    }
    #[test]
    fn collection_arguments_do_not_bypass_dependency_visibility() {
        let f = Fixture::new();
        f.write(
            "dep/types.svr",
            "mod types{struct Secret<T>{value:T}export struct Box<T>{value:T}}",
        );
        f.write(
            "app/main.svr",
            "use dep::types;fn private(x:std::Map<Int,dep::types::Secret<Int>>){}fn main(){}",
        );
        assert!(check(&f).is_err());
        f.write(
            "app/main.svr",
            "use dep::types;fn public_value(x:std::Map<Int,dep::types::Box<Int>>){}fn main(){}",
        );
        check(&f).unwrap();
    }
    #[test]
    fn opaque_record_construction_and_field_projection_stay_refused() {
        let f = Fixture::new();
        for source in [
            "fn wrong()->std::Map<Int,Int>{return std::Map::<Int,Int>{};}fn main(){}",
            "fn wrong(value:std::Map<Int,Int>)->Int{return value.fake;}fn main(){}",
            "fn wrong()->std::Set<Int>{return std::Set::<Int>{};}fn main(){}",
        ] {
            f.write("app/main.svr", source);
            let types = check(&f).unwrap();
            assert!(check_generic_package_bodies(
                types,
                GenericCopyBodyLimits::PACKAGE,
                GenericPropagationLimits::DEFAULT
            )
            .is_err());
        }
    }
    #[test]
    fn new_intrinsic_headers_consume_cumulative_registry_admission_only_in_opt_in_profile() {
        let f = Fixture::new();
        f.write("app/main.svr", "fn main(){}");
        let old =
            check_generic_package_types(table(&f), GenericPackageTypeLimits::DEFAULT).unwrap();
        let n = old.types().environment().registry().headers().len();
        let new = check(&f).unwrap();
        assert_eq!(new.types().environment().registry().headers().len(), n + 3);
        let mut limits = GenericPackageTypeLimits::DEFAULT;
        limits.registry.max_declarations = n + 3;
        assert!(check_generic_collection_package_types(table(&f), limits).is_ok());
        limits.registry.max_declarations = n + 2;
        assert!(check_generic_collection_package_types(table(&f), limits).is_err());
        assert!(check_generic_package_types(table(&f), limits).is_ok());
        limits.registry.max_declarations = n + 3;
        let parameters = old
            .types()
            .environment()
            .registry()
            .headers()
            .iter()
            .map(|h| h.parameters.parameters().len())
            .sum::<usize>();
        limits.registry.max_parameters = parameters + 5;
        assert!(check_generic_collection_package_types(table(&f), limits).is_ok());
        limits.registry.max_parameters = parameters + 4;
        assert!(check_generic_collection_package_types(table(&f), limits).is_err());
    }
    #[test]
    fn default_compile_and_native_profile_do_not_silently_admit_new_type_features() {
        let f = Fixture::new();
        f.write(
            "app/main.svr",
            "fn unused(value:std::Map<Int,Int>){}fn main(){}",
        );
        check(&f).unwrap();
        assert!(check_generic_package_types(table(&f), GenericPackageTypeLimits::DEFAULT).is_err());
        assert!(compile_generic_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT
        )
        .is_err());
    }
}

#[test]
fn semantic_collection_identity_forwarding_rechecks_but_executable_layouts_refuse() {
    let f = Fixture::new();
    f.write(
        "app/main.svr",
        "fn main(value:std::Map<Int,Int>)->std::Map<Int,Int>{return value;}",
    );
    let table = bind_generic_package_declarations(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        GenericPackageDeclarationLimits::DEFAULT,
    )
    .unwrap();
    let types =
        check_generic_collection_package_types(table, GenericPackageTypeLimits::DEFAULT).unwrap();
    let root = types
        .signatures()
        .signatures()
        .iter()
        .find(|s| s.syntax().name() == "main")
        .unwrap()
        .id();
    let bodies = check_generic_package_bodies(
        types,
        GenericCopyBodyLimits::PACKAGE,
        GenericPropagationLimits::DEFAULT,
    )
    .unwrap();
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
    )
    .unwrap();
    let literals = decode_generic_literals(&concrete, GenericLiteralLimits::DEFAULT).unwrap();
    assert!(matches!(
        plan_generic_layouts(&literals, GenericLayoutLimits::DEFAULT),
        Err(GenericLayoutError::Type(
            "intrinsic collection execution is not enabled"
        ))
    ));
}
#[test]
fn intrinsic_definition_parameter_owners_cannot_be_replayed_from_fresh_headers() {
    let a = standard_generic_collection_types().unwrap();
    let b = standard_generic_collection_types().unwrap();
    let inputs = a
        .iter()
        .map(GenericTypeDeclaration::Collection)
        .collect::<Vec<_>>();
    let foreign = b
        .iter()
        .map(GenericTypeDeclaration::Collection)
        .collect::<Vec<_>>();
    let registry = register_generic_types(&inputs, GenericRegistryLimits::DEFAULT).unwrap();
    let env =
        prepare_generic_aliases(&registry, &[], GenericAliasEnvironmentLimits::DEFAULT).unwrap();
    check_generic_declarations(&env, &inputs, GenericDeclarationLimits::DEFAULT).unwrap();
    assert!(matches!(
        check_generic_declarations(&env, &foreign, GenericDeclarationLimits::DEFAULT),
        Err(GenericDeclarationError::Ownership { .. })
    ));
}

mod executable_collection_layouts {
    use super::*;
    fn literals(source: &str) -> CheckedGenericLiterals {
        let f = Fixture::new();
        f.write("app/main.svr", source);
        let table = bind_generic_package_declarations(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            GenericPackageDeclarationLimits::DEFAULT,
        )
        .unwrap();
        let types =
            check_generic_collection_package_types(table, GenericPackageTypeLimits::DEFAULT)
                .unwrap();
        let root = types
            .signatures()
            .signatures()
            .iter()
            .find(|s| s.syntax().name() == "main")
            .unwrap()
            .id();
        let bodies = check_generic_package_bodies(
            types,
            GenericCopyBodyLimits::PACKAGE,
            GenericPropagationLimits::DEFAULT,
        )
        .unwrap();
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
        )
        .unwrap();
        decode_generic_literals(&concrete, GenericLiteralLimits::DEFAULT).unwrap()
    }
    #[test]
    fn opaque_map_set_shapes_bind_exact_argument_edges_without_fake_members() {
        let input = literals("fn main(a:std::Map<Int,String>,b:std::Set<Bool>){}");
        let layouts =
            plan_generic_collection_layouts(&input, GenericLayoutLimits::DEFAULT).unwrap();
        assert!(layouts.has_collections());
        let args = layouts.bodies()[0].parameter_layouts();
        let GenericRuntimeLayoutKind::Map {
            declaration,
            key,
            value,
        } = layouts.layout(args[0]).unwrap().kind()
        else {
            panic!("map shape missing")
        };
        assert_eq!(
            layouts.layout(*key).unwrap().ty().primitive(),
            Some(GenericPrimitive::Int)
        );
        assert_eq!(
            layouts.layout(*value).unwrap().ty().primitive(),
            Some(GenericPrimitive::String)
        );
        assert!(
            matches!(input.concrete().graph().bodies().signatures().types().definition(*declaration),Some(CheckedGenericDefinition::Collection(c))if c.syntax().kind()==GenericCollectionLayoutKind::Map)
        );
        let GenericRuntimeLayoutKind::Set { key, .. } = layouts.layout(args[1]).unwrap().kind()
        else {
            panic!("set shape missing")
        };
        assert_eq!(
            layouts.layout(*key).unwrap().ty().primitive(),
            Some(GenericPrimitive::Bool)
        );
        assert_eq!(layouts.usage().members, 0);
        assert_eq!(layouts.usage().instantiations, 3);
        assert_eq!(layouts.usage().edges, 3);
        let mut zero_members = GenericLayoutLimits::DEFAULT;
        zero_members.max_members = 0;
        plan_generic_collection_layouts(&input, zero_members).unwrap();
    }
    #[test]
    fn aliases_deduplicate_but_changed_arguments_and_constructor_kinds_do_not() {
        let input=literals("type M<K:std::Key,V>=std::Map<K,V>;fn main(a:std::Map<Int,String>,b:M<Int,String>,c:std::Map<String,Int>,d:std::Set<Int>){}");
        let layouts =
            plan_generic_collection_layouts(&input, GenericLayoutLimits::DEFAULT).unwrap();
        let args = layouts.bodies()[0].parameter_layouts();
        assert_eq!(args[0], args[1]);
        assert_ne!(args[0], args[2]);
        assert_ne!(args[0], args[3]);
        let GenericRuntimeLayoutKind::Map { declaration: a, .. } =
            layouts.layout(args[0]).unwrap().kind()
        else {
            panic!()
        };
        let GenericRuntimeLayoutKind::Map { declaration: b, .. } =
            layouts.layout(args[2]).unwrap().kind()
        else {
            panic!()
        };
        let GenericRuntimeLayoutKind::Set { declaration: c, .. } =
            layouts.layout(args[3]).unwrap().kind()
        else {
            panic!()
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
    #[test]
    fn phantom_value_type_arguments_remain_in_collection_layout_identity() {
        let input=literals("use dep::types;fn main(a:std::Map<Int,dep::types::Empty<Int>>,b:std::Map<Int,dep::types::Empty<String>>){}");
        let layouts =
            plan_generic_collection_layouts(&input, GenericLayoutLimits::DEFAULT).unwrap();
        let args = layouts.bodies()[0].parameter_layouts();
        assert_ne!(args[0], args[1]);
        let GenericRuntimeLayoutKind::Map { value: a, .. } =
            layouts.layout(args[0]).unwrap().kind()
        else {
            panic!()
        };
        let GenericRuntimeLayoutKind::Map { value: b, .. } =
            layouts.layout(args[1]).unwrap().kind()
        else {
            panic!()
        };
        assert_ne!(a, b);
        let GenericRuntimeLayoutKind::Record {
            declaration: ad,
            fields: af,
            ..
        } = layouts.layout(*a).unwrap().kind()
        else {
            panic!()
        };
        let GenericRuntimeLayoutKind::Record {
            declaration: bd,
            fields: bf,
            ..
        } = layouts.layout(*b).unwrap().kind()
        else {
            panic!()
        };
        assert_eq!(ad, bd);
        assert!(af.is_empty() && bf.is_empty());
    }
    #[test]
    fn recursive_map_value_edges_close_the_existing_nominal_layout_graph() {
        let input=literals("struct Node<T>{children:std::Map<String,Node<T>>}fn main(value:Node<Int>)->Node<Int>{return value;}");
        let layouts =
            plan_generic_collection_layouts(&input, GenericLayoutLimits::DEFAULT).unwrap();
        let node = layouts.bodies()[0].parameter_layouts()[0];
        let GenericRuntimeLayoutKind::Record { fields, .. } = layouts.layout(node).unwrap().kind()
        else {
            panic!()
        };
        assert_eq!(fields.len(), 1);
        let GenericRuntimeLayoutKind::Map { key, value, .. } =
            layouts.layout(fields[0]).unwrap().kind()
        else {
            panic!()
        };
        assert_eq!(*value, node);
        assert_eq!(
            layouts.layout(*key).unwrap().ty().primitive(),
            Some(GenericPrimitive::String)
        );
        assert_eq!(layouts.layouts().len(), 4);
        assert_eq!(layouts.usage().instantiations, 3);
        assert_eq!(layouts.usage().members, 1);
    }
    #[test]
    fn expanding_generic_recursion_through_map_values_refuses_at_shared_instantiation_cap() {
        let input =
            literals("struct Grow<T>{next:std::Map<Int,Grow<[T]>>}fn main(value:Grow<Int>){}");
        let mut limits = GenericLayoutLimits::DEFAULT;
        limits.max_instantiations = 12;
        assert!(matches!(
            plan_generic_collection_layouts(&input, limits),
            Err(GenericLayoutError::Limit("layout instantiations"))
        ));
    }
    #[test]
    fn exact_cumulative_layout_limits_pass_and_one_under_refuses_without_partial_table() {
        let input = literals(
            "fn main(a:std::Map<Int,std::Set<String>>)->std::Map<Int,std::Set<String>>{return a;}",
        );
        let planned =
            plan_generic_collection_layouts(&input, GenericLayoutLimits::DEFAULT).unwrap();
        let u = planned.usage();
        let exact = GenericLayoutLimits {
            max_instantiations: u.instantiations,
            max_layouts: u.layouts,
            max_members: u.members,
            max_edges: u.edges,
            max_references: u.references,
            max_generated_nodes: u.generated_nodes,
            max_generated_edges: u.generated_edges,
            max_key_nodes: u.key_nodes,
            max_key_edges: u.key_edges,
            max_work: u.work,
            ..GenericLayoutLimits::DEFAULT
        };
        assert_eq!(
            plan_generic_collection_layouts(&input, exact)
                .unwrap()
                .usage(),
            u
        );
        for dimension in 0..10 {
            let mut small = exact;
            let n = match dimension {
                0 => &mut small.max_instantiations,
                1 => &mut small.max_layouts,
                2 => &mut small.max_members,
                3 => &mut small.max_edges,
                4 => &mut small.max_references,
                5 => &mut small.max_generated_nodes,
                6 => &mut small.max_generated_edges,
                7 => &mut small.max_key_nodes,
                8 => &mut small.max_key_edges,
                _ => &mut small.max_work,
            };
            if *n == 0 {
                continue;
            }
            *n -= 1;
            assert!(
                plan_generic_collection_layouts(&input, small).is_err(),
                "dimension {dimension}"
            );
        }
    }
    #[test]
    fn opaque_features_in_phantom_arguments_cannot_escape_the_ir_operation_gate() {
        let input = literals("struct Empty<T>{}fn main(value:Empty<std::Map<Int,Int>>){}");
        assert!(plan_generic_layouts(&input, GenericLayoutLimits::DEFAULT).is_err());
        let layouts =
            plan_generic_collection_layouts(&input, GenericLayoutLimits::DEFAULT).unwrap();
        assert!(layouts.has_collections());
        assert!(matches!(
            plan_generic_construction(&layouts, GenericConstructionLimits::DEFAULT),
            Err(GenericConstructionError::CollectionOperationsUnavailable)
        ));
        let propagation =
            plan_generic_propagation_layouts(&layouts, GenericPropagationLayoutLimits::DEFAULT)
                .unwrap();
        assert!(matches!(
            lower_generic_propagation_ir(
                &propagation,
                GenericConstructionLimits::DEFAULT,
                GenericIrLimits::DEFAULT
            ),
            Err(GenericIrError::Construction(
                GenericConstructionError::CollectionOperationsUnavailable
            ))
        ));
    }
    #[test]
    fn collection_free_layouts_keep_usage_and_execution_unchanged_in_opt_in_planner() {
        let input = literals("fn main()->Int{return 42;}");
        let old = plan_generic_layouts(&input, GenericLayoutLimits::DEFAULT).unwrap();
        let new = plan_generic_collection_layouts(&input, GenericLayoutLimits::DEFAULT).unwrap();
        assert!(!old.has_collections());
        assert!(!new.has_collections());
        assert_eq!(old.usage(), new.usage());
        let plan = plan_generic_construction(&new, GenericConstructionLimits::DEFAULT).unwrap();
        let ir = lower_generic_ir(&plan, GenericIrLimits::DEFAULT).unwrap();
        assert_eq!(
            execute_generic_ir(&ir, GenericVmPolicy::DEFAULT)
                .unwrap()
                .value()
                .as_int(),
            Some(42)
        );
    }
}

mod compiler_storage_layout_bridge {
    use super::*;
    fn literals(bodies: &CheckedGenericPackageBodies, name: &str) -> CheckedGenericLiterals {
        let root = bodies
            .types()
            .signatures()
            .signatures()
            .iter()
            .find(|s| s.syntax().name() == name)
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
        )
        .unwrap();
        decode_generic_literals(&concrete, GenericLiteralLimits::DEFAULT).unwrap()
    }
    fn run(layouts: &CheckedGenericLayouts) -> GenericVmOutcome {
        let plans = plan_generic_construction(layouts, GenericConstructionLimits::DEFAULT).unwrap();
        let ir = lower_generic_ir(&plans, GenericIrLimits::DEFAULT).unwrap();
        execute_generic_ir(&ir, GenericVmPolicy::DEFAULT).unwrap()
    }
    #[test]
    fn actual_map_layout_binds_storage_and_accepts_same_owner_values_from_other_layout_tables() {
        let f = Fixture::new();
        f.write("app/main.svr","use dep::types;struct Box<T>{value:T}fn main(value:std::Map<Int,dep::types::Box<Int>>){}fn make()->dep::types::Box<Int>{return dep::types::Box::<Int>{value:7};}fn local()->Box<Int>{return Box::<Int>{value:7};}");
        let table = bind_generic_package_declarations(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            GenericPackageDeclarationLimits::DEFAULT,
        )
        .unwrap();
        let types =
            check_generic_collection_package_types(table, GenericPackageTypeLimits::DEFAULT)
                .unwrap();
        let bodies = check_generic_package_bodies(
            types,
            GenericCopyBodyLimits::PACKAGE,
            GenericPropagationLimits::DEFAULT,
        )
        .unwrap();
        let main = plan_generic_collection_layouts(
            &literals(&bodies, "main"),
            GenericLayoutLimits::DEFAULT,
        )
        .unwrap();
        let root = main.bodies()[0].parameter_layouts()[0];
        let GenericRuntimeLayoutKind::Map {
            value: expected, ..
        } = main.layout(root).unwrap().kind()
        else {
            panic!()
        };
        let layout = GenericCollectionLayout::from_checked_layout(
            &main,
            root,
            GenericTypeEquivalenceLimits::DEFAULT,
        )
        .unwrap();
        let other =
            plan_generic_layouts(&literals(&bodies, "make"), GenericLayoutLimits::DEFAULT).unwrap();
        let actual = snap(&run(&other), &[]);
        assert_ne!(*expected, actual.value().layout());
        let root_type = GenericCollectionType::from_layout(&main, root).unwrap();
        assert!(root_type.declaration().is_some());
        assert!(!root_type.is_key());
        let empty = layout.instantiate_map(&mut unlimited()).unwrap();
        let map = empty
            .insert(&RuntimeKey::Int(1), &actual, &mut unlimited(), &mut work())
            .unwrap();
        assert!(eq(
            &map.get(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
                .unwrap()
                .unwrap(),
            &actual
        ));
        let local = plan_generic_layouts(&literals(&bodies, "local"), GenericLayoutLimits::DEFAULT)
            .unwrap();
        let forged = snap(&run(&local), &[]);
        assert_eq!(
            empty
                .insert(&RuntimeKey::Int(1), &forged, &mut fuel(0, 0), &mut work())
                .unwrap_err(),
            GenericCollectionMapError::ValueType
        );
    }
    #[test]
    fn checked_set_shapes_create_their_exact_key_domain_and_bad_indices_or_record_shapes_refuse() {
        let f = Fixture::new();
        f.write(
            "app/main.svr",
            "struct Map<K,V>{key:K,value:V}fn main(value:std::Set<String>){}fn plain()->Map<Int,String>{return Map::<Int,String>{key:1,value:\"plain\"};}",
        );
        let table = bind_generic_package_declarations(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            GenericPackageDeclarationLimits::DEFAULT,
        )
        .unwrap();
        let types =
            check_generic_collection_package_types(table, GenericPackageTypeLimits::DEFAULT)
                .unwrap();
        let bodies = check_generic_package_bodies(
            types,
            GenericCopyBodyLimits::PACKAGE,
            GenericPropagationLimits::DEFAULT,
        )
        .unwrap();
        let layouts = plan_generic_collection_layouts(
            &literals(&bodies, "main"),
            GenericLayoutLimits::DEFAULT,
        )
        .unwrap();
        let root = layouts.bodies()[0].parameter_layouts()[0];
        let plan = GenericCollectionLayout::from_checked_layout(
            &layouts,
            root,
            GenericTypeEquivalenceLimits::DEFAULT,
        )
        .unwrap();
        assert_eq!(plan.kind(), GenericCollectionLayoutKind::Set);
        let mut f = fuel(1, 0);
        let set = plan.instantiate_set(&mut f).unwrap();
        assert_eq!(set.key_type(), RuntimeKeyType::String);
        assert_eq!(f.remaining(), fuel(0, 0).remaining());
        assert!(matches!(
            GenericCollectionType::from_layout(&layouts, usize::MAX),
            Err(GenericCollectionError::Invalid)
        ));
        assert!(GenericCollectionLayout::from_checked_layout(
            &layouts,
            usize::MAX,
            GenericTypeEquivalenceLimits::DEFAULT
        )
        .is_err());
        let plain = plan_generic_layouts(&literals(&bodies, "plain"), GenericLayoutLimits::DEFAULT)
            .unwrap();
        assert!(matches!(
            GenericCollectionLayout::from_checked_layout(
                &plain,
                plain.bodies()[0].return_layout(),
                GenericTypeEquivalenceLimits::DEFAULT
            ),
            Err(GenericCollectionLayoutError::Kind)
        ));
    }
}

mod collection_execution {
    use super::*;
    fn compile(
        f: &Fixture,
    ) -> Result<CompiledGenericPackage, sovra::compiler::diagnostics::Diagnostics> {
        compile_generic_collection_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
    }
    fn run(source: &str) -> GenericVmOutcome {
        let f = Fixture::new();
        f.write("app/main.svr", source);
        execute_generic_collection_package(&compile(&f).unwrap(), GenericVmPolicy::DEFAULT).unwrap()
    }
    #[test]
    fn real_source_creates_empty_map_and_set_values_with_exact_retained_layouts() {
        let map = run("fn main()->std::Map<Int,String>{return std::map_new();}");
        assert_eq!(map.value().map_len(), Some(0));
        assert_eq!(map.value().set_len(), None);
        assert_eq!(map.value().footprint(), (1, 0));
        assert!(matches!(
            map.ir()
                .plans()
                .layouts()
                .layout(map.value().layout())
                .unwrap()
                .kind(),
            GenericRuntimeLayoutKind::Map { .. }
        ));
        let set = run("fn main()->std::Set<Bool>{return std::set_new();}");
        assert_eq!(set.value().set_len(), Some(0));
        assert_eq!(set.value().map_len(), None);
        assert_eq!(set.value().footprint(), (1, 0));
    }
    #[test]
    fn source_length_queries_execute_and_materialize_real_int_results() {
        for source in [
            "fn main()->Int{let m:std::Map<Int,String>=std::map_new();return std::map_len(m);}",
            "fn main()->Int{let s:std::Set<String>=std::set_new();return std::set_len(s);}",
        ] {
            let out = run(source);
            assert_eq!(out.value().as_int(), Some(0));
        }
    }
    #[test]
    fn aliases_symbolic_factories_and_cross_file_calls_preserve_collection_types() {
        let f = Fixture::new();
        f.write(
            "dep/types.svr",
            "mod types{export fn empty<K:std::Key,V>()->std::Map<K,V>{return std::map_new();}}",
        );
        f.write("app/main.svr","use dep::types;type Lookup<K:std::Key,V>=std::Map<K,V>;fn main()->Int{let m:Lookup<Int,String>=dep::types::empty::<Int,String>();return std::map_len(m);}");
        let p = compile(&f).unwrap();
        assert_eq!(
            execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT)
                .unwrap()
                .value()
                .as_int(),
            Some(0)
        );
    }
    #[test]
    fn arrays_records_mutable_slots_and_borrowed_snapshots_retain_empty_collections() {
        let out=run("struct Box<T>{value:T}fn main()->Int{let a:std::Map<Int,String>=std::map_new();let b:std::Map<Int,String>=std::map_new();let mut current=a;let old=current;current=b;let list:[std::Map<Int,String>]=[old,current];let wrapper=Box::<[std::Map<Int,String>]>{value:list};return std::map_len(wrapper.value[0])+std::array_len(wrapper.value);}");
        assert_eq!(out.value().as_int(), Some(2));
    }
    #[test]
    fn empty_collection_equality_and_map_valued_propagation_follow_existing_control_flow() {
        let out=run("fn maybe()->std::Option<std::Map<Int,Int>>{return std::Option::<std::Map<Int,Int>>::Some(std::map_new());}fn check()->std::Option<Bool>{let a=maybe()?;let b:std::Map<Int,Int>=std::map_new();return std::Option::<Bool>::Some(a==b);}fn main()->std::Option<Bool>{return check();}");
        assert_eq!(out.value().enum_tag(), Some(0));
        assert_eq!(out.value().payload().unwrap().as_bool(), Some(true));
    }
    #[test]
    fn exact_creation_and_length_node_budgets_admit_and_one_under_refuses() {
        let f = Fixture::new();
        f.write(
            "app/main.svr",
            "fn main()->Int{let m:std::Map<Int,String>=std::map_new();return std::map_len(m);}",
        );
        let p = compile(&f).unwrap();
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.runtime.materialization.max_value_nodes = Some(3);
        policy.runtime.materialization.max_text_bytes = Some(0);
        let out = execute_generic_collection_package(&p, policy.clone()).unwrap();
        assert_eq!(out.remaining().materialization.max_value_nodes, Some(0));
        assert_eq!(out.value().as_int(), Some(0));
        policy.runtime.materialization.max_value_nodes = Some(2);
        assert!(execute_generic_collection_package(&p, policy).is_err());
        assert_eq!(
            execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT)
                .unwrap()
                .value()
                .as_int(),
            Some(0)
        );
    }
    #[test]
    fn shared_instruction_work_and_cancellation_limits_apply_before_collection_success() {
        use sovra::compiler::execution::{CancellationToken, ExecutionControl};
        let f = Fixture::new();
        f.write(
            "app/main.svr",
            "fn main()->Int{let s:std::Set<Int>=std::set_new();return std::set_len(s);}",
        );
        let p = compile(&f).unwrap();
        let out = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        let mut limited = GenericVmPolicy::DEFAULT;
        limited.runtime.execution.max_instructions = Some(out.instructions() - 1);
        assert!(execute_generic_collection_package(&p, limited).is_err());
        let mut work = GenericVmPolicy::DEFAULT;
        work.max_work = 0;
        assert!(execute_generic_collection_package(&p, work).is_err());
        let token = CancellationToken::new();
        token.cancel();
        let mut cancelled = GenericVmPolicy::DEFAULT;
        cancelled.control = ExecutionControl {
            cancellation: Some(token),
            max_elapsed: None,
        };
        assert!(execute_generic_collection_package(&p, cancelled).is_err());
    }
    #[test]
    fn missing_context_wrong_kind_arity_and_record_impostors_fail_before_ir() {
        let f = Fixture::new();
        for source in [
            "fn main(){let m=std::map_new();}",
            "fn main()->std::Map<Int,Int>{return std::set_new();}",
            "fn main()->Int{return std::map_len(1);}",
            "fn main()->Int{let s:std::Set<Int>=std::set_new();return std::map_len(s);}",
            "fn main()->std::Map<Int,Int>{return std::map_new(1);}",
            "struct Map<K,V>{}fn main()->Map<Int,Int>{return std::map_new();}",
        ] {
            f.write("app/main.svr", source);
            assert!(compile(&f).is_err(), "{source}");
        }
    }
    #[test]
    fn old_execution_and_native_encoding_refuse_the_new_profile_even_for_unused_annotations() {
        let f = Fixture::new();
        for source in [
            "fn main(){let m:std::Map<Int,Int>=std::map_new();}",
            "fn unused(value:std::Map<Int,Int>){}fn main(){}",
        ] {
            f.write("app/main.svr", source);
            let p = compile(&f).unwrap();
            assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
            assert_eq!(
                p.encode_generic_package_native_image().unwrap_err(),
                "native collection feature is not enabled"
            );
            assert!(compile_generic_package(
                capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
                "main",
                GenericPackageCompilationLimits::DEFAULT
            )
            .is_err());
            execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        }
    }
    #[test]
    fn wrong_collection_removal_result_refuses_and_empty_display_is_defined() {
        let f = Fixture::new();
        f.write(
            "app/main.svr",
            "fn main()->Int{let m:std::Map<Int,Int>=std::map_new();return std::map_remove(m,1);}",
        );
        assert!(compile(&f).is_err());
        f.write(
            "app/main.svr",
            "fn main()->String{let m:std::Map<Int,Int>=std::map_new();return std::to_string(m);}",
        );
        let p = compile(&f).unwrap();
        let result = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(result.value().as_str(), Some("std::Map { }"));
    }
}

#[test]
fn published_empty_collection_source_runs_only_with_explicit_collection_profile() {
    let f = Fixture::new();
    let source = std::fs::read_to_string("docs/examples/intrinsic-empty-collections.svr").unwrap();
    f.write("app/main.svr", &source);
    let p = compile_generic_collection_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    assert!(
        execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .is_unit()
    );
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(p.encode_generic_package_native_image().is_err());
}

mod collection_insert_execution {
    use super::*;
    fn compile(
        f: &Fixture,
    ) -> Result<CompiledGenericPackage, sovra::compiler::diagnostics::Diagnostics> {
        compile_generic_collection_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
    }
    fn run(source: &str) -> GenericVmOutcome {
        let f = Fixture::new();
        f.write("app/main.svr", source);
        execute_generic_collection_package(&compile(&f).unwrap(), GenericVmPolicy::DEFAULT).unwrap()
    }
    #[test]
    fn real_map_insert_replaces_in_position_and_keeps_original_snapshots() {
        let out=run("struct Pair<T>{old:T,new:T}fn main()->Pair<std::Map<Int,String>>{let empty:std::Map<Int,String>=std::map_new();let a=std::map_insert(empty,1,\"one\");let b=std::map_insert(a,2,\"two\");let c=std::map_insert(b,1,\"new\");return Pair::<std::Map<Int,String>>{old:b,new:c};}");
        let old = out.value().field(0).unwrap();
        let new = out.value().field(1).unwrap();
        assert_eq!(old.map_len(), Some(2));
        assert_eq!(new.map_len(), Some(2));
        assert_eq!(new.map_entry(0).unwrap().0.as_int(), Some(1));
        assert_eq!(new.map_entry(1).unwrap().0.as_int(), Some(2));
        assert_eq!(old.map_entry(0).unwrap().1.as_str(), Some("one"));
        assert_eq!(new.map_entry(0).unwrap().1.as_str(), Some("new"));
        assert!(new.map_entry(2).is_none());
    }
    #[test]
    fn source_map_length_now_observes_real_entries_and_set_duplicates_keep_position() {
        assert_eq!(run("fn main()->Int{let m:std::Map<Int,Int>=std::map_new();let a=std::map_insert(m,1,10);let b=std::map_insert(a,2,20);return std::map_len(b);}").value().as_int(),Some(2));
        let out=run("fn main()->std::Set<Int>{let s:std::Set<Int>=std::set_new();let a=std::set_insert(s,7);let b=std::set_insert(a,9);return std::set_insert(b,7);}");
        assert_eq!(out.value().set_len(), Some(2));
        assert_eq!(out.value().set_item(0).unwrap().as_int(), Some(7));
        assert_eq!(out.value().set_item(1).unwrap().as_int(), Some(9));
        assert_eq!(out.value().footprint(), (3, 0));
    }
    #[test]
    fn equality_is_association_or_membership_based_not_insertion_order() {
        let out=run("fn main()->Bool{let m:std::Map<Int,Int>=std::map_new();let a=std::map_insert(std::map_insert(m,1,7),2,9);let b=std::map_insert(std::map_insert(m,2,9),1,7);let c=std::map_insert(b,1,8);let s:std::Set<Int>=std::set_new();let x=std::set_insert(std::set_insert(s,1),2);let y=std::set_insert(std::set_insert(s,2),1);return a==b&&a!=c&&x==y;}");
        assert_eq!(out.value().as_bool(), Some(true));
    }
    #[test]
    fn nested_values_and_returned_keys_keep_real_type_proof_through_compaction() {
        let out=run("fn main()->std::Map<String,[Int]>{let m:std::Map<String,[Int]>=std::map_new();let mut data:[Int]=[1,2];let a=std::map_insert(m,\"λ\",data);data[0]=99;return a;}");
        let (k, v) = out.value().map_entry(0).unwrap();
        assert_eq!(k.as_str(), Some("λ"));
        assert_eq!(v.array_item(0).unwrap().as_int(), Some(1));
        let snap = out
            .collection_snapshot(
                &[GenericCollectionSelector::MapValue(0), Item(0)],
                &mut unlimited(),
                &mut work(),
            )
            .unwrap();
        assert_eq!(snap.value().as_int(), Some(1));
        assert!(snap.ty().is_key());
        let key = out
            .collection_snapshot(
                &[GenericCollectionSelector::MapKey(0)],
                &mut unlimited(),
                &mut work(),
            )
            .unwrap();
        assert_eq!(key.value().as_str(), Some("λ"));
    }
    #[test]
    fn nested_collections_and_loops_preserve_flat_value_intervals() {
        let out=run("fn main()->std::Map<Int,std::Set<Int>>{let m:std::Map<Int,std::Set<Int>>=std::map_new();let s:std::Set<Int>=std::set_new();let mut result=m;let mut i=0;while i<8{let values=std::set_insert(s,i);result=std::map_insert(result,i,values);i=i+1;}return result;}");
        assert_eq!(out.value().map_len(), Some(8));
        for i in 0..8 {
            let (k, v) = out.value().map_entry(i).unwrap();
            assert_eq!(k.as_int(), Some(i as i64));
            assert_eq!(v.set_item(0).unwrap().as_int(), Some(i as i64));
        }
        let s = out
            .collection_snapshot(
                &[
                    GenericCollectionSelector::MapValue(7),
                    GenericCollectionSelector::SetItem(0),
                ],
                &mut unlimited(),
                &mut work(),
            )
            .unwrap();
        assert_eq!(s.value().as_int(), Some(7));
        let copied = out
            .collection_snapshot(&[], &mut unlimited(), &mut work())
            .unwrap();
        assert!(eq(
            &copied,
            &copied.snapshot(&mut unlimited(), &mut work()).unwrap()
        ));
    }
    #[test]
    fn exact_insert_node_and_text_costs_include_internal_entries_and_copied_descendants() {
        let f = Fixture::new();
        f.write("app/main.svr","fn main()->std::Map<Int,Int>{let m:std::Map<Int,Int>=std::map_new();return std::map_insert(m,1,7);}");
        let p = compile(&f).unwrap();
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.runtime.materialization.max_value_nodes = Some(8);
        policy.runtime.materialization.max_text_bytes = Some(0);
        let out = execute_generic_collection_package(&p, policy.clone()).unwrap();
        assert_eq!(out.value().footprint(), (4, 0));
        assert_eq!(out.remaining().materialization.max_value_nodes, Some(0));
        policy.runtime.materialization.max_value_nodes = Some(7);
        assert!(execute_generic_collection_package(&p, policy).is_err());
        f.write("app/main.svr","fn main()->std::Map<String,String>{let m:std::Map<String,String>=std::map_new();return std::map_insert(m,\"λ\",\"😀\");}");
        let p = compile(&f).unwrap();
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.runtime.materialization.max_value_nodes = Some(8);
        policy.runtime.materialization.max_text_bytes = Some(12);
        let out = execute_generic_collection_package(&p, policy.clone()).unwrap();
        assert_eq!(out.value().footprint(), (4, 6));
        assert_eq!(out.remaining().materialization.max_text_bytes, Some(0));
        policy.runtime.materialization.max_text_bytes = Some(11);
        assert!(execute_generic_collection_package(&p, policy).is_err());
    }
    #[test]
    fn exact_key_value_types_and_collection_receiver_are_checked_before_execution() {
        let f = Fixture::new();
        for source in [
            "fn main(){let m:std::Map<Int,Int>=std::map_new();let bad=std::map_insert(m,true,1);}",
            "fn main(){let m:std::Map<Int,Float>=std::map_new();let bad=std::map_insert(m,1,1);}",
            "fn main(){let s:std::Set<Int>=std::set_new();let bad=std::set_insert(s,1.0);}",
            "fn main(){let s:std::Set<Int>=std::set_new();let bad=std::map_insert(s,1,2);}",
            "fn main(){let m:std::Map<Int,Int>=std::map_new();let bad=std::map_insert(m,1);}",
        ] {
            f.write("app/main.svr", source);
            assert!(compile(&f).is_err(), "{source}");
        }
    }
    #[test]
    fn string_keys_use_exact_unicode_and_bool_key_insertion_is_supported() {
        let out=run("fn main()->std::Map<String,Int>{let m:std::Map<String,Int>=std::map_new();let a=std::map_insert(m,\"é\",1);let b=std::map_insert(a,\"é\",2);return std::map_insert(b,\"\",3);}");
        assert_eq!(out.value().map_len(), Some(3));
        let out=run("fn main()->std::Set<Bool>{let s:std::Set<Bool>=std::set_new();let a=std::set_insert(s,true);return std::set_insert(a,false);}");
        assert_eq!(out.value().set_len(), Some(2));
    }
    #[test]
    fn imported_generic_insertion_and_snapshot_equality_preserve_nominal_values() {
        let f = Fixture::new();
        f.write("dep/types.svr","mod types{export struct Box<T>{value:T}export fn put<K:std::Key,V>(m:std::Map<K,V>,key:K,value:V)->std::Map<K,V>{return std::map_insert(m,key,value);}}");
        f.write("app/main.svr","use dep::types;fn main()->std::Map<Int,dep::types::Box<Int>>{let m:std::Map<Int,dep::types::Box<Int>>=std::map_new();return dep::types::put::<Int,dep::types::Box<Int>>(m,1,dep::types::Box::<Int>{value:7});}");
        let p = compile(&f).unwrap();
        let out = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(
            out.value()
                .map_entry(0)
                .unwrap()
                .1
                .field(0)
                .unwrap()
                .as_int(),
            Some(7)
        );
        let a = out
            .collection_snapshot(&[], &mut unlimited(), &mut work())
            .unwrap();
        assert!(eq(&a, &a.snapshot(&mut unlimited(), &mut work()).unwrap()));
    }
    #[test]
    fn shared_work_limits_refuse_collection_copy_and_association_search_without_fallback() {
        let f = Fixture::new();
        f.write("app/main.svr","fn main()->Bool{let m:std::Map<Int,Int>=std::map_new();let a=std::map_insert(std::map_insert(m,1,7),2,9);let b=std::map_insert(std::map_insert(m,2,9),1,7);return a==b;}");
        let p = compile(&f).unwrap();
        let out = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        let mut limited = GenericVmPolicy::DEFAULT;
        limited.max_work = out.work() - 1;
        assert!(execute_generic_collection_package(&p, limited).is_err());
        assert_eq!(
            execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT)
                .unwrap()
                .value()
                .as_bool(),
            Some(true)
        );
    }
}

#[test]
fn source_insert_histories_match_ordered_oracle_and_reverse_order_equality() {
    let f = Fixture::new();
    for trace in 0..4usize.pow(3) {
        let mut source=String::from("struct Out<M,S>{map:M,set:S,equal:Bool}fn main()->Out<std::Map<Int,Int>,std::Set<Int>>{let m0:std::Map<Int,Int>=std::map_new();let s0:std::Set<Int>=std::set_new();");
        let mut code = trace;
        let mut expected: Vec<(i64, i64)> = Vec::new();
        for step in 0..3 {
            let op = code % 4;
            code /= 4;
            let key = (op % 2) as i64;
            let value = if op < 2 { 7 } else { 9 };
            if let Some(pair) = expected.iter_mut().find(|p| p.0 == key) {
                pair.1 = value
            } else {
                expected.push((key, value));
            }
            source.push_str(&format!("let m{}=std::map_insert(m{step},{key},{value});let s{}=std::set_insert(s{step},{key});",step+1,step+1));
        }
        source.push_str(
            "let r0:std::Map<Int,Int>=std::map_new();let t0:std::Set<Int>=std::set_new();",
        );
        for (i, (key, value)) in expected.iter().rev().enumerate() {
            source.push_str(&format!(
                "let r{}=std::map_insert(r{i},{key},{value});let t{}=std::set_insert(t{i},{key});",
                i + 1,
                i + 1
            ));
        }
        source.push_str(&format!("return Out::<std::Map<Int,Int>,std::Set<Int>>{{map:m3,set:s3,equal:m3==r{}&&s3==t{}}};}}",expected.len(),expected.len()));
        f.write("app/main.svr", &source);
        let p = compile_generic_collection_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
        .unwrap();
        let out = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        let map = out.value().field(0).unwrap();
        let set = out.value().field(1).unwrap();
        assert_eq!(map.map_len(), Some(expected.len()));
        assert_eq!(set.set_len(), Some(expected.len()));
        assert_eq!(
            out.value().field(2).unwrap().as_bool(),
            Some(true),
            "history {trace}"
        );
        for (i, (key, value)) in expected.iter().enumerate() {
            let (k, v) = map.map_entry(i).unwrap();
            assert_eq!(k.as_int(), Some(*key));
            assert_eq!(v.as_int(), Some(*value));
            assert_eq!(set.set_item(i).unwrap().as_int(), Some(*key));
        }
    }
}
#[test]
fn recursive_record_values_inside_nonempty_maps_copy_compare_and_drop_flat() {
    let f = Fixture::new();
    f.write("app/main.svr","struct Node<T>{children:std::Map<String,Node<T>>,value:T}fn main()->Node<Int>{let empty:std::Map<String,Node<Int>>=std::map_new();let mut node=Node::<Int>{children:empty,value:0};let mut i=0;while i<16{let children=std::map_insert(empty,\"next\",node);node=Node::<Int>{children:children,value:i};i=i+1;}return node;}");
    let p = compile_generic_collection_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    let out = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
    let snapshot = out
        .collection_snapshot(&[], &mut unlimited(), &mut work())
        .unwrap();
    let copied = snapshot.snapshot(&mut unlimited(), &mut work()).unwrap();
    assert!(eq(&snapshot, &copied));
    let mut path = Vec::new();
    for _ in 0..16 {
        path.push(Field(0));
        path.push(GenericCollectionSelector::MapValue(0));
    }
    path.push(Field(1));
    assert_eq!(
        copied
            .select(&path, &mut unlimited(), &mut work())
            .unwrap()
            .value()
            .as_int(),
        Some(0)
    );
    drop(out);
    drop(snapshot);
    drop(copied);
}

#[test]
fn published_collection_insertion_source_preserves_old_execution_and_native_gates() {
    let f = Fixture::new();
    f.write(
        "app/main.svr",
        &std::fs::read_to_string("docs/examples/intrinsic-collection-insertion.svr").unwrap(),
    );
    let p = compile_generic_collection_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .as_bool(),
        Some(true)
    );
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert_eq!(
        p.encode_generic_package_native_image().unwrap_err(),
        "native collection feature is not enabled"
    );
}

mod collection_membership_execution {
    use super::*;
    fn compile(
        f: &Fixture,
    ) -> Result<CompiledGenericPackage, sovra::compiler::diagnostics::Diagnostics> {
        compile_generic_collection_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
    }
    #[test]
    fn membership_covers_empty_present_missing_and_exact_unicode_keys() {
        for (collection, key_type, key, other) in [
            ("map", "Int", "7", "8"),
            ("set", "Bool", "true", "false"),
            ("map", "String", "\"é\"", "\"é\""),
            ("set", "String", "\"λ😀\"", "\"λ\""),
        ] {
            let f = Fixture::new();
            let ty = if collection == "map" {
                format!("std::Map<{key_type},String>")
            } else {
                format!("std::Set<{key_type}>")
            };
            let extra = if collection == "map" {
                ",\"payload\""
            } else {
                ""
            };
            f.write("app/main.svr", &format!("fn main()->Bool{{let empty:{ty}=std::{collection}_new();if (std::{collection}_contains(empty,{key})){{return false;}}let full=std::{collection}_insert(empty,{key}{extra});if (std::{collection}_contains(full,{other})){{return false;}}return std::{collection}_contains(full,{key});}}"));
            let p = compile(&f).unwrap();
            assert_eq!(
                execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT)
                    .unwrap()
                    .value()
                    .as_bool(),
                Some(true)
            );
            assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
            assert!(p.encode_generic_package_native_image().is_err());
        }
    }
    #[test]
    fn membership_refuses_wrong_key_constructor_and_arity() {
        for call in [
            "std::map_contains(m,true)",
            "std::set_contains(m,1)",
            "std::map_contains(m)",
            "std::map_contains(m,1,2)",
        ] {
            let f = Fixture::new();
            f.write(
                "app/main.svr",
                &format!(
                    "fn main()->Bool{{let m:std::Map<Int,String>=std::map_new();return {call};}}"
                ),
            );
            assert!(compile(&f).is_err(), "{call}");
        }
    }
    #[test]
    fn membership_scans_share_the_vm_work_limit() {
        let f = Fixture::new();
        f.write("app/main.svr", "fn main()->Bool{let m:std::Map<String,Int>=std::map_new();let full=std::map_insert(std::map_insert(m,\"λ😀\",1),\"last\",2);return std::map_contains(full,\"missing\");}");
        let p = compile(&f).unwrap();
        let out = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert_eq!(out.value().as_bool(), Some(false));
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.max_work = out.work();
        assert!(execute_generic_collection_package(&p, policy.clone()).is_ok());
        policy.max_work -= 1;
        assert!(execute_generic_collection_package(&p, policy.clone()).is_err());
    }
    #[test]
    fn membership_result_is_a_budgeted_bool() {
        let f = Fixture::new();
        f.write(
            "app/main.svr",
            "fn main()->Bool{let s:std::Set<Int>=std::set_new();return std::set_contains(s,1);}",
        );
        let p = compile(&f).unwrap();
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.runtime.materialization.max_value_nodes = Some(4);
        let out = execute_generic_collection_package(&p, policy.clone()).unwrap();
        assert_eq!(out.value().as_bool(), Some(false));
        assert_eq!(out.remaining().materialization.max_value_nodes, Some(0));
        policy.runtime.materialization.max_value_nodes = Some(3);
        assert!(execute_generic_collection_package(&p, policy.clone()).is_err());
    }
}

mod collection_lookup_execution {
    use super::*;
    fn compile(
        f: &Fixture,
    ) -> Result<CompiledGenericPackage, sovra::compiler::diagnostics::Diagnostics> {
        compile_generic_collection_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
    }
    fn run(source: &str) -> GenericVmOutcome {
        let f = Fixture::new();
        f.write("app/main.svr", source);
        execute_generic_collection_package(&compile(&f).unwrap(), GenericVmPolicy::DEFAULT).unwrap()
    }
    #[test]
    fn present_value_and_missing_key_produce_real_standard_options() {
        let out=run("struct Out<A,B>{a:A,b:B}fn main()->Out<std::Option<String>,std::Option<String>>{let m:std::Map<Int,String>=std::map_new();let full=std::map_insert(m,7,\"λ😀\");return Out::<std::Option<String>,std::Option<String>>{a:std::map_get(full,7),b:std::map_get(full,8)};}");
        let some = out.value().field(0).unwrap();
        let none = out.value().field(1).unwrap();
        assert_eq!(some.enum_tag(), Some(0));
        assert_eq!(some.payload().unwrap().as_str(), Some("λ😀"));
        assert_eq!(some.footprint(), (2, 21)); // std::Option + Some + UTF-8 payload
        assert_eq!(none.enum_tag(), Some(1));
        assert!(none.payload().is_none());
        assert_eq!(none.footprint(), (1, 15));
    }
    #[test]
    fn missing_lookup_does_not_construct_an_uninhabited_value() {
        let out=run("struct Never<T>{next:Never<T>}fn main()->std::Option<Never<Int>>{let m:std::Map<Int,Never<Int>>=std::map_new();return std::map_get(m,7);}");
        assert_eq!(out.value().enum_tag(), Some(1));
        assert!(out.value().payload().is_none());
    }
    #[test]
    fn set_get_uses_exact_unicode_and_boolean_keys() {
        for (ty, key, other) in [
            ("String", "\"é\"", "\"é\""),
            ("Bool", "true", "false"),
            ("Int", "7", "8"),
        ] {
            let out=run(&format!("fn main()->std::Option<{ty}>{{let empty:std::Set<{ty}>=std::set_new();let s=std::set_insert(empty,{key});let absent=std::set_get(s,{other});match absent{{std::Option::Some(v)=>{{return std::Option::<{ty}>::None();}},std::Option::None()=>{{return std::set_get(s,{key});}}}}}}"));
            assert_eq!(out.value().enum_tag(), Some(0));
            assert!(out.value().payload().is_some());
        }
    }
    #[test]
    fn lookup_composes_with_generic_helpers_matches_and_propagation() {
        let out=run("fn get<K:std::Key,V>(m:std::Map<K,V>,k:K)->std::Option<V>{return std::map_get(m,k);}fn value(m:std::Map<Int,Int>,k:Int)->std::Option<Int>{let v=get::<Int,Int>(m,k)?;return std::Option::<Int>::Some(v+1);}fn main()->Int{let m:std::Map<Int,Int>=std::map_new();let full=std::map_insert(m,7,40);let absent=value(full,8);match absent{std::Option::Some(v)=>{return 0;},std::Option::None()=>{let present=value(full,7);match present{std::Option::Some(v)=>{return v;},std::Option::None()=>{return 0;}}}}}");
        assert_eq!(out.value().as_int(), Some(41));
    }
    #[test]
    fn returned_record_is_an_independent_snapshot() {
        let out=run("struct Box<T>{value:T}fn main()->Int{let empty:std::Map<Int,Box<[Int]>>=std::map_new();let m=std::map_insert(empty,1,Box::<[Int]>{value:[7]});let result=std::map_get(m,1);match result{std::Option::Some(v)=>{let mut changed=v.value;changed[0]=99;let again=std::map_get(m,1);match again{std::Option::Some(original)=>{let values=original.value;return values[0];},std::Option::None()=>{return 0;}}},std::Option::None()=>{return 0;}}}");
        assert_eq!(out.value().as_int(), Some(7));
    }
    #[test]
    fn nested_collection_payloads_keep_their_layouts_and_entries() {
        let out=run("fn main()->std::Option<std::Set<String>>{let s:std::Set<String>=std::set_new();let full=std::set_insert(s,\"λ😀\");let m:std::Map<Bool,std::Set<String>>=std::map_new();return std::map_get(std::map_insert(m,true,full),true);}");
        let payload = out.value().payload().unwrap();
        assert_eq!(payload.set_len(), Some(1));
        assert_eq!(payload.set_item(0).unwrap().as_str(), Some("λ😀"));
        let snapshot = snap(&out, &[]);
        assert!(eq(
            &snapshot,
            &snapshot.snapshot(&mut unlimited(), &mut work()).unwrap()
        ));
    }
    #[test]
    fn lookup_preserves_imported_alias_and_phantom_type_ownership() {
        let source="use dep::types;struct Box<T>{value:T}type Alias<T>=dep::types::Box<T>;fn main()->std::Option<Alias<Int>>{let m:std::Map<Int,dep::types::Box<Int>>=std::map_new();let full=std::map_insert(m,1,dep::types::Box::<Int>{value:7});return std::map_get(full,1);}";
        let out = run(source);
        assert_eq!(
            out.value().payload().unwrap().field(0).unwrap().as_int(),
            Some(7)
        );
        for bad in [source.replace("std::Option<Alias<Int>>","std::Option<Box<Int>>"),
            "use dep::types;fn main()->std::Option<dep::types::Empty<String>>{let m:std::Map<Int,dep::types::Empty<Int>>=std::map_new();return std::map_get(m,1);}".into()] {
            let f=Fixture::new(); f.write("app/main.svr",&bad);assert!(compile(&f).is_err());
        }
    }
    #[test]
    fn derived_option_types_respect_generated_node_edge_work_and_depth_limits() {
        let f = Fixture::new();
        f.write("app/main.svr","fn main()->std::Option<Int>{let m:std::Map<Int,Int>=std::map_new();return std::map_get(m,1);}");
        for kind in 0..4 {
            let mut limits = GenericPackageCompilationLimits::DEFAULT;
            let expansion = &mut limits.bodies.calls.arguments.expansion;
            match kind {
                0 => expansion.max_nodes = 1,
                1 => expansion.max_edges = 0,
                2 => expansion.max_work = 0,
                _ => expansion.max_depth = 0,
            }
            assert!(compile_generic_collection_package(
                capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
                "main",
                limits
            )
            .is_err());
        }
    }
    #[test]
    fn lookup_requires_exact_key_receiver_and_option_payload_types() {
        for source in [
            "fn main()->std::Option<String>{let m:std::Map<Int,String>=std::map_new();return std::map_get(m,true);}",
            "fn main()->std::Option<Float>{let m:std::Map<Int,Int>=std::map_new();return std::map_get(m,1);}",
            "fn main()->std::Option<Int>{let m:std::Map<Int,Int>=std::map_new();return std::set_get(m,1);}",
            "fn main()->std::Option<Int>{let s:std::Set<Int>=std::set_new();return std::map_get(s,1);}",
            "fn main()->std::Option<Int>{let m:std::Map<Int,Int>=std::map_new();return std::map_get(m);}",
            "fn main()->std::Option<Int>{let m:std::Map<Int,Int>=std::map_new();return std::map_get::<Int>(m,1);}",
        ] { let f=Fixture::new();f.write("app/main.svr",source);assert!(compile(&f).is_err(),"{source}"); }
    }
    #[test]
    fn none_and_some_charge_all_nodes_and_owned_labels_at_exact_limits() {
        for (source,nodes,bytes) in [
            ("fn main()->std::Option<Int>{let m:std::Map<Int,Int>=std::map_new();return std::map_get(m,1);}",4,15),
            ("fn main()->std::Option<String>{let m:std::Map<Int,String>=std::map_new();let full=std::map_insert(m,1,\"λ😀\");return std::map_get(full,1);}",15,39),
        ] {
            let f=Fixture::new();f.write("app/main.svr",source);let p=compile(&f).unwrap();
            let mut policy=GenericVmPolicy::DEFAULT;
            policy.runtime.materialization.max_value_nodes=Some(nodes);
            policy.runtime.materialization.max_text_bytes=Some(bytes);
            let out=execute_generic_collection_package(&p,policy.clone()).unwrap();
            assert_eq!(out.remaining().materialization.max_value_nodes,Some(0));
            assert_eq!(out.remaining().materialization.max_text_bytes,Some(0));
            policy.runtime.materialization.max_value_nodes=Some(nodes-1);
            assert!(execute_generic_collection_package(&p,policy.clone()).is_err());
            policy.runtime.materialization.max_value_nodes=Some(nodes);
            policy.runtime.materialization.max_text_bytes=Some(bytes-1);
            assert!(execute_generic_collection_package(&p,policy).is_err());
        }
    }
    #[test]
    fn search_and_payload_copy_respect_shared_work_and_old_profile_gates() {
        let f = Fixture::new();
        f.write("app/main.svr","fn main()->std::Option<String>{let m:std::Map<Int,String>=std::map_new();return std::map_get(std::map_insert(m,1,\"λ😀\"),1);}");
        let p = compile(&f).unwrap();
        let out = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.max_work = out.work();
        assert!(execute_generic_collection_package(&p, policy.clone()).is_ok());
        policy.max_work -= 1;
        assert!(execute_generic_collection_package(&p, policy).is_err());
        assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
        assert_eq!(
            p.encode_generic_package_native_image().unwrap_err(),
            "native collection feature is not enabled"
        );
        assert!(compile_generic_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT
        )
        .is_err());
    }
}

#[test]
fn published_collection_lookup_source_executes_only_in_the_explicit_profile() {
    let f = Fixture::new();
    f.write(
        "app/main.svr",
        &std::fs::read_to_string("docs/examples/intrinsic-collection-lookup.svr").unwrap(),
    );
    let p = compile_generic_collection_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .as_bool(),
        Some(true)
    );
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert_eq!(
        p.encode_generic_package_native_image().unwrap_err(),
        "native collection feature is not enabled"
    );
}

mod collection_removal_execution {
    use super::*;
    fn compile(
        f: &Fixture,
    ) -> Result<CompiledGenericPackage, sovra::compiler::diagnostics::Diagnostics> {
        compile_generic_collection_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
    }
    fn run(source: &str) -> GenericVmOutcome {
        let f = Fixture::new();
        f.write("app/main.svr", source);
        execute_generic_collection_package(&compile(&f).unwrap(), GenericVmPolicy::DEFAULT).unwrap()
    }
    #[test]
    fn map_removal_copies_survivors_in_order_and_preserves_original() {
        let out=run("struct Out<T>{old:T,result:std::Option<T>}fn main()->Out<std::Map<Int,String>>{let empty:std::Map<Int,String>=std::map_new();let full=std::map_insert(std::map_insert(std::map_insert(empty,1,\"one\"),2,\"two\"),3,\"three\");return Out::<std::Map<Int,String>>{old:full,result:std::map_remove(full,2)};}");
        let old = out.value().field(0).unwrap();
        let result = out.value().field(1).unwrap();
        assert_eq!(old.map_len(), Some(3));
        assert_eq!(result.enum_tag(), Some(0));
        let updated = result.payload().unwrap();
        assert_eq!(updated.map_len(), Some(2));
        assert_eq!(updated.map_entry(0).unwrap().0.as_int(), Some(1));
        assert_eq!(updated.map_entry(1).unwrap().0.as_int(), Some(3));
        assert_eq!(old.map_entry(1).unwrap().1.as_str(), Some("two"));
        assert_eq!(updated.map_entry(1).unwrap().1.as_str(), Some("three"));
        assert_eq!(result.footprint(), (8, 23)); // wrapper, map, two entries/keys/values + labels/text
    }
    #[test]
    fn empty_or_missing_removal_returns_none_without_payload() {
        for source in [
            "fn main()->std::Option<std::Map<Int,Int>>{let m:std::Map<Int,Int>=std::map_new();return std::map_remove(m,1);}",
            "fn main()->std::Option<std::Set<Bool>>{let s:std::Set<Bool>=std::set_new();return std::set_remove(std::set_insert(s,true),false);}",
            "fn main()->std::Option<std::Map<String,String>>{let m:std::Map<String,String>=std::map_new();return std::map_remove(std::map_insert(m,\"é\",\"value\"),\"é\");}",
        ] {
            let out=run(source);assert_eq!(out.value().enum_tag(),Some(1));
            assert!(out.value().payload().is_none());assert_eq!(out.value().footprint(),(1,15));
        }
    }
    #[test]
    fn removing_only_key_produces_some_empty_collection_not_none() {
        let out=run("fn main()->std::Option<std::Map<Int,Int>>{let m:std::Map<Int,Int>=std::map_new();return std::map_remove(std::map_insert(m,7,9),7);}");
        assert_eq!(out.value().enum_tag(), Some(0));
        assert_eq!(out.value().payload().unwrap().map_len(), Some(0));
        assert_eq!(out.value().footprint(), (2, 15));
    }
    #[test]
    fn generic_removal_propagates_and_reinsertion_appends() {
        let out=run("fn remove<K:std::Key,V>(m:std::Map<K,V>,k:K)->std::Option<std::Map<K,V>>{return std::map_remove(m,k);}fn again(m:std::Map<Int,Int>)->std::Option<std::Map<Int,Int>>{let updated=remove::<Int,Int>(m,1)?;return std::Option::<std::Map<Int,Int>>::Some(std::map_insert(updated,1,99));}fn main()->std::Option<std::Map<Int,Int>>{let m:std::Map<Int,Int>=std::map_new();return again(std::map_insert(std::map_insert(m,1,7),2,8));}");
        let m = out.value().payload().unwrap();
        assert_eq!(m.map_len(), Some(2));
        assert_eq!(m.map_entry(0).unwrap().0.as_int(), Some(2));
        assert_eq!(m.map_entry(1).unwrap().0.as_int(), Some(1));
        assert_eq!(m.map_entry(1).unwrap().1.as_int(), Some(99));
    }
    #[test]
    fn set_remove_preserves_order_and_reinserted_key_moves_last() {
        let out=run("fn main()->std::Set<String>{let empty:std::Set<String>=std::set_new();let full=std::set_insert(std::set_insert(std::set_insert(empty,\"λ\"),\"😀\"),\"z\");let result=std::set_remove(full,\"😀\");match result{std::Option::Some(updated)=>{return std::set_insert(updated,\"😀\");},std::Option::None()=>{return empty;}}}");
        assert_eq!(out.value().set_len(), Some(3));
        for (i, k) in ["λ", "z", "😀"].iter().enumerate() {
            assert_eq!(out.value().set_item(i).unwrap().as_str(), Some(*k));
        }
    }
    #[test]
    fn nested_surviving_values_keep_imported_owners_labels_and_independent_arrays() {
        let out=run("use dep::types;struct Out<T>{old:T,result:std::Option<T>}fn main()->Out<std::Map<Int,dep::types::Box<[String]>>>{let m:std::Map<Int,dep::types::Box<[String]>>=std::map_new();let full=std::map_insert(std::map_insert(m,1,dep::types::Box::<[String]>{value:[\"λ😀\"]}),2,dep::types::Box::<[String]>{value:[\"remove\"]});return Out::<std::Map<Int,dep::types::Box<[String]>>>{old:full,result:std::map_remove(full,2)};}");
        let updated = out.value().field(1).unwrap().payload().unwrap();
        let value = updated.map_entry(0).unwrap().1;
        assert_eq!(
            value.field(0).unwrap().array_item(0).unwrap().as_str(),
            Some("λ😀")
        );
        let old = snap(&out, &[Field(0), GenericCollectionSelector::MapValue(0)]);
        let survivor = snap(
            &out,
            &[Field(1), Payload, GenericCollectionSelector::MapValue(0)],
        );
        assert!(eq(&old, &survivor));
        let clone = survivor.snapshot(&mut unlimited(), &mut work()).unwrap();
        assert!(eq(&survivor, &clone));
    }
    #[test]
    fn nested_collection_survivors_copy_complete_private_entry_subtrees() {
        let out=run("fn main()->std::Option<std::Map<Int,std::Map<String,std::Set<Bool>>>>{let s:std::Set<Bool>=std::set_new();let full=std::set_insert(s,true);let inner:std::Map<String,std::Set<Bool>>=std::map_new();let nested=std::map_insert(inner,\"λ\",full);let outer:std::Map<Int,std::Map<String,std::Set<Bool>>>=std::map_new();let both=std::map_insert(std::map_insert(outer,1,nested),2,inner);return std::map_remove(both,2);}");
        let root = out.value().payload().unwrap();
        let inner = root.map_entry(0).unwrap().1;
        assert_eq!(inner.map_entry(0).unwrap().0.as_str(), Some("λ"));
        let set = inner.map_entry(0).unwrap().1;
        assert_eq!(set.set_item(0).unwrap().as_bool(), Some(true));
        let snapshot = snap(&out, &[]);
        assert!(eq(
            &snapshot,
            &snapshot.snapshot(&mut unlimited(), &mut work()).unwrap()
        ));
    }
    #[test]
    fn removal_option_of_collection_obeys_derived_type_depth_limits() {
        let f = Fixture::new();
        f.write("app/main.svr","fn main()->std::Option<std::Map<Int,Int>>{let m:std::Map<Int,Int>=std::map_new();return std::map_remove(m,1);}");
        let mut limits = GenericPackageCompilationLimits::DEFAULT;
        limits.bodies.calls.arguments.expansion.max_depth = 3;
        assert!(compile_generic_collection_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            limits
        )
        .is_ok());
        limits.bodies.calls.arguments.expansion.max_depth = 2;
        assert!(compile_generic_collection_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            limits
        )
        .is_err());
    }
    #[test]
    fn removal_rechecks_exact_receiver_key_and_result_arguments() {
        for source in [
            "fn main()->std::Option<std::Map<Int,Int>>{let m:std::Map<Int,Int>=std::map_new();return std::map_remove(m,true);}",
            "fn main()->std::Option<std::Map<Int,Float>>{let m:std::Map<Int,Int>=std::map_new();return std::map_remove(m,1);}",
            "fn main()->std::Option<std::Set<Int>>{let m:std::Map<Int,Int>=std::map_new();return std::set_remove(m,1);}",
            "fn main()->std::Option<std::Map<Int,Int>>{let s:std::Set<Int>=std::set_new();return std::map_remove(s,1);}",
            "fn main()->std::Option<std::Map<Int,Int>>{let m:std::Map<Int,Int>=std::map_new();return std::map_remove(m);}",
            "fn main()->std::Option<std::Map<Int,Int>>{let m:std::Map<Int,Int>=std::map_new();return std::map_remove::<Int>(m,1);}",
        ]{let f=Fixture::new();f.write("app/main.svr",source);assert!(compile(&f).is_err(),"{source}");}
    }
    #[test]
    fn result_admission_includes_option_empty_container_and_all_copied_text() {
        for (source,nodes,bytes) in [
            ("fn main()->std::Option<std::Map<Int,Int>>{let m:std::Map<Int,Int>=std::map_new();return std::map_remove(m,1);}",4,15),
            ("fn main()->std::Option<std::Set<Int>>{let s:std::Set<Int>=std::set_new();let full=std::set_insert(s,7);return std::set_remove(full,7);}",10,15),
            ("fn main()->std::Option<std::Map<Int,String>>{let m:std::Map<Int,String>=std::map_new();let full=std::map_insert(m,1,\"λ😀\");return std::map_remove(full,1);}",15,33),
            ("fn main()->std::Option<std::Set<String>>{let s:std::Set<String>=std::set_new();let full=std::set_insert(s,\"λ😀\");return std::set_remove(full,\"λ😀\");}",10,39),
            ("fn main()->std::Option<std::Map<Int,String>>{let m:std::Map<Int,String>=std::map_new();let one=std::map_insert(m,1,\"λ😀\");let full=std::map_insert(one,2,\"x\");return std::map_remove(full,2);}",34,54),
            ("fn main()->std::Option<std::Set<String>>{let s:std::Set<String>=std::set_new();let one=std::set_insert(s,\"λ😀\");let full=std::set_insert(one,\"x\");return std::set_remove(full,\"x\");}",18,55),
        ]{
            let f=Fixture::new();f.write("app/main.svr",source);let p=compile(&f).unwrap();
            let mut policy=GenericVmPolicy::DEFAULT;
            policy.runtime.materialization.max_value_nodes=Some(nodes);
            policy.runtime.materialization.max_text_bytes=Some(bytes);
            let out=execute_generic_collection_package(&p,policy.clone()).unwrap();
            assert_eq!(out.remaining().materialization.max_value_nodes,Some(0));
            assert_eq!(out.remaining().materialization.max_text_bytes,Some(0));
            policy.runtime.materialization.max_value_nodes=Some(nodes-1);
            assert!(execute_generic_collection_package(&p,policy.clone()).is_err());
            policy.runtime.materialization.max_value_nodes=Some(nodes);
            policy.runtime.materialization.max_text_bytes=Some(bytes-1);
            assert!(execute_generic_collection_package(&p,policy).is_err());
        }
    }
    #[test]
    fn search_and_copy_respect_work_limit_without_successful_fallback() {
        let f = Fixture::new();
        f.write("app/main.svr","fn main()->std::Option<std::Map<String,String>>{let m:std::Map<String,String>=std::map_new();let full=std::map_insert(std::map_insert(m,\"λ\",\"😀\"),\"last\",\"keep\");return std::map_remove(full,\"λ\");}");
        let p = compile(&f).unwrap();
        let out = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.max_work = out.work();
        assert!(execute_generic_collection_package(&p, policy.clone()).is_ok());
        policy.max_work -= 1;
        assert!(execute_generic_collection_package(&p, policy).is_err());
        assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
        assert_eq!(
            p.encode_generic_package_native_image().unwrap_err(),
            "native collection feature is not enabled"
        );
    }
    #[test]
    fn short_insert_remove_histories_match_an_ordered_oracle() {
        for trace in 0..64 {
            let mut source=String::from("struct Out<M,S>{map:M,set:S}fn main()->Out<std::Map<Int,Int>,std::Set<Int>>{let mut m:std::Map<Int,Int>=std::map_new();let mut s:std::Set<Int>=std::set_new();");
            let mut code = trace;
            let mut expected: Vec<(i64, i64)> = Vec::new();
            for step in 0..3 {
                let op = code % 4;
                code /= 4;
                let key = (op % 2) as i64;
                if op < 2 {
                    let value = step + 10;
                    source.push_str(&format!(
                        "m=std::map_insert(m,{key},{value});s=std::set_insert(s,{key});"
                    ));
                    if let Some(pair) = expected.iter_mut().find(|p| p.0 == key) {
                        pair.1 = value;
                    } else {
                        expected.push((key, value));
                    }
                } else {
                    source.push_str(&format!("let mr{step}=std::map_remove(m,{key});match mr{step}{{std::Option::Some(updated)=>{{m=updated;}},std::Option::None()=>{{}}}}let sr{step}=std::set_remove(s,{key});match sr{step}{{std::Option::Some(updated)=>{{s=updated;}},std::Option::None()=>{{}}}}"));
                    expected.retain(|p| p.0 != key);
                }
            }
            source.push_str("return Out::<std::Map<Int,Int>,std::Set<Int>>{map:m,set:s};}");
            let out = run(&source);
            let map = out.value().field(0).unwrap();
            let set = out.value().field(1).unwrap();
            assert_eq!(map.map_len(), Some(expected.len()), "trace {trace}");
            assert_eq!(set.set_len(), Some(expected.len()));
            for (i, (key, value)) in expected.iter().enumerate() {
                assert_eq!(map.map_entry(i).unwrap().0.as_int(), Some(*key));
                assert_eq!(map.map_entry(i).unwrap().1.as_int(), Some(*value));
                assert_eq!(set.set_item(i).unwrap().as_int(), Some(*key));
            }
        }
    }
}

#[test]
fn published_collection_removal_source_executes_only_in_the_explicit_profile() {
    let f = Fixture::new();
    f.write(
        "app/main.svr",
        &std::fs::read_to_string("docs/examples/intrinsic-collection-removal.svr").unwrap(),
    );
    let sources = capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap();
    let p = compile_generic_collection_package(
        sources,
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .as_bool(),
        Some(true)
    );
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(compile_generic_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT
    )
    .is_err());
    assert_eq!(
        p.encode_generic_package_native_image().unwrap_err(),
        "native collection feature is not enabled"
    );
}

mod collection_iteration_execution {
    use super::*;
    fn compile(
        f: &Fixture,
    ) -> Result<CompiledGenericPackage, sovra::compiler::diagnostics::Diagnostics> {
        compile_generic_collection_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
    }
    fn run(source: &str) -> GenericVmOutcome {
        let f = Fixture::new();
        f.write("app/main.svr", source);
        execute_generic_collection_package(&compile(&f).unwrap(), GenericVmPolicy::DEFAULT).unwrap()
    }
    #[test]
    fn empty_snapshots_are_typed_arrays_without_default_elements() {
        for source in [
            "fn main()->[Int]{let m:std::Map<Int,String>=std::map_new();return std::map_keys(m);}",
            "struct Never<T>{next:Never<T>}fn main()->[Never<Int>]{let m:std::Map<Bool,Never<Int>>=std::map_new();return std::map_values(m);}",
            "fn main()->[String]{let s:std::Set<String>=std::set_new();return std::set_values(s);}",
        ]{
            let out=run(source);assert_eq!(out.value().array_len(),Some(0));assert_eq!(out.value().footprint(),(1,0));
        }
    }
    #[test]
    fn map_key_value_snapshots_share_order_and_replacement_stays_in_position() {
        let out=run("struct Out<K,V>{keys:[K],values:[V]}fn main()->Out<Int,String>{let m:std::Map<Int,String>=std::map_new();let full=std::map_insert(std::map_insert(std::map_insert(m,7,\"old\"),9,\"second\"),7,\"new\");return Out::<Int,String>{keys:std::map_keys(full),values:std::map_values(full)};}");
        let keys = out.value().field(0).unwrap();
        let values = out.value().field(1).unwrap();
        assert_eq!(keys.array_len(), Some(2));
        assert_eq!(values.array_len(), Some(2));
        assert_eq!(keys.array_item(0).unwrap().as_int(), Some(7));
        assert_eq!(keys.array_item(1).unwrap().as_int(), Some(9));
        assert_eq!(values.array_item(0).unwrap().as_str(), Some("new"));
        assert_eq!(values.array_item(1).unwrap().as_str(), Some("second"));
        assert_eq!(keys.footprint(), (3, 0));
        assert_eq!(values.footprint(), (3, 9));
    }
    #[test]
    fn set_snapshot_preserves_duplicate_and_remove_reinsert_order() {
        let out=run("fn main()->[String]{let s:std::Set<String>=std::set_new();let full=std::set_insert(std::set_insert(std::set_insert(s,\"λ\"),\"😀\"),\"λ\");let removed=std::set_remove(full,\"λ\");match removed{std::Option::Some(updated)=>{return std::set_values(std::set_insert(updated,\"λ\"));},std::Option::None()=>{return std::set_values(full);}}}");
        assert_eq!(out.value().array_len(), Some(2));
        assert_eq!(out.value().array_item(0).unwrap().as_str(), Some("😀"));
        assert_eq!(out.value().array_item(1).unwrap().as_str(), Some("λ"));
        assert_eq!(out.value().footprint(), (3, 6));
    }
    #[test]
    fn generic_helpers_and_ordinary_array_algorithms_accept_snapshots() {
        let out=run("fn keys<K:std::Key,V>(m:std::Map<K,V>)->[K]{return std::map_keys(m);}fn values<K:std::Key,V>(m:std::Map<K,V>)->[V]{return std::map_values(m);}fn main()->Int{let m:std::Map<Int,Int>=std::map_new();let full=std::map_insert(std::map_insert(m,3,7),4,9);let k=keys::<Int,Int>(full);let v=std::array_append(values::<Int,Int>(full),11);return k[0]+v[1]+std::array_len(v);}");
        assert_eq!(out.value().as_int(), Some(15));
    }
    #[test]
    fn returned_value_arrays_do_not_mutate_stored_values() {
        let out=run("fn main()->Int{let m:std::Map<Int,[Int]>=std::map_new();let full=std::map_insert(m,1,[7]);let values=std::map_values(full);let mut changed=values[0];changed[0]=99;let original=std::map_get(full,1);match original{std::Option::Some(v)=>{return v[0];},std::Option::None()=>{return 0;}}}");
        assert_eq!(out.value().as_int(), Some(7));
    }
    #[test]
    fn nested_imported_and_collection_values_retain_owners_and_all_descendants() {
        let out=run("use dep::types;fn main()->[dep::types::Box<std::Map<String,std::Set<Bool>>>]{let s:std::Set<Bool>=std::set_new();let full=std::set_insert(s,true);let inner:std::Map<String,std::Set<Bool>>=std::map_new();let nested=std::map_insert(inner,\"λ😀\",full);let outer:std::Map<Int,dep::types::Box<std::Map<String,std::Set<Bool>>>>=std::map_new();return std::map_values(std::map_insert(outer,7,dep::types::Box::<std::Map<String,std::Set<Bool>>>{value:nested}));}");
        let inner = out.value().array_item(0).unwrap().field(0).unwrap();
        assert_eq!(inner.map_entry(0).unwrap().0.as_str(), Some("λ😀"));
        assert_eq!(
            inner.map_entry(0).unwrap().1.set_item(0).unwrap().as_bool(),
            Some(true)
        );
        let snapshot = snap(&out, &[]);
        assert!(eq(
            &snapshot,
            &snapshot.snapshot(&mut unlimited(), &mut work()).unwrap()
        ));
    }
    #[test]
    fn snapshot_types_require_exact_element_and_receiver_ownership() {
        for source in [
            "fn main()->[Float]{let m:std::Map<Int,Int>=std::map_new();return std::map_values(m);}",
            "fn main()->[String]{let m:std::Map<Int,String>=std::map_new();return std::map_keys(m);}",
            "fn main()->[Int]{let s:std::Set<Int>=std::set_new();return std::map_values(s);}",
            "fn main()->[Int]{let m:std::Map<Int,Int>=std::map_new();return std::set_values(m);}",
            "fn main()->[Int]{let m:std::Map<Int,Int>=std::map_new();return std::map_keys(m,1);}",
            "fn main()->[Int]{let m:std::Map<Int,Int>=std::map_new();return std::map_keys::<Int>(m);}",
            "use dep::types;fn main()->[dep::types::Empty<String>]{let m:std::Map<Int,dep::types::Empty<Int>>=std::map_new();return std::map_values(m);}",
            "use dep::types;struct Box<T>{value:T}fn main()->[Box<Int>]{let m:std::Map<Int,dep::types::Box<Int>>=std::map_new();return std::map_values(m);}",
        ]{let f=Fixture::new();f.write("app/main.svr",source);assert!(compile(&f).is_err(),"{source}");}
    }
    #[test]
    fn snapshot_node_and_text_limits_include_the_entire_result_once() {
        for (source,nodes,bytes) in [
            ("fn main()->[Int]{let m:std::Map<Int,String>=std::map_new();return std::map_keys(m);}",3,0),
            ("fn main()->[Int]{let m:std::Map<Int,String>=std::map_new();let full=std::map_insert(m,1,\"λ😀\");return std::map_keys(full);}",14,18),
            ("fn main()->[String]{let m:std::Map<Int,String>=std::map_new();let full=std::map_insert(m,1,\"λ😀\");return std::map_values(full);}",14,24),
            ("fn main()->[String]{let s:std::Set<String>=std::set_new();let full=std::set_insert(s,\"λ😀\");return std::set_values(full);}",9,24),
        ]{
            let f=Fixture::new();f.write("app/main.svr",source);let p=compile(&f).unwrap();
            let mut policy=GenericVmPolicy::DEFAULT;policy.runtime.materialization.max_value_nodes=Some(nodes);policy.runtime.materialization.max_text_bytes=Some(bytes);
            let out=execute_generic_collection_package(&p,policy.clone()).unwrap();
            assert_eq!(out.remaining().materialization.max_value_nodes,Some(0));assert_eq!(out.remaining().materialization.max_text_bytes,Some(0));
            policy.runtime.materialization.max_value_nodes=Some(nodes-1);assert!(execute_generic_collection_package(&p,policy.clone()).is_err());
            if bytes>0{policy.runtime.materialization.max_value_nodes=Some(nodes);policy.runtime.materialization.max_text_bytes=Some(bytes-1);assert!(execute_generic_collection_package(&p,policy).is_err());}
        }
    }
    #[test]
    fn iteration_after_short_update_histories_matches_an_ordered_oracle() {
        for trace in 0..64 {
            let mut source=String::from("struct Out<K,V>{keys:[K],values:[V],set:[K]}fn main()->Out<Int,Int>{let mut m:std::Map<Int,Int>=std::map_new();let mut s:std::Set<Int>=std::set_new();");
            let mut code = trace;
            let mut expected: Vec<(i64, i64)> = Vec::new();
            for step in 0..3 {
                let op = code % 4;
                code /= 4;
                let key = (op % 2) as i64;
                if op < 2 {
                    let value = step + 10;
                    source.push_str(&format!(
                        "m=std::map_insert(m,{key},{value});s=std::set_insert(s,{key});"
                    ));
                    if let Some(pair) = expected.iter_mut().find(|p| p.0 == key) {
                        pair.1 = value;
                    } else {
                        expected.push((key, value));
                    }
                } else {
                    source.push_str(&format!("let mr{step}=std::map_remove(m,{key});match mr{step}{{std::Option::Some(updated)=>{{m=updated;}},std::Option::None()=>{{}}}}let sr{step}=std::set_remove(s,{key});match sr{step}{{std::Option::Some(updated)=>{{s=updated;}},std::Option::None()=>{{}}}}"));
                    expected.retain(|p| p.0 != key);
                }
            }
            source.push_str("return Out::<Int,Int>{keys:std::map_keys(m),values:std::map_values(m),set:std::set_values(s)};}");
            let out = run(&source);
            for field in 0..3 {
                assert_eq!(
                    out.value().field(field).unwrap().array_len(),
                    Some(expected.len()),
                    "trace {trace}"
                );
            }
            for (i, (key, value)) in expected.iter().enumerate() {
                assert_eq!(
                    out.value()
                        .field(0)
                        .unwrap()
                        .array_item(i)
                        .unwrap()
                        .as_int(),
                    Some(*key)
                );
                assert_eq!(
                    out.value()
                        .field(1)
                        .unwrap()
                        .array_item(i)
                        .unwrap()
                        .as_int(),
                    Some(*value)
                );
                assert_eq!(
                    out.value()
                        .field(2)
                        .unwrap()
                        .array_item(i)
                        .unwrap()
                        .as_int(),
                    Some(*key)
                );
            }
        }
    }
    #[test]
    fn derived_array_types_and_runtime_work_have_finite_boundaries() {
        let f = Fixture::new();
        f.write("app/main.svr","fn main()->[String]{let m:std::Map<Int,String>=std::map_new();return std::map_values(std::map_insert(m,1,\"λ😀\"));}");
        let p = compile(&f).unwrap();
        let out = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.max_work = out.work();
        assert!(execute_generic_collection_package(&p, policy.clone()).is_ok());
        policy.max_work -= 1;
        assert!(execute_generic_collection_package(&p, policy).is_err());
        for kind in 0..4 {
            let mut limits = GenericPackageCompilationLimits::DEFAULT;
            let expansion = &mut limits.bodies.calls.arguments.expansion;
            match kind {
                0 => expansion.max_nodes = 1,
                1 => expansion.max_edges = 0,
                2 => expansion.max_work = 0,
                _ => expansion.max_depth = 0,
            }
            assert!(compile_generic_collection_package(
                capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
                "main",
                limits
            )
            .is_err());
        }
        assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
        assert_eq!(
            p.encode_generic_package_native_image().unwrap_err(),
            "native collection feature is not enabled"
        );
    }
}

#[test]
fn published_collection_iteration_source_executes_only_in_the_explicit_profile() {
    let f = Fixture::new();
    f.write(
        "app/main.svr",
        &std::fs::read_to_string("docs/examples/intrinsic-collection-iteration.svr").unwrap(),
    );
    let p = compile_generic_collection_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .as_bool(),
        Some(true)
    );
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(compile_generic_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT
    )
    .is_err());
    assert_eq!(
        p.encode_generic_package_native_image().unwrap_err(),
        "native collection feature is not enabled"
    );
}

mod collection_entries_execution {
    use super::*;
    fn compile(
        f: &Fixture,
    ) -> Result<CompiledGenericPackage, sovra::compiler::diagnostics::Diagnostics> {
        compile_generic_collection_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
    }
    fn run(source: &str) -> GenericVmOutcome {
        let f = Fixture::new();
        f.write("app/main.svr", source);
        execute_generic_collection_package(&compile(&f).unwrap(), GenericVmPolicy::DEFAULT).unwrap()
    }
    #[test]
    fn source_entries_have_public_key_value_fields_and_generic_composition() {
        let out=run("fn entries<K:std::Key,V>(m:std::Map<K,V>)->[std::MapEntry<K,V>]{return std::map_entries(m);}fn main()->Bool{let m:std::Map<Int,String>=std::map_new();let full=std::map_insert(std::map_insert(std::map_insert(m,7,\"old\"),9,\"second\"),7,\"new\");let result=entries::<Int,String>(full);let first=result[0];let second=result[1];return std::array_len(result)==2&&first.key==7&&first.value==\"new\"&&second.key==9&&second.value==\"second\";}");
        assert_eq!(out.value().as_bool(), Some(true));
    }
    #[test]
    fn empty_entries_need_no_fabricated_value_or_record() {
        let out=run("struct Never<T>{next:Never<T>}fn main()->[std::MapEntry<Bool,Never<Int>>]{let m:std::Map<Bool,Never<Int>>=std::map_new();return std::map_entries(m);}");
        assert_eq!(out.value().array_len(), Some(0));
        assert_eq!(out.value().footprint(), (1, 0));
    }
    #[test]
    fn public_record_wrappers_charge_their_own_labels_and_preserve_source_order() {
        let out=run("fn main()->[std::MapEntry<String,String>]{let m:std::Map<String,String>=std::map_new();return std::map_entries(std::map_insert(m,\"λ\",\"😀\"));}");
        assert_eq!(out.value().footprint(), (4, 27));
        let first = out.value().array_item(0).unwrap();
        assert_eq!(first.footprint(), (3, 27));
        assert_eq!(first.field(0).unwrap().as_str(), Some("λ"));
        assert_eq!(first.field(1).unwrap().as_str(), Some("😀"));
        assert!(first.map_len().is_none());
        assert!(first.field(2).is_none());
    }
    #[test]
    fn entries_are_constructible_standard_records_and_user_homonyms_stay_distinct() {
        let out=run("struct MapEntry<K:std::Key,V>{key:K,value:V}fn main()->Bool{let expected=std::MapEntry::<Int,String>{value:\"value\",key:7};let m:std::Map<Int,String>=std::map_new();let result=std::map_entries(std::map_insert(m,7,\"value\"));return result[0]==expected;}");
        assert_eq!(out.value().as_bool(), Some(true));
        let f = Fixture::new();
        f.write("app/main.svr","struct MapEntry<K:std::Key,V>{key:K,value:V}fn main()->[MapEntry<Int,String>]{let m:std::Map<Int,String>=std::map_new();return std::map_entries(m);}");
        assert!(compile(&f).is_err());
    }
    #[test]
    fn copied_nested_arrays_do_not_change_the_original_map() {
        let out=run("fn main()->Int{let m:std::Map<Int,[Int]>=std::map_new();let full=std::map_insert(m,1,[7]);let entries=std::map_entries(full);let first=entries[0];let mut values=first.value;values[0]=99;let original=std::map_get(full,1);match original{std::Option::Some(v)=>{return v[0];},std::Option::None()=>{return 0;}}}");
        assert_eq!(out.value().as_int(), Some(7));
    }
    #[test]
    fn imported_and_nested_collection_payloads_keep_owners_through_entry_wrappers() {
        let out=run("use dep::types;fn main()->[std::MapEntry<Int,dep::types::Box<std::Map<String,std::Set<Bool>>>>]{let s:std::Set<Bool>=std::set_new();let inner:std::Map<String,std::Set<Bool>>=std::map_new();let nested=std::map_insert(inner,\"λ😀\",std::set_insert(s,true));let m:std::Map<Int,dep::types::Box<std::Map<String,std::Set<Bool>>>>=std::map_new();return std::map_entries(std::map_insert(m,7,dep::types::Box::<std::Map<String,std::Set<Bool>>>{value:nested}));}");
        let value = out.value().array_item(0).unwrap().field(1).unwrap();
        let inner = value.field(0).unwrap();
        assert_eq!(inner.map_entry(0).unwrap().0.as_str(), Some("λ😀"));
        assert_eq!(
            inner.map_entry(0).unwrap().1.set_item(0).unwrap().as_bool(),
            Some(true)
        );
        let snapshot = snap(&out, &[]);
        assert!(eq(
            &snapshot,
            &snapshot.snapshot(&mut unlimited(), &mut work()).unwrap()
        ));
    }
    #[test]
    fn entries_refuse_wrong_receiver_arity_key_bound_and_payload_ownership() {
        for source in [
            "fn main()->[std::MapEntry<Int,Float>]{let m:std::Map<Int,Int>=std::map_new();return std::map_entries(m);}",
            "fn main()->[std::MapEntry<String,Int>]{let m:std::Map<Int,String>=std::map_new();return std::map_entries(m);}",
            "fn main()->[std::MapEntry<Int,Int>]{let s:std::Set<Int>=std::set_new();return std::map_entries(s);}",
            "fn main()->[std::MapEntry<Int,Int>]{let m:std::Map<Int,Int>=std::map_new();return std::map_entries(m,1);}",
            "fn main()->[std::MapEntry<Int,Int>]{let m:std::Map<Int,Int>=std::map_new();return std::map_entries::<Int>(m);}",
            "fn main()->std::MapEntry<Float,Int>{return std::MapEntry::<Float,Int>{key:1.0,value:1};}",
            "use dep::types;fn main()->[std::MapEntry<Int,dep::types::Empty<String>>]{let m:std::Map<Int,dep::types::Empty<Int>>=std::map_new();return std::map_entries(m);}",
        ]{let f=Fixture::new();f.write("app/main.svr",source);assert!(compile(&f).is_err(),"{source}");}
    }
    #[test]
    fn full_result_limits_count_all_entry_labels_and_descendants_once() {
        for (source,nodes,bytes) in [
            ("fn main()->[std::MapEntry<Int,Int>]{let m:std::Map<Int,Int>=std::map_new();return std::map_entries(m);}",3,0),
            ("fn main()->[std::MapEntry<Int,Int>]{let m:std::Map<Int,Int>=std::map_new();let full=std::map_insert(m,1,7);return std::map_entries(full);}",16,21),
            ("fn main()->[std::MapEntry<String,String>]{let m:std::Map<String,String>=std::map_new();let full=std::map_insert(m,\"λ\",\"😀\");return std::map_entries(full);}",16,45),
            ("fn main()->[std::MapEntry<Int,String>]{let m:std::Map<Int,String>=std::map_new();let one=std::map_insert(m,1,\"λ😀\");let full=std::map_insert(one,2,\"x\");return std::map_entries(full);}",35,82),
        ]{
            let f=Fixture::new();f.write("app/main.svr",source);let p=compile(&f).unwrap();let mut policy=GenericVmPolicy::DEFAULT;policy.runtime.materialization.max_value_nodes=Some(nodes);policy.runtime.materialization.max_text_bytes=Some(bytes);
            let out=execute_generic_collection_package(&p,policy.clone()).unwrap();assert_eq!(out.remaining().materialization.max_value_nodes,Some(0));assert_eq!(out.remaining().materialization.max_text_bytes,Some(0));
            policy.runtime.materialization.max_value_nodes=Some(nodes-1);assert!(execute_generic_collection_package(&p,policy.clone()).is_err());if bytes>0{policy.runtime.materialization.max_value_nodes=Some(nodes);policy.runtime.materialization.max_text_bytes=Some(bytes-1);assert!(execute_generic_collection_package(&p,policy).is_err());}
        }
    }
    #[test]
    fn derived_entry_and_array_proofs_have_exact_finite_node_edge_and_depth_limits() {
        let f = Fixture::new();
        f.write("app/main.svr","fn main()->[std::MapEntry<Int,String>]{let m:std::Map<Int,String>=std::map_new();return std::map_entries(m);}");
        for kind in 0..3 {
            let mut limits = GenericPackageCompilationLimits::DEFAULT;
            let expansion = &mut limits.bodies.calls.arguments.expansion;
            match kind {
                0 => expansion.max_nodes = 5,
                1 => expansion.max_edges = 5,
                _ => expansion.max_depth = 3,
            }
            assert!(compile_generic_collection_package(
                capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
                "main",
                limits
            )
            .is_ok());
            let expansion = &mut limits.bodies.calls.arguments.expansion;
            match kind {
                0 => expansion.max_nodes -= 1,
                1 => expansion.max_edges -= 1,
                _ => expansion.max_depth -= 1,
            }
            assert!(compile_generic_collection_package(
                capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
                "main",
                limits
            )
            .is_err());
        }
    }
    #[test]
    fn ordered_entry_arrays_preserve_order_when_map_association_equality_ignores_it() {
        let out=run("fn main()->Bool{let m:std::Map<Int,Int>=std::map_new();let a=std::map_insert(std::map_insert(m,1,7),2,9);let b=std::map_insert(std::map_insert(m,2,9),1,7);return a==b&&(std::map_entries(a)==std::map_entries(b))==false;}");
        assert_eq!(out.value().as_bool(), Some(true));
    }
    #[test]
    fn entry_only_source_cannot_bypass_default_compiler_execution_or_native_gates() {
        let f = Fixture::new();
        f.write(
            "app/main.svr",
            "fn main(){let entry=std::MapEntry::<Int,String>{key:7,value:\"x\"};}",
        );
        let p = compile(&f).unwrap();
        execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
        assert!(compile_generic_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT
        )
        .is_err());
        assert_eq!(
            p.encode_generic_package_native_image().unwrap_err(),
            "native collection feature is not enabled"
        );
    }
    #[test]
    fn entries_after_short_update_histories_match_an_ordered_reference() {
        for trace in 0..64 {
            let mut source = String::from(
                "fn main()->[std::MapEntry<Int,Int>]{let mut m:std::Map<Int,Int>=std::map_new();",
            );
            let mut code = trace;
            let mut expected: Vec<(i64, i64)> = Vec::new();
            for step in 0..3 {
                let op = code % 4;
                code /= 4;
                let key = (op % 2) as i64;
                if op < 2 {
                    let value = step + 10;
                    source.push_str(&format!("m=std::map_insert(m,{key},{value});"));
                    if let Some(pair) = expected.iter_mut().find(|p| p.0 == key) {
                        pair.1 = value;
                    } else {
                        expected.push((key, value));
                    }
                } else {
                    source.push_str(&format!("let removed{step}=std::map_remove(m,{key});match removed{step}{{std::Option::Some(updated)=>{{m=updated;}},std::Option::None()=>{{}}}}"));
                    expected.retain(|p| p.0 != key);
                }
            }
            source.push_str("return std::map_entries(m);}");
            let out = run(&source);
            assert_eq!(
                out.value().array_len(),
                Some(expected.len()),
                "trace {trace}"
            );
            for (i, (key, value)) in expected.iter().enumerate() {
                let entry = out.value().array_item(i).unwrap();
                assert_eq!(entry.field(0).unwrap().as_int(), Some(*key));
                assert_eq!(entry.field(1).unwrap().as_int(), Some(*value));
            }
        }
    }
    #[test]
    fn shared_work_and_old_profiles_refuse_without_partial_or_invented_entries() {
        let f = Fixture::new();
        f.write("app/main.svr","fn main()->[std::MapEntry<Int,String>]{let m:std::Map<Int,String>=std::map_new();return std::map_entries(std::map_insert(m,1,\"λ😀\"));}");
        let p = compile(&f).unwrap();
        let out = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.max_work = out.work();
        assert!(execute_generic_collection_package(&p, policy.clone()).is_ok());
        policy.max_work -= 1;
        assert!(execute_generic_collection_package(&p, policy).is_err());
        assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
        assert_eq!(
            p.encode_generic_package_native_image().unwrap_err(),
            "native collection feature is not enabled"
        );
    }
}

#[test]
fn published_collection_entries_source_executes_only_in_the_explicit_profile() {
    let f = Fixture::new();
    f.write(
        "app/main.svr",
        &std::fs::read_to_string("docs/examples/intrinsic-collection-entries.svr").unwrap(),
    );
    let p = compile_generic_collection_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .as_bool(),
        Some(true)
    );
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(compile_generic_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT
    )
    .is_err());
    assert_eq!(
        p.encode_generic_package_native_image().unwrap_err(),
        "native collection feature is not enabled"
    );
}

mod collection_display_execution {
    use super::*;
    fn compile(
        f: &Fixture,
    ) -> Result<CompiledGenericPackage, sovra::compiler::diagnostics::Diagnostics> {
        compile_generic_collection_package(
            capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
            "main",
            GenericPackageCompilationLimits::DEFAULT,
        )
    }
    fn run(source: &str) -> GenericVmOutcome {
        let f = Fixture::new();
        f.write("app/main.svr", source);
        execute_generic_collection_package(&compile(&f).unwrap(), GenericVmPolicy::DEFAULT).unwrap()
    }
    #[test]
    fn empty_and_nonempty_collections_have_explicit_ordered_display() {
        for (source,expected) in [
            ("fn main()->String{let m:std::Map<Int,String>=std::map_new();return std::to_string(m);}","std::Map { }"),
            ("fn main()->String{let s:std::Set<Bool>=std::set_new();return std::to_string(s);}","std::Set { }"),
            ("fn main()->String{let m:std::Map<Int,String>=std::map_new();let full=std::map_insert(std::map_insert(std::map_insert(m,7,\"old\"),9,\"second\"),7,\"new\");return std::to_string(full);}","std::Map { 7: new, 9: second }"),
            ("fn main()->String{let s:std::Set<String>=std::set_new();return std::to_string(std::set_insert(std::set_insert(std::set_insert(s,\"λ\"),\"😀\"),\"λ\"));}","std::Set { λ, 😀 }"),
        ]{assert_eq!(run(source).value().as_str(),Some(expected));}
    }
    #[test]
    fn nested_arrays_records_options_and_collections_format_iteratively() {
        let out=run("struct Box<T>{value:T}fn main()->String{let s:std::Set<String>=std::set_new();let full=std::set_insert(std::set_insert(s,\"λ\"),\"😀\");let m:std::Map<Int,std::Set<String>>=std::map_new();let nested=std::map_insert(m,1,full);let value=std::Option::<Box<[std::Map<Int,std::Set<String>>]>>::Some(Box::<[std::Map<Int,std::Set<String>>]>{value:[nested]});return std::to_string(value);}");
        assert_eq!(
            out.value().as_str(),
            Some("std::Option::Some(Box { value: [std::Map { 1: std::Set { λ, 😀 } }] })")
        );
    }
    #[test]
    fn public_entries_keep_record_display_while_private_entries_are_key_value_pairs() {
        let out=run("fn main()->String{let m:std::Map<Int,String>=std::map_new();return std::to_string(std::map_entries(std::map_insert(m,1,\"x\")));}");
        assert_eq!(
            out.value().as_str(),
            Some("[std::MapEntry { key: 1, value: x }]")
        );
    }
    #[test]
    fn generic_display_retains_raw_strings_and_existing_unit_and_float_conventions() {
        let out=run("fn show<T>(value:T)->String{return std::to_string(value);}fn main()->String{let m:std::Map<String,[Float]>=std::map_new();return show::<std::Map<String,[Float]>>(std::map_insert(m,\"a\\nb\",[1.5,2.0]));}");
        assert_eq!(out.value().as_str(), Some("std::Map { a\nb: [1.5, 2] }"));
        let out=run("fn unit(){}fn main()->String{let m:std::Map<Bool,Unit>=std::map_new();return std::to_string(std::map_insert(m,true,unit()));}");
        assert_eq!(out.value().as_str(), Some("std::Map { true:  }"));
    }
    #[test]
    fn formatting_equal_associations_keeps_each_snapshots_insertion_order() {
        let out=run("fn main()->Bool{let m:std::Map<Int,Int>=std::map_new();let a=std::map_insert(std::map_insert(m,1,7),2,9);let b=std::map_insert(std::map_insert(m,2,9),1,7);return a==b&&(std::to_string(a)==std::to_string(b))==false;}");
        assert_eq!(out.value().as_bool(), Some(true));
    }
    #[test]
    fn output_and_temporary_task_admission_have_exact_node_and_utf8_boundaries() {
        for (source,nodes,bytes) in [
            ("fn main()->String{let m:std::Map<Int,String>=std::map_new();return std::to_string(m);}",8,12),
            ("fn main()->String{let m:std::Map<String,String>=std::map_new();let full=std::map_insert(m,\"λ\",\"😀\");return std::to_string(full);}",30,39),
            ("fn main()->String{let s:std::Set<String>=std::set_new();let full=std::set_insert(s,\"λ\");return std::to_string(full);}",17,21),
        ]{
            let f=Fixture::new();f.write("app/main.svr",source);let p=compile(&f).unwrap();let mut policy=GenericVmPolicy::DEFAULT;policy.runtime.materialization.max_value_nodes=Some(nodes);policy.runtime.materialization.max_text_bytes=Some(bytes);
            let out=execute_generic_collection_package(&p,policy.clone()).unwrap();assert_eq!(out.remaining().materialization.max_value_nodes,Some(0));assert_eq!(out.remaining().materialization.max_text_bytes,Some(0));
            policy.runtime.materialization.max_value_nodes=Some(nodes-1);assert!(execute_generic_collection_package(&p,policy.clone()).is_err());policy.runtime.materialization.max_value_nodes=Some(nodes);policy.runtime.materialization.max_text_bytes=Some(bytes-1);assert!(execute_generic_collection_package(&p,policy).is_err());
        }
    }
    #[test]
    fn shared_work_instruction_and_cancellation_limits_refuse_without_display_fallback() {
        use sovra::compiler::execution::{CancellationToken, ExecutionControl};
        let f = Fixture::new();
        f.write("app/main.svr","fn main()->String{let m:std::Map<Int,String>=std::map_new();return std::to_string(std::map_insert(m,7,\"λ😀\"));}");
        let p = compile(&f).unwrap();
        let out = execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT).unwrap();
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.max_work = out.work();
        assert!(execute_generic_collection_package(&p, policy.clone()).is_ok());
        policy.max_work -= 1;
        assert!(execute_generic_collection_package(&p, policy).is_err());
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.runtime.execution.max_instructions = Some(out.instructions() - 1);
        assert!(execute_generic_collection_package(&p, policy).is_err());
        let token = CancellationToken::new();
        token.cancel();
        let mut policy = GenericVmPolicy::DEFAULT;
        policy.control = ExecutionControl {
            cancellation: Some(token),
            ..ExecutionControl::UNLIMITED
        };
        assert!(execute_generic_collection_package(&p, policy).is_err());
        assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
        assert_eq!(
            p.encode_generic_package_native_image().unwrap_err(),
            "native collection feature is not enabled"
        );
    }
    #[test]
    fn recursive_map_record_values_format_without_recursive_rust_traversal() {
        let out=run("struct Node<T>{children:std::Map<Int,Node<T>>,value:T}fn main()->String{let empty:std::Map<Int,Node<Int>>=std::map_new();let mut node=Node::<Int>{children:empty,value:0};let mut i=0;while i<128{let children=std::map_insert(empty,1,node);node=Node::<Int>{children:children,value:i};i=i+1;}return std::to_string(node);}");
        let mut expected = String::from("Node { children: std::Map { }, value: 0 }");
        for i in 0..128 {
            expected = format!("Node {{ children: std::Map {{ 1: {expected} }}, value: {i} }}");
        }
        assert_eq!(out.value().as_str(), Some(expected.as_str()));
    }
    #[test]
    fn display_after_short_update_histories_matches_an_ordered_text_oracle() {
        for trace in 0..64 {
            let mut source=String::from("struct Out<T>{map:T,set:T}fn main()->Out<String>{let mut m:std::Map<Int,Int>=std::map_new();let mut s:std::Set<Int>=std::set_new();");
            let mut code = trace;
            let mut expected: Vec<(i64, i64)> = Vec::new();
            for step in 0..3 {
                let op = code % 4;
                code /= 4;
                let key = (op % 2) as i64;
                if op < 2 {
                    let value = step + 10;
                    source.push_str(&format!(
                        "m=std::map_insert(m,{key},{value});s=std::set_insert(s,{key});"
                    ));
                    if let Some(pair) = expected.iter_mut().find(|p| p.0 == key) {
                        pair.1 = value;
                    } else {
                        expected.push((key, value));
                    }
                } else {
                    source.push_str(&format!("let mr{step}=std::map_remove(m,{key});match mr{step}{{std::Option::Some(updated)=>{{m=updated;}},std::Option::None()=>{{}}}}let sr{step}=std::set_remove(s,{key});match sr{step}{{std::Option::Some(updated)=>{{s=updated;}},std::Option::None()=>{{}}}}"));
                    expected.retain(|p| p.0 != key);
                }
            }
            source.push_str("return Out::<String>{map:std::to_string(m),set:std::to_string(s)};}");
            let out = run(&source);
            let pairs = expected
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join(", ");
            let keys = expected
                .iter()
                .map(|(k, _)| k.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let map = if expected.is_empty() {
                "std::Map { }".into()
            } else {
                format!("std::Map {{ {pairs} }}")
            };
            let set = if expected.is_empty() {
                "std::Set { }".into()
            } else {
                format!("std::Set {{ {keys} }}")
            };
            assert_eq!(out.value().field(0).unwrap().as_str(), Some(map.as_str()));
            assert_eq!(out.value().field(1).unwrap().as_str(), Some(set.as_str()));
        }
    }
}

#[test]
fn published_collection_display_source_executes_only_in_the_explicit_profile() {
    let f = Fixture::new();
    f.write(
        "app/main.svr",
        &std::fs::read_to_string("docs/examples/intrinsic-collection-display.svr").unwrap(),
    );
    let p = compile_generic_collection_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        execute_generic_collection_package(&p, GenericVmPolicy::DEFAULT)
            .unwrap()
            .value()
            .as_bool(),
        Some(true)
    );
    assert!(execute_generic_package(&p, GenericVmPolicy::DEFAULT).is_err());
    assert!(compile_generic_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT
    )
    .is_err());
    assert_eq!(
        p.encode_generic_package_native_image().unwrap_err(),
        "native collection feature is not enabled"
    );
}

#[test]
fn collection_image_encoding_keeps_snapshots_after_project_deletion_and_old_encoders_strict() {
    let f = Fixture::new();
    f.write("app/main.svr","fn main(){let m:std::Map<Int,String>=std::map_new();let full=std::map_insert(m,7,\"λ😀\");if std::to_string(full)!=\"std::Map { 7: λ😀 }\"{let bad=1/0;}}");
    let root = f.0.clone();
    let p = compile_generic_collection_package(
        capture_package_sources(f.0.join("app"), PackageCaptureLimits::DEFAULT).unwrap(),
        "main",
        GenericPackageCompilationLimits::DEFAULT,
    )
    .unwrap();
    drop(f);
    let image = p.encode_generic_collection_package_native_image().unwrap();
    assert_eq!(&image[8..12], &1023u32.to_le_bytes());
    assert!(!String::from_utf8_lossy(&image).contains(root.to_str().unwrap()));
    let decoded = decode_generic_collection_package_native_image(&image).unwrap();
    assert!(decoded
        .bodies()
        .types()
        .declarations()
        .inputs()
        .captured()
        .is_none());
    assert_eq!(
        decoded
            .encode_generic_collection_package_native_image()
            .unwrap(),
        image
    );
    assert!(execute_generic_collection_package_native_image(&image).is_ok());
    assert!(p.encode_generic_package_native_image().is_err());
    assert!(decode_generic_package_native_image(&image).is_err());
}
#[test]
fn ordinary_compiled_packages_do_not_silently_promote_to_collection_images() {
    let f = Fixture::new();
    f.write("app/main.svr", "fn main(){}");
    let old = f.compile();
    assert!(old.encode_generic_package_native_image().is_ok());
    assert_eq!(
        old.encode_generic_collection_package_native_image()
            .unwrap_err(),
        "native collection profile requires collection compilation"
    );
}
