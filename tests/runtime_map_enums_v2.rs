//! Checked enum ownership and complete label/payload events at map ingress.
use sovra::compiler::{execution::MaterializationLimits as Limits, parser::Parser, v2::*};
fn unlimited() -> RuntimeValueFuel {
    RuntimeValueFuel::new(Limits::UNLIMITED)
}
fn fuel(nodes: u64, text: u64) -> RuntimeValueFuel {
    RuntimeValueFuel::new(Limits {
        max_value_nodes: Some(nodes),
        max_text_bytes: Some(text),
    })
}
fn work() -> RuntimeSetWork {
    RuntimeSetWork::new(1_000_000)
}
fn env(context: &str, declarations: &[&str]) -> CheckedEnumTypes {
    let sources = declarations
        .iter()
        .map(|s| EnumSource {
            label: s,
            source: s,
        })
        .collect::<Vec<_>>();
    check_enum_types(
        "o",
        "c",
        &Parser::new().parse_source(context).unwrap(),
        &sources,
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
fn choice() -> CheckedEnumTypes {
    env("", &["enum Choice{Text(String),None}"])
}
fn value(
    env: &CheckedEnumTypes,
    name: &str,
    tag: usize,
    payload: Option<RuntimeValueV2>,
) -> RuntimeValueV2 {
    RuntimeValueV2::enum_value(env, env.find(name).unwrap(), tag, payload, &mut unlimited())
        .unwrap()
}
fn string(s: &str) -> RuntimeValueV2 {
    RuntimeValueV2::string(s, &mut unlimited()).unwrap()
}
fn empty(env: &CheckedEnumTypes, name: &str, depth: u8) -> RuntimeMapV2 {
    RuntimeMapV2::new(
        RuntimeKeyType::Int,
        RuntimeMapValueType::new(RuntimeMapPrimitive::Enum(env.find(name).unwrap()), depth)
            .unwrap(),
        &mut unlimited(),
    )
    .unwrap()
}
fn cost(
    nodes: u64,
    text: u64,
    operation: impl Fn(&mut RuntimeValueFuel) -> Result<(), RuntimeMapError>,
) {
    let mut f = fuel(nodes, text);
    operation(&mut f).unwrap();
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    for (n, t, e) in [
        (nodes - 1, text, RuntimeValueError::NodeLimit),
        (nodes, text - 1, RuntimeValueError::TextLimit),
    ] {
        let mut f = fuel(n, t);
        let before = f.remaining();
        assert_eq!(
            operation(&mut f),
            Err(RuntimeMapError::Collection(
                RuntimeSetError::Materialization(e)
            ))
        );
        assert_eq!(f.remaining(), before);
    }
}
#[test]
fn nominal_enum_identity_is_retained_including_empty_maps() {
    let a = choice();
    let b = choice();
    let clone = a.clone();
    let m = empty(&a, "Choice", 0);
    assert_eq!(
        m.value_type().leaf(),
        RuntimeMapPrimitive::Enum(a.find("Choice").unwrap())
    );
    assert!(m.equal(&empty(&clone, "Choice", 0), &mut work()).unwrap());
    assert_eq!(
        m.equal(&empty(&b, "Choice", 0), &mut work()),
        Err(RuntimeMapError::ValueType)
    );
    let foreign = value(&b, "Choice", 1, None);
    let mut f = fuel(0, 0);
    assert_eq!(
        m.insert(&RuntimeKey::Int(1), &foreign, &mut f, &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    let e = env("", &["enum E{None}", "enum F{None}"]);
    let m = empty(&e, "E", 0);
    assert_eq!(
        m.insert(
            &RuntimeKey::Int(1),
            &value(&e, "F", 0, None),
            &mut unlimited(),
            &mut work()
        )
        .unwrap_err(),
        RuntimeMapError::ValueType
    );
}
#[test]
fn labels_and_payload_are_charged_by_every_snapshot_event() {
    let e = choice();
    let m = empty(&e, "Choice", 0);
    let v = value(&e, "Choice", 0, Some(string("λ")));
    let key = RuntimeKey::Int(1);
    assert_eq!(v.logical_footprint().unwrap().owned_text_bytes, 12);
    cost(5, 12, |f| m.insert(&key, &v, f, &mut work()).map(|_| ()));
    let m = m.insert(&key, &v, &mut unlimited(), &mut work()).unwrap();
    cost(5, 12, |f| m.snapshot(f, &mut work()).map(|_| ()));
    cost(5, 12, |f| m.entries(f, &mut work()).map(|_| ()));
    cost(3, 12, |f| m.values(f, &mut work()).map(|_| ()));
    cost(2, 12, |f| m.get(&key, f, &mut work()).map(|_| ()));
    cost(3, 12, |f| m.remove(&key, f, &mut work()).map(|_| ()));
    // Replacing with a nullary variant discards payload ownership from result cost.
    let none = value(&e, "Choice", 1, None);
    cost(4, 10, |f| m.insert(&key, &none, f, &mut work()).map(|_| ()));
    let mut f = fuel(5, 12);
    let snapshot = m.snapshot(&mut f, &mut work()).unwrap();
    drop(snapshot);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
}
#[test]
fn enum_arrays_keep_exact_nominal_leaf_and_charge_every_variant() {
    let e = choice();
    let foreign = choice();
    let m = empty(&e, "Choice", 1);
    let v = RuntimeValueV2::array(
        vec![
            value(&e, "Choice", 0, Some(string("λ"))),
            value(&e, "Choice", 1, None),
        ],
        &mut unlimited(),
    )
    .unwrap();
    cost(7, 22, |f| {
        m.insert(&RuntimeKey::Int(1), &v, f, &mut work())
            .map(|_| ())
    });
    let mixed = RuntimeValueV2::array(
        vec![
            value(&e, "Choice", 1, None),
            value(&foreign, "Choice", 1, None),
        ],
        &mut unlimited(),
    )
    .unwrap();
    let mut f = fuel(0, 0);
    assert_eq!(
        m.insert(&RuntimeKey::Int(1), &mixed, &mut f, &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    let raw = RuntimeValueV2::array(vec![], &mut unlimited()).unwrap();
    let a = m
        .insert(&RuntimeKey::Int(1), &raw, &mut unlimited(), &mut work())
        .unwrap();
    let b = empty(&foreign, "Choice", 1)
        .insert(&RuntimeKey::Int(1), &raw, &mut unlimited(), &mut work())
        .unwrap();
    assert_eq!(a.equal(&b, &mut work()), Err(RuntimeMapError::ValueType));
}
#[test]
fn nested_enums_charge_all_owned_labels_and_compare_payloads() {
    let e = env(
        "",
        &[
            "enum Inner{Text(String),Nothing}",
            "enum Outer{Wrap(Inner),Empty}",
        ],
    );
    let v = value(
        &e,
        "Outer",
        0,
        Some(value(&e, "Inner", 0, Some(string("λ")))),
    );
    let m = empty(&e, "Outer", 0);
    cost(6, 20, |f| {
        m.insert(&RuntimeKey::Int(1), &v, f, &mut work())
            .map(|_| ())
    });
    let a = m
        .insert(&RuntimeKey::Int(1), &v, &mut unlimited(), &mut work())
        .unwrap();
    let b = m
        .insert(
            &RuntimeKey::Int(1),
            &value(&e, "Outer", 0, Some(value(&e, "Inner", 1, None))),
            &mut unlimited(),
            &mut work(),
        )
        .unwrap();
    assert!(!a.equal(&b, &mut work()).unwrap());
    assert!(a
        .equal(
            &a.snapshot(&mut unlimited(), &mut work()).unwrap(),
            &mut work()
        )
        .unwrap());
}
#[test]
fn record_payloads_remain_gated_even_when_existing_enum_constructor_accepts_them() {
    let e = env("struct Row{value:Int}", &["enum Boxed{Item(Row),Empty}"]);
    let raw = RuntimeValueV2::record(
        "Row",
        vec![("value", RuntimeValueV2::int(1, &mut unlimited()).unwrap())],
        &mut unlimited(),
    )
    .unwrap();
    let boxed = value(&e, "Boxed", 0, Some(raw));
    let m = empty(&e, "Boxed", 0);
    let mut f = fuel(0, 0);
    assert_eq!(
        m.insert(&RuntimeKey::Int(1), &boxed, &mut f, &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    assert!(m
        .insert(
            &RuntimeKey::Int(1),
            &value(&e, "Boxed", 1, None),
            &mut unlimited(),
            &mut work()
        )
        .is_ok());
    // A record deeper under nested enum/array payloads must not bypass the gate.
    let e = env(
        "struct Row{value:Int}",
        &["enum Boxed{Items([Row])}", "enum Outer{Wrap(Boxed)}"],
    );
    let raw = RuntimeValueV2::record(
        "Row",
        vec![("value", RuntimeValueV2::int(1, &mut unlimited()).unwrap())],
        &mut unlimited(),
    )
    .unwrap();
    let rows = RuntimeValueV2::array(vec![raw], &mut unlimited()).unwrap();
    let outer = value(&e, "Outer", 0, Some(value(&e, "Boxed", 0, Some(rows))));
    assert_eq!(
        empty(&e, "Outer", 0)
            .insert(&RuntimeKey::Int(1), &outer, &mut f, &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
}
#[test]
fn ingress_work_refuses_before_copying_large_array_payloads() {
    let e = env("", &["enum Rows{Values([Int])}"]);
    let ints = (0..100)
        .map(|n| RuntimeValueV2::int(n, &mut unlimited()).unwrap())
        .collect();
    let v = value(
        &e,
        "Rows",
        0,
        Some(RuntimeValueV2::array(ints, &mut unlimited()).unwrap()),
    );
    let mut f = fuel(1000, 1000);
    let before = f.remaining();
    let mut w = RuntimeSetWork::new(4);
    assert_eq!(
        empty(&e, "Rows", 0)
            .insert(&RuntimeKey::Int(1), &v, &mut f, &mut w)
            .unwrap_err(),
        RuntimeMapError::Collection(RuntimeSetError::WorkLimit)
    );
    assert_eq!(f.remaining(), before);
}
#[test]
fn recursive_enum_payload_copy_comparison_and_drop_remain_iterative() {
    let e = env("", &["enum Chain{Next(Chain),End}"]);
    let mut v = value(&e, "Chain", 1, None);
    for _ in 0..2048 {
        v = value(&e, "Chain", 0, Some(v));
    }
    let m = empty(&e, "Chain", 0)
        .insert(&RuntimeKey::Int(1), &v, &mut unlimited(), &mut work())
        .unwrap();
    let copy = m.snapshot(&mut unlimited(), &mut work()).unwrap();
    assert!(copy.equal(&m, &mut work()).unwrap());
    let got = copy
        .get(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
        .unwrap()
        .unwrap();
    assert!(got.structurally_equal(&v));
    drop(got);
    drop(copy);
    drop(m);
    drop(v);
}
#[test]
fn primitive_key_domains_and_forged_enum_text_do_not_bypass_nominal_ingress() {
    let e = choice();
    let v = value(&e, "Choice", 1, None);
    let m = RuntimeMapV2::new(
        RuntimeKeyType::Int,
        RuntimeMapValueType::new(RuntimeMapPrimitive::String, 0).unwrap(),
        &mut unlimited(),
    )
    .unwrap();
    assert_eq!(
        m.insert(&RuntimeKey::Int(1), &v, &mut unlimited(), &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
    let enum_map = empty(&e, "Choice", 0);
    assert_eq!(
        enum_map
            .insert(
                &RuntimeKey::String("Choice".into()),
                &v,
                &mut unlimited(),
                &mut work()
            )
            .unwrap_err(),
        RuntimeMapError::Collection(RuntimeSetError::KeyType)
    );
    assert_eq!(
        enum_map
            .insert(
                &RuntimeKey::Int(1),
                &string("Choice::None()"),
                &mut unlimited(),
                &mut work()
            )
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
}

#[test]
fn exact_enum_ingress_get_and_equality_work_preserve_atomic_fuel() {
    let e = choice();
    let v = value(&e, "Choice", 0, Some(string("λ")));
    let m = empty(&e, "Choice", 0);
    let mut f = fuel(5, 12);
    // Root, payload edge, String node: 3. Complete result copy: 5 + 12.
    let mut w = RuntimeSetWork::new(19);
    assert_eq!(
        m.insert(&RuntimeKey::Int(1), &v, &mut f, &mut w)
            .unwrap_err(),
        RuntimeMapError::Collection(RuntimeSetError::WorkLimit)
    );
    assert_eq!(w.remaining(), 16);
    assert_eq!(f.remaining(), fuel(5, 12).remaining());
    let mut w = RuntimeSetWork::new(20);
    let m = m.insert(&RuntimeKey::Int(1), &v, &mut f, &mut w).unwrap();
    assert_eq!(w.remaining(), 0);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    let mut f = fuel(2, 12);
    let mut w = RuntimeSetWork::new(15);
    m.get(&RuntimeKey::Int(1), &mut f, &mut w).unwrap().unwrap();
    assert_eq!(w.remaining(), 0);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    let mut w = RuntimeSetWork::new(30);
    assert!(m.equal(&m, &mut w).unwrap());
    assert_eq!(w.remaining(), 0);
}
