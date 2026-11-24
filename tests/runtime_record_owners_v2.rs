//! Sealed record ingress, nominal ownership, full descendant costs and comparison work.
use sovra::compiler::{execution::MaterializationLimits as Limits, parser::Parser, v2::*};
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
fn env(context: &str, enums: &[&str]) -> CheckedEnumTypes {
    let sources = enums
        .iter()
        .map(|s| EnumSource {
            label: s,
            source: s,
        })
        .collect::<Vec<_>>();
    check_enum_types(
        "owner",
        "context",
        &Parser::new().parse_source(context).unwrap(),
        &sources,
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
fn string(s: &str) -> RuntimeValueV2 {
    RuntimeValueV2::string(s, &mut unlimited()).unwrap()
}
fn row(e: &CheckedEnumTypes) -> RuntimeValueV2 {
    RuntimeValueV2::checked_record(
        e,
        e.find_record("Row").unwrap(),
        vec![("text", string("λ"))],
        &mut unlimited(),
        &mut work(),
    )
    .unwrap()
}
fn empty(e: &CheckedEnumTypes) -> RuntimeMapV2 {
    RuntimeMapV2::new(
        RuntimeKeyType::Int,
        RuntimeMapValueType::new(
            RuntimeMapPrimitive::Record(e.find_record("Row").unwrap()),
            0,
        )
        .unwrap(),
        &mut unlimited(),
    )
    .unwrap()
}
fn cost(n: u64, t: u64, op: impl Fn(&mut RuntimeValueFuel) -> Result<(), RuntimeMapError>) {
    let mut f = fuel(n, t);
    op(&mut f).unwrap();
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    for (n, t, error) in [
        (n - 1, t, RuntimeValueError::NodeLimit),
        (n, t - 1, RuntimeValueError::TextLimit),
    ] {
        let mut f = fuel(n, t);
        let before = f.remaining();
        assert_eq!(
            op(&mut f),
            Err(RuntimeMapError::Collection(
                RuntimeSetError::Materialization(error)
            ))
        );
        assert_eq!(f.remaining(), before);
    }
}
#[test]
fn checked_schema_ids_are_stable_in_clones_and_distinct_across_environments() {
    let a = env("struct Row{text:String}", &[]);
    let clone = a.clone();
    let b = env("struct Row{text:String}", &[]);
    assert_eq!(a.find_record("Row"), clone.find_record("Row"));
    assert_ne!(a.find_record("Row"), b.find_record("Row"));
    assert_eq!(a.find_record("Missing"), None);
    let r = row(&a);
    assert_eq!(r.record_owner(), a.find_record("Row"));
    assert_eq!(
        r.snapshot(&mut unlimited()).unwrap().record_owner(),
        a.find_record("Row")
    );
    assert!(!r.structurally_equal(&row(&b)));
    assert!(r.structurally_equal(&row(&clone)));
    let mut f = fuel(0, 0);
    assert_eq!(
        RuntimeValueV2::checked_record(
            &b,
            a.find_record("Row").unwrap(),
            vec![("text", string("λ"))],
            &mut f,
            &mut work()
        )
        .unwrap_err(),
        RuntimeMapError::ValueType
    );
    assert_eq!(
        empty(&a)
            .insert(&RuntimeKey::Int(1), &row(&b), &mut f, &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
    assert_eq!(
        empty(&a).equal(&empty(&b), &mut work()),
        Err(RuntimeMapError::ValueType)
    );
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
}
#[test]
fn name_only_record_cannot_forge_a_sealed_value() {
    let e = env("struct Row{text:String}", &[]);
    let raw = RuntimeValueV2::record("Row", vec![("text", string("λ"))], &mut unlimited()).unwrap();
    assert_eq!(raw.record_owner(), None);
    assert!(!raw.structurally_equal(&row(&e)));
    assert_eq!(
        empty(&e)
            .insert(&RuntimeKey::Int(1), &raw, &mut unlimited(), &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
    let other =
        RuntimeValueV2::record("Row", vec![("text", string("λ"))], &mut unlimited()).unwrap();
    assert!(raw.structurally_equal(&other));
}
#[test]
fn checked_record_constructor_transfers_children_and_only_charges_wrapper_and_labels() {
    let e = env("struct Row{text:String}", &[]);
    cost(1, 7, |f| {
        RuntimeValueV2::checked_record(
            &e,
            e.find_record("Row").unwrap(),
            vec![("text", string("λ"))],
            f,
            &mut work(),
        )
        .map(|_| ())
    });
    let v = row(&e);
    let size = v.logical_footprint().unwrap();
    assert_eq!(size.value_nodes, 2);
    assert_eq!(size.owned_text_bytes, 9);
}
#[test]
fn every_map_copy_event_charges_record_labels_and_all_descendants() {
    let e = env("struct Row{text:String}", &[]);
    let r = row(&e);
    let key = RuntimeKey::Int(1);
    let m = empty(&e);
    cost(5, 9, |f| m.insert(&key, &r, f, &mut work()).map(|_| ()));
    let m = m.insert(&key, &r, &mut unlimited(), &mut work()).unwrap();
    cost(5, 9, |f| m.snapshot(f, &mut work()).map(|_| ()));
    cost(5, 9, |f| m.entries(f, &mut work()).map(|_| ()));
    cost(3, 9, |f| m.values(f, &mut work()).map(|_| ()));
    cost(2, 9, |f| m.get(&key, f, &mut work()).map(|_| ()));
    cost(3, 9, |f| m.remove(&key, f, &mut work()).map(|_| ()));
    let removed = m
        .remove(&key, &mut unlimited(), &mut work())
        .unwrap()
        .unwrap();
    assert_eq!(removed.value.record_owner(), e.find_record("Row"));
    assert!(removed.map.is_empty());
}
#[test]
fn missing_duplicate_unknown_and_wrongly_typed_fields_refuse_before_fuel() {
    let e = env("struct Pair{left:Int,right:String}", &[]);
    let id = e.find_record("Pair").unwrap();
    for fields in [
        vec![],
        vec![("left", string("x")), ("right", string("y"))],
        vec![
            ("left", RuntimeValueV2::int(1, &mut unlimited()).unwrap()),
            ("wrong", string("y")),
        ],
        vec![
            ("left", RuntimeValueV2::int(1, &mut unlimited()).unwrap()),
            ("left", string("y")),
        ],
    ] {
        let mut f = fuel(0, 0);
        assert_eq!(
            RuntimeValueV2::checked_record(&e, id, fields, &mut f, &mut work()).unwrap_err(),
            RuntimeMapError::ValueType
        );
        assert_eq!(f.remaining(), fuel(0, 0).remaining());
    }
    let e = env("struct F{number:Float}", &[]);
    assert_eq!(
        RuntimeValueV2::checked_record(
            &e,
            e.find_record("F").unwrap(),
            vec![("number", RuntimeValueV2::int(1, &mut unlimited()).unwrap())],
            &mut unlimited(),
            &mut work()
        )
        .unwrap_err(),
        RuntimeMapError::ValueType
    );
}
#[test]
fn nested_record_and_array_fields_require_exact_owners() {
    let e = env("struct Row{text:String}\nstruct Group{rows:[Row]}", &[]);
    let foreign = env("struct Row{text:String}", &[]);
    let rows = RuntimeValueV2::array(vec![row(&e)], &mut unlimited()).unwrap();
    let group = RuntimeValueV2::checked_record(
        &e,
        e.find_record("Group").unwrap(),
        vec![("rows", rows)],
        &mut unlimited(),
        &mut work(),
    )
    .unwrap();
    let child = group
        .field_snapshot("rows", &mut unlimited())
        .unwrap()
        .array_item_snapshot(0, &mut unlimited())
        .unwrap();
    assert_eq!(child.record_owner(), e.find_record("Row"));
    let rows = RuntimeValueV2::array(vec![row(&foreign)], &mut unlimited()).unwrap();
    assert_eq!(
        RuntimeValueV2::checked_record(
            &e,
            e.find_record("Group").unwrap(),
            vec![("rows", rows)],
            &mut unlimited(),
            &mut work()
        )
        .unwrap_err(),
        RuntimeMapError::ValueType
    );
}
#[test]
fn enum_payload_records_require_sealed_ownership_transitively() {
    let e = env("struct Row{text:String}", &["enum Boxed{Item(Row)}"]);
    let foreign = env("struct Row{text:String}", &[]);
    let id = e.find("Boxed").unwrap();
    assert!(RuntimeValueV2::enum_value(&e, id, 0, Some(row(&foreign)), &mut unlimited()).is_err());
    let v = RuntimeValueV2::enum_value(&e, id, 0, Some(row(&e)), &mut unlimited()).unwrap();
    let m = RuntimeMapV2::new(
        RuntimeKeyType::Int,
        RuntimeMapValueType::new(RuntimeMapPrimitive::Enum(id), 0).unwrap(),
        &mut unlimited(),
    )
    .unwrap();
    assert!(m
        .insert(&RuntimeKey::Int(1), &v, &mut unlimited(), &mut work())
        .is_ok());
    let raw = RuntimeValueV2::record("Row", vec![("text", string("λ"))], &mut unlimited()).unwrap();
    let v = RuntimeValueV2::enum_value(&e, id, 0, Some(raw), &mut unlimited()).unwrap();
    assert_eq!(
        m.insert(&RuntimeKey::Int(1), &v, &mut unlimited(), &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
}
#[test]
fn field_order_does_not_change_equality_and_lookup_searches_consume_work() {
    let e = env("struct Pair{a:Int,b:Int}", &[]);
    let id = e.find_record("Pair").unwrap();
    let build = |fields| {
        RuntimeValueV2::checked_record(&e, id, fields, &mut unlimited(), &mut work()).unwrap()
    };
    let a = build(vec![
        ("a", RuntimeValueV2::int(1, &mut unlimited()).unwrap()),
        ("b", RuntimeValueV2::int(2, &mut unlimited()).unwrap()),
    ]);
    let b = build(vec![
        ("b", RuntimeValueV2::int(2, &mut unlimited()).unwrap()),
        ("a", RuntimeValueV2::int(1, &mut unlimited()).unwrap()),
    ]);
    assert!(a.structurally_equal(&b));
    let empty = RuntimeMapV2::new(
        RuntimeKeyType::Int,
        RuntimeMapValueType::new(RuntimeMapPrimitive::Record(id), 0).unwrap(),
        &mut unlimited(),
    )
    .unwrap();
    let a = empty
        .insert(&RuntimeKey::Int(1), &a, &mut unlimited(), &mut work())
        .unwrap();
    let b = empty
        .insert(&RuntimeKey::Int(1), &b, &mut unlimited(), &mut work())
        .unwrap();
    assert!(a.equal(&b, &mut work()).unwrap());
    // Length and key lookup 2; linear nodes/text 3+3+6+6=18; three label searches at 3 each=9.
    let mut w = RuntimeSetWork::new(28);
    assert_eq!(
        a.equal(&b, &mut w),
        Err(RuntimeMapError::Collection(RuntimeSetError::WorkLimit))
    );
    let mut w = RuntimeSetWork::new(29);
    assert!(a.equal(&b, &mut w).unwrap());
    assert_eq!(w.remaining(), 0);
}
#[test]
fn exact_constructor_and_ingress_work_refuse_without_committing_fuel() {
    let e = env("struct Row{text:String}", &[]);
    let id = e.find_record("Row").unwrap();
    let mut f = fuel(1, 7);
    let mut w = RuntimeSetWork::new(15);
    assert_eq!(
        RuntimeValueV2::checked_record(&e, id, vec![("text", string("λ"))], &mut f, &mut w)
            .unwrap_err(),
        RuntimeMapError::Collection(RuntimeSetError::WorkLimit)
    );
    assert_eq!(f.remaining(), fuel(1, 7).remaining());
    let mut w = RuntimeSetWork::new(16);
    let r = RuntimeValueV2::checked_record(&e, id, vec![("text", string("λ"))], &mut f, &mut w)
        .unwrap();
    assert_eq!(w.remaining(), 0);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    let mut f = fuel(5, 9);
    let mut w = RuntimeSetWork::new(20);
    assert!(empty(&e)
        .insert(&RuntimeKey::Int(1), &r, &mut f, &mut w)
        .is_err());
    assert_eq!(f.remaining(), fuel(5, 9).remaining());
    let mut w = RuntimeSetWork::new(21);
    empty(&e)
        .insert(&RuntimeKey::Int(1), &r, &mut f, &mut w)
        .unwrap();
    assert_eq!(w.remaining(), 0);
}

#[test]
fn record_array_leaves_retain_owner_and_reject_unsealed_members() {
    let e = env("struct Row{text:String}", &[]);
    let id = e.find_record("Row").unwrap();
    let m = RuntimeMapV2::new(
        RuntimeKeyType::Int,
        RuntimeMapValueType::new(RuntimeMapPrimitive::Record(id), 1).unwrap(),
        &mut unlimited(),
    )
    .unwrap();
    let good = RuntimeValueV2::array(vec![row(&e)], &mut unlimited()).unwrap();
    let m = m
        .insert(&RuntimeKey::Int(1), &good, &mut unlimited(), &mut work())
        .unwrap();
    let got = m
        .get(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
        .unwrap()
        .unwrap();
    assert_eq!(
        got.array_item_snapshot(0, &mut unlimited())
            .unwrap()
            .record_owner(),
        Some(id)
    );
    let raw = RuntimeValueV2::record("Row", vec![("text", string("λ"))], &mut unlimited()).unwrap();
    let mixed = RuntimeValueV2::array(vec![row(&e), raw], &mut unlimited()).unwrap();
    assert_eq!(
        m.insert(&RuntimeKey::Int(2), &mixed, &mut unlimited(), &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
}
