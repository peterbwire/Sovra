//! Typed opaque maps: ordering, descendant ownership, exact admission and refusal.
use sovra::compiler::{execution::MaterializationLimits as Limits, v2::*};
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
fn ty(leaf: RuntimeMapPrimitive, depth: u8) -> RuntimeMapValueType {
    RuntimeMapValueType::new(leaf, depth).unwrap()
}
fn int(v: i64) -> RuntimeValueV2 {
    RuntimeValueV2::int(v, &mut unlimited()).unwrap()
}
fn string(v: &str) -> RuntimeValueV2 {
    RuntimeValueV2::string(v, &mut unlimited()).unwrap()
}
fn map(pairs: &[(i64, i64)]) -> RuntimeMapV2 {
    let mut m = RuntimeMapV2::new(
        RuntimeKeyType::Int,
        ty(RuntimeMapPrimitive::Int, 0),
        &mut unlimited(),
    )
    .unwrap();
    for (k, v) in pairs {
        m = m
            .insert(
                &RuntimeKey::Int(*k),
                &int(*v),
                &mut unlimited(),
                &mut work(),
            )
            .unwrap()
    }
    m
}
fn cost(
    nodes: u64,
    text: u64,
    operation: impl Fn(&mut RuntimeValueFuel) -> Result<(), RuntimeMapError>,
) {
    let mut f = fuel(nodes, text);
    operation(&mut f).unwrap();
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    if nodes > 0 {
        let mut f = fuel(nodes - 1, text);
        let before = f.remaining();
        assert_eq!(
            operation(&mut f),
            Err(RuntimeMapError::Collection(
                RuntimeSetError::Materialization(RuntimeValueError::NodeLimit)
            ))
        );
        assert_eq!(f.remaining(), before);
    }
    if text > 0 {
        let mut f = fuel(nodes, text - 1);
        let before = f.remaining();
        assert_eq!(
            operation(&mut f),
            Err(RuntimeMapError::Collection(
                RuntimeSetError::Materialization(RuntimeValueError::TextLimit)
            ))
        );
        assert_eq!(f.remaining(), before);
    }
}
#[test]
fn creation_keeps_domains_and_rejects_invalid_type_depth() {
    cost(1, 0, |f| {
        RuntimeMapV2::new(RuntimeKeyType::Bool, ty(RuntimeMapPrimitive::String, 2), f).map(|_| ())
    });
    let m = map(&[]);
    assert!(m.is_empty());
    assert_eq!(m.len(), 0);
    assert_eq!(m.key_type(), RuntimeKeyType::Int);
    assert_eq!(m.value_type(), ty(RuntimeMapPrimitive::Int, 0));
    assert!(RuntimeMapValueType::new(RuntimeMapPrimitive::Int, 128).is_ok());
    assert_eq!(
        RuntimeMapValueType::new(RuntimeMapPrimitive::Int, 129),
        Err(RuntimeMapError::ValueType)
    );
}
#[test]
fn replacement_keeps_position_and_removal_reinsertion_moves_last() {
    let original = map(&[(1, 10), (2, 20)]);
    let replaced = original
        .insert(&RuntimeKey::Int(1), &int(30), &mut unlimited(), &mut work())
        .unwrap();
    assert_eq!(
        replaced.keys(&mut unlimited(), &mut work()).unwrap(),
        vec![RuntimeKey::Int(1), RuntimeKey::Int(2)]
    );
    assert_eq!(
        replaced
            .get(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
            .unwrap()
            .unwrap()
            .as_int(),
        Some(30)
    );
    assert_eq!(
        original
            .get(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
            .unwrap()
            .unwrap()
            .as_int(),
        Some(10)
    );
    let removed = replaced
        .remove(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
        .unwrap()
        .unwrap();
    assert_eq!(removed.value.as_int(), Some(30));
    let again = removed
        .map
        .insert(&RuntimeKey::Int(1), &int(30), &mut unlimited(), &mut work())
        .unwrap();
    assert_eq!(
        again.keys(&mut unlimited(), &mut work()).unwrap(),
        vec![RuntimeKey::Int(2), RuntimeKey::Int(1)]
    );
    assert!(again.equal(&replaced, &mut work()).unwrap());
    assert!(!again.equal(&original, &mut work()).unwrap());
    assert!(!again.equal(&map(&[(1, 30)]), &mut work()).unwrap());
    assert!(!again.equal(&map(&[(1, 30), (9, 20)]), &mut work()).unwrap());
}
#[test]
fn missing_key_options_and_membership_do_not_materialize_results() {
    let m = map(&[(1, 10)]);
    let mut f = fuel(0, 0);
    assert!(m.contains(&RuntimeKey::Int(1), &mut work()).unwrap());
    assert!(!m.contains(&RuntimeKey::Int(2), &mut work()).unwrap());
    assert!(m
        .get(&RuntimeKey::Int(2), &mut f, &mut work())
        .unwrap()
        .is_none());
    assert!(m
        .remove(&RuntimeKey::Int(2), &mut f, &mut work())
        .unwrap()
        .is_none());
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
}
#[test]
fn full_descendant_cost_table_is_atomic_for_every_copy_operation() {
    let key = RuntimeKey::String("k".into());
    let values = RuntimeValueV2::array(vec![string("λ"), string("😀")], &mut unlimited()).unwrap();
    let empty = RuntimeMapV2::new(
        RuntimeKeyType::String,
        ty(RuntimeMapPrimitive::String, 1),
        &mut unlimited(),
    )
    .unwrap();
    cost(6, 7, |f| {
        empty.insert(&key, &values, f, &mut work()).map(|_| ())
    });
    let m = empty
        .insert(&key, &values, &mut unlimited(), &mut work())
        .unwrap();
    cost(6, 7, |f| m.snapshot(f, &mut work()).map(|_| ()));
    cost(6, 7, |f| {
        m.insert(&key, &values, f, &mut work()).map(|_| ())
    });
    cost(6, 7, |f| m.entries(f, &mut work()).map(|_| ()));
    cost(4, 6, |f| m.values(f, &mut work()).map(|_| ()));
    cost(2, 1, |f| m.keys(f, &mut work()).map(|_| ()));
    cost(3, 6, |f| m.get(&key, f, &mut work()).map(|_| ()));
    cost(4, 6, |f| m.remove(&key, f, &mut work()).map(|_| ()));
    let mut f = fuel(6, 7);
    let copy = m.snapshot(&mut f, &mut work()).unwrap();
    drop(copy);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
}
#[test]
fn failed_copy_retains_original_and_successful_outputs_are_independent() {
    let m = map(&[(1, 10), (2, 20)]);
    let mut f = fuel(0, 0);
    assert!(m
        .insert(&RuntimeKey::Int(3), &int(30), &mut f, &mut work())
        .is_err());
    assert!(m.remove(&RuntimeKey::Int(1), &mut f, &mut work()).is_err());
    let mut entries = m.entries(&mut unlimited(), &mut work()).unwrap();
    entries[0].key = RuntimeKey::Int(99);
    entries[0].value = int(99);
    entries.clear();
    let mut values = m.values(&mut unlimited(), &mut work()).unwrap();
    values.clear();
    assert!(m.equal(&map(&[(1, 10), (2, 20)]), &mut work()).unwrap());
}
#[test]
fn key_domain_and_value_shape_refuse_before_materialization() {
    let m = map(&[]);
    let mut f = fuel(0, 0);
    assert_eq!(
        m.insert(
            &RuntimeKey::String("x".into()),
            &int(1),
            &mut f,
            &mut work()
        )
        .unwrap_err(),
        RuntimeMapError::Collection(RuntimeSetError::KeyType)
    );
    assert_eq!(
        m.insert(&RuntimeKey::Int(1), &string("wrong"), &mut f, &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
    let record = RuntimeValueV2::record("Forged", vec![("x", int(1))], &mut unlimited()).unwrap();
    assert_eq!(
        m.insert(&RuntimeKey::Int(1), &record, &mut f, &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
    let arrays = RuntimeMapV2::new(
        RuntimeKeyType::Int,
        ty(RuntimeMapPrimitive::Int, 1),
        &mut unlimited(),
    )
    .unwrap();
    let mixed = RuntimeValueV2::array(vec![int(1), string("x")], &mut unlimited()).unwrap();
    assert_eq!(
        arrays
            .insert(&RuntimeKey::Int(1), &mixed, &mut f, &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
    assert_eq!(
        arrays
            .insert(&RuntimeKey::Int(1), &int(1), &mut f, &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
    assert_eq!(
        m.equal(&arrays, &mut work()),
        Err(RuntimeMapError::ValueType)
    );
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
}
#[test]
fn unicode_keys_and_values_compare_exactly_without_normalization() {
    let mut m = RuntimeMapV2::new(
        RuntimeKeyType::String,
        ty(RuntimeMapPrimitive::String, 0),
        &mut unlimited(),
    )
    .unwrap();
    for k in ["é", "e\u{301}", ""] {
        m = m
            .insert(
                &RuntimeKey::String(k.into()),
                &string(k),
                &mut unlimited(),
                &mut work(),
            )
            .unwrap();
    }
    assert_eq!(m.len(), 3);
    assert_eq!(
        m.get(
            &RuntimeKey::String("é".into()),
            &mut unlimited(),
            &mut work()
        )
        .unwrap()
        .unwrap()
        .as_str(),
        Some("é")
    );
}
#[test]
fn nested_array_and_empty_array_type_witnesses_are_preserved() {
    let empty = RuntimeValueV2::array(vec![], &mut unlimited()).unwrap();
    let inner =
        RuntimeValueV2::array(vec![int(i64::MIN), int(i64::MAX)], &mut unlimited()).unwrap();
    let outer = RuntimeValueV2::array(vec![inner, empty], &mut unlimited()).unwrap();
    let m = RuntimeMapV2::new(
        RuntimeKeyType::Bool,
        ty(RuntimeMapPrimitive::Int, 2),
        &mut unlimited(),
    )
    .unwrap();
    let m = m
        .insert(
            &RuntimeKey::Bool(true),
            &outer,
            &mut unlimited(),
            &mut work(),
        )
        .unwrap();
    assert!(m
        .get(&RuntimeKey::Bool(true), &mut unlimited(), &mut work())
        .unwrap()
        .unwrap()
        .structurally_equal(&outer));
    let empty = RuntimeValueV2::array(vec![], &mut unlimited()).unwrap();
    let m = RuntimeMapV2::new(
        RuntimeKeyType::Int,
        ty(RuntimeMapPrimitive::Int, 1),
        &mut unlimited(),
    )
    .unwrap()
    .insert(&RuntimeKey::Int(1), &empty, &mut unlimited(), &mut work())
    .unwrap();
    let other = RuntimeMapV2::new(
        RuntimeKeyType::Int,
        ty(RuntimeMapPrimitive::String, 1),
        &mut unlimited(),
    )
    .unwrap()
    .insert(&RuntimeKey::Int(1), &empty, &mut unlimited(), &mut work())
    .unwrap();
    assert_eq!(
        m.equal(&other, &mut work()),
        Err(RuntimeMapError::ValueType)
    );
}
#[test]
fn primitive_value_domains_do_not_implicitly_widen() {
    for (kind, v) in [
        (
            RuntimeMapPrimitive::Unit,
            RuntimeValueV2::unit(&mut unlimited()).unwrap(),
        ),
        (
            RuntimeMapPrimitive::Bool,
            RuntimeValueV2::boolean(true, &mut unlimited()).unwrap(),
        ),
        (
            RuntimeMapPrimitive::Float,
            RuntimeValueV2::float(1.5, &mut unlimited()).unwrap(),
        ),
    ] {
        let m = RuntimeMapV2::new(RuntimeKeyType::Int, ty(kind, 0), &mut unlimited()).unwrap();
        let m = m
            .insert(&RuntimeKey::Int(1), &v, &mut unlimited(), &mut work())
            .unwrap();
        assert!(m
            .get(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
            .unwrap()
            .unwrap()
            .structurally_equal(&v));
    }
    let m = RuntimeMapV2::new(
        RuntimeKeyType::Int,
        ty(RuntimeMapPrimitive::Float, 0),
        &mut unlimited(),
    )
    .unwrap();
    assert_eq!(
        m.insert(&RuntimeKey::Int(1), &int(1), &mut unlimited(), &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
}
#[test]
fn exact_helper_work_and_refusal_do_not_commit_result_fuel() {
    let m = map(&[(1, 10)]);
    let mut w = RuntimeSetWork::new(1);
    assert!(m.contains(&RuntimeKey::Int(1), &mut w).unwrap());
    assert_eq!(w.remaining(), 0);
    assert_eq!(
        m.contains(&RuntimeKey::Int(1), &mut w),
        Err(RuntimeMapError::Collection(RuntimeSetError::WorkLimit))
    );
    // One entry visit, then 4 result nodes: map, entry, key and value.
    let mut f = fuel(4, 0);
    let mut w = RuntimeSetWork::new(4);
    assert_eq!(
        m.snapshot(&mut f, &mut w).unwrap_err(),
        RuntimeMapError::Collection(RuntimeSetError::WorkLimit)
    );
    assert_eq!(f.remaining(), fuel(4, 0).remaining());
    assert_eq!(w.remaining(), 3);
    let mut w = RuntimeSetWork::new(5);
    m.snapshot(&mut f, &mut w).unwrap();
    assert_eq!(w.remaining(), 0);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    let m = map(&[]);
    let mut f = fuel(4, 0);
    let mut w = RuntimeSetWork::new(0);
    assert_eq!(
        m.insert(&RuntimeKey::Int(1), &int(1), &mut f, &mut w)
            .unwrap_err(),
        RuntimeMapError::Collection(RuntimeSetError::WorkLimit)
    );
    assert_eq!(f.remaining(), fuel(4, 0).remaining());
}
#[test]
fn exhaustive_short_updates_match_ordered_association_oracle() {
    // 8^4 histories: insert each of two keys with each of three values, or remove.
    for trace in 0..8usize.pow(4) {
        let mut code = trace;
        let mut actual = map(&[]);
        let mut expected: Vec<(i64, i64)> = Vec::new();
        for _ in 0..4 {
            let op = code % 8;
            code /= 8;
            let key = (op % 2) as i64;
            if op < 6 {
                let value = (op / 2) as i64;
                if let Some(pair) = expected.iter_mut().find(|p| p.0 == key) {
                    pair.1 = value
                } else {
                    expected.push((key, value))
                }
                actual = actual
                    .insert(
                        &RuntimeKey::Int(key),
                        &int(value),
                        &mut unlimited(),
                        &mut work(),
                    )
                    .unwrap();
            } else {
                let pos = expected.iter().position(|p| p.0 == key);
                let removed = actual
                    .remove(&RuntimeKey::Int(key), &mut unlimited(), &mut work())
                    .unwrap();
                assert_eq!(removed.is_some(), pos.is_some());
                if let Some(i) = pos {
                    let (_, value) = expected.remove(i);
                    let removed = removed.unwrap();
                    assert_eq!(removed.value.as_int(), Some(value));
                    actual = removed.map;
                }
            }
            let entries = actual.entries(&mut unlimited(), &mut work()).unwrap();
            assert_eq!(entries.len(), expected.len());
            for (e, (k, v)) in entries.iter().zip(&expected) {
                assert_eq!(e.key, RuntimeKey::Int(*k));
                assert_eq!(e.value.as_int(), Some(*v));
            }
            let reverse = map(&expected.iter().copied().rev().collect::<Vec<_>>());
            assert!(
                actual.equal(&reverse, &mut work()).unwrap(),
                "trace {trace}"
            );
        }
    }
}

#[test]
fn deepest_admitted_array_shape_copies_and_one_extra_level_refuses() {
    let mut value = int(1);
    for _ in 0..128 {
        value = RuntimeValueV2::array(vec![value], &mut unlimited()).unwrap();
    }
    let empty = RuntimeMapV2::new(
        RuntimeKeyType::Bool,
        ty(RuntimeMapPrimitive::Int, 128),
        &mut unlimited(),
    )
    .unwrap();
    cost(132, 0, |f| {
        empty
            .insert(&RuntimeKey::Bool(true), &value, f, &mut work())
            .map(|_| ())
    });
    let m = empty
        .insert(
            &RuntimeKey::Bool(true),
            &value,
            &mut unlimited(),
            &mut work(),
        )
        .unwrap();
    let copy = m.snapshot(&mut unlimited(), &mut work()).unwrap();
    assert!(copy.equal(&m, &mut work()).unwrap());
    let too_deep = RuntimeValueV2::array(vec![value], &mut unlimited()).unwrap();
    let mut f = fuel(0, 0);
    assert_eq!(
        empty
            .insert(&RuntimeKey::Bool(true), &too_deep, &mut f, &mut work())
            .unwrap_err(),
        RuntimeMapError::ValueType
    );
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
}
