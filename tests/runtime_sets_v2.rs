//! Opaque set integrity, event costs, ordering and bounded helper work.
use sovra::compiler::{execution::MaterializationLimits as Limits, v2::*};
fn fuel(nodes: u64, text: u64) -> RuntimeValueFuel {
    RuntimeValueFuel::new(Limits {
        max_value_nodes: Some(nodes),
        max_text_bytes: Some(text),
    })
}
fn unlimited() -> RuntimeValueFuel {
    RuntimeValueFuel::new(Limits::UNLIMITED)
}
fn work() -> RuntimeSetWork {
    RuntimeSetWork::new(1_000_000)
}
fn set(keys: &[i64]) -> RuntimeSetV2 {
    let mut s = RuntimeSetV2::new(RuntimeKeyType::Int, &mut unlimited()).unwrap();
    for k in keys {
        s = s
            .insert(&RuntimeKey::Int(*k), &mut unlimited(), &mut work())
            .unwrap()
    }
    s
}
#[test]
fn empty_creation_and_nominal_key_domains() {
    let mut f = fuel(1, 0);
    let s = RuntimeSetV2::new(RuntimeKeyType::String, &mut f).unwrap();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
    assert_eq!(s.key_type(), RuntimeKeyType::String);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    assert!(matches!(
        RuntimeSetV2::new(RuntimeKeyType::Bool, &mut f),
        Err(RuntimeSetError::Materialization(
            RuntimeValueError::NodeLimit
        ))
    ));
    assert_eq!(
        s.contains(&RuntimeKey::Int(0), &mut work()),
        Err(RuntimeSetError::KeyType)
    );
    assert!(matches!(
        s.insert(&RuntimeKey::Bool(true), &mut unlimited(), &mut work()),
        Err(RuntimeSetError::KeyType)
    ));
    assert!(matches!(
        s.remove(&RuntimeKey::Int(0), &mut unlimited(), &mut work()),
        Err(RuntimeSetError::KeyType)
    ));
    assert_eq!(
        s.equal(&set(&[]), &mut work()),
        Err(RuntimeSetError::KeyType)
    );
}
#[test]
fn duplicate_insertion_remove_reinsert_and_membership_equality() {
    let original = set(&[1, 2]);
    let duplicate = original
        .insert(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
        .unwrap();
    assert_eq!(
        duplicate.keys(&mut unlimited(), &mut work()).unwrap(),
        vec![RuntimeKey::Int(1), RuntimeKey::Int(2)]
    );
    let removed = duplicate
        .remove(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
        .unwrap()
        .unwrap();
    let again = removed
        .insert(&RuntimeKey::Int(1), &mut unlimited(), &mut work())
        .unwrap();
    assert_eq!(
        again.keys(&mut unlimited(), &mut work()).unwrap(),
        vec![RuntimeKey::Int(2), RuntimeKey::Int(1)]
    );
    assert!(again.equal(&original, &mut work()).unwrap());
    assert!(!again.equal(&set(&[2, 3]), &mut work()).unwrap());
    assert!(!again.equal(&set(&[1]), &mut work()).unwrap());
    assert_eq!(original.len(), 2);
    assert!(original.contains(&RuntimeKey::Int(1), &mut work()).unwrap());
}
#[test]
fn missing_results_do_not_fabricate_values_or_charge_materialization() {
    let s = set(&[1]);
    let mut f = fuel(0, 0);
    assert_eq!(
        s.get(&RuntimeKey::Int(99), &mut f, &mut work()).unwrap(),
        None
    );
    assert!(s
        .remove(&RuntimeKey::Int(99), &mut f, &mut work())
        .unwrap()
        .is_none());
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
}
#[test]
fn unicode_keys_are_exact_and_returned_strings_are_independent() {
    let mut s = RuntimeSetV2::new(RuntimeKeyType::String, &mut unlimited()).unwrap();
    for k in ["é", "e\u{301}", "", "😀"] {
        s = s
            .insert(&RuntimeKey::String(k.into()), &mut unlimited(), &mut work())
            .unwrap();
    }
    assert_eq!(s.len(), 4);
    let key = RuntimeKey::String("é".into());
    let mut got = s.get(&key, &mut unlimited(), &mut work()).unwrap().unwrap();
    if let RuntimeKey::String(text) = &mut got {
        text.clear()
    }
    assert!(s.contains(&key, &mut work()).unwrap());
    let mut keys = s.keys(&mut unlimited(), &mut work()).unwrap();
    keys.clear();
    assert_eq!(s.len(), 4);
}
#[test]
fn exact_snapshot_and_keys_costs_and_atomic_two_dimension_refusal() {
    let mut s = RuntimeSetV2::new(RuntimeKeyType::String, &mut unlimited()).unwrap();
    for k in ["λ", "😀"] {
        s = s
            .insert(&RuntimeKey::String(k.into()), &mut unlimited(), &mut work())
            .unwrap();
    }
    for (nodes, text, error) in [
        (2, 6, RuntimeValueError::NodeLimit),
        (3, 5, RuntimeValueError::TextLimit),
    ] {
        let mut f = fuel(nodes, text);
        let before = f.remaining();
        assert!(
            matches!(s.snapshot(&mut f,&mut work()),Err(RuntimeSetError::Materialization(e)) if e==error)
        );
        assert_eq!(f.remaining(), before);
    }
    let mut f = fuel(3, 6);
    let copy = s.snapshot(&mut f, &mut work()).unwrap();
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    drop(copy);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    let mut f = fuel(3, 6);
    assert_eq!(s.keys(&mut f, &mut work()).unwrap().len(), 2);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
}
#[test]
fn insertion_duplicate_removal_and_get_charge_complete_results() {
    let s = set(&[1, 2]);
    let mut f = fuel(4, 0);
    let next = s.insert(&RuntimeKey::Int(3), &mut f, &mut work()).unwrap();
    assert_eq!(next.len(), 3);
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    let mut f = fuel(3, 0);
    s.insert(&RuntimeKey::Int(1), &mut f, &mut work()).unwrap();
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    let mut f = fuel(2, 0);
    s.remove(&RuntimeKey::Int(1), &mut f, &mut work())
        .unwrap()
        .unwrap();
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    let mut f = fuel(1, 0);
    assert_eq!(
        s.get(&RuntimeKey::Int(2), &mut f, &mut work()).unwrap(),
        Some(RuntimeKey::Int(2))
    );
    assert_eq!(f.remaining(), fuel(0, 0).remaining());
    let mut f = fuel(0, 0);
    assert!(s.insert(&RuntimeKey::Int(3), &mut f, &mut work()).is_err());
    assert_eq!(
        s.keys(&mut unlimited(), &mut work()).unwrap(),
        vec![RuntimeKey::Int(1), RuntimeKey::Int(2)]
    );
}
#[test]
fn exact_lookup_and_copy_work_and_no_fuel_commit_on_work_refusal() {
    let s = set(&[1, 2]);
    let mut w = RuntimeSetWork::new(2);
    assert!(s.contains(&RuntimeKey::Int(2), &mut w).unwrap());
    assert_eq!(w.remaining(), 0);
    assert_eq!(
        s.contains(&RuntimeKey::Int(2), &mut w),
        Err(RuntimeSetError::WorkLimit)
    );
    // Snapshot footprint traversal 2, then container+keys copy 3.
    let mut f = fuel(3, 0);
    let mut w = RuntimeSetWork::new(4);
    assert!(matches!(
        s.snapshot(&mut f, &mut w),
        Err(RuntimeSetError::WorkLimit)
    ));
    assert_eq!(f.remaining(), fuel(3, 0).remaining());
    assert_eq!(w.remaining(), 2);
    let mut w = RuntimeSetWork::new(5);
    s.snapshot(&mut f, &mut w).unwrap();
    assert_eq!(w.remaining(), 0);
    let mut s = RuntimeSetV2::new(RuntimeKeyType::String, &mut unlimited()).unwrap();
    s = s
        .insert(
            &RuntimeKey::String("λ".into()),
            &mut unlimited(),
            &mut work(),
        )
        .unwrap();
    let mut w = RuntimeSetWork::new(5);
    assert!(s.contains(&RuntimeKey::String("λ".into()), &mut w).unwrap());
    assert_eq!(w.remaining(), 0);
}
#[test]
fn bool_keys_and_integer_extremes() {
    let s = set(&[i64::MIN, i64::MAX, 0]);
    assert_eq!(s.len(), 3);
    let s = RuntimeSetV2::new(RuntimeKeyType::Bool, &mut unlimited()).unwrap();
    let s = s
        .insert(&RuntimeKey::Bool(true), &mut unlimited(), &mut work())
        .unwrap();
    let s = s
        .insert(&RuntimeKey::Bool(false), &mut unlimited(), &mut work())
        .unwrap();
    assert_eq!(
        s.keys(&mut unlimited(), &mut work()).unwrap(),
        vec![RuntimeKey::Bool(true), RuntimeKey::Bool(false)]
    );
}

#[test]
fn exhaustive_short_insert_remove_traces_match_ordered_reference() {
    // All 6^4 four-operation histories over three keys, including duplicates.
    for trace in 0..6usize.pow(4) {
        let mut code = trace;
        let mut expected = Vec::new();
        let mut actual = set(&[]);
        for _ in 0..4 {
            let op = code % 6;
            code /= 6;
            let key = (op % 3) as i64;
            if op < 3 {
                if !expected.contains(&key) {
                    expected.push(key);
                }
                actual = actual
                    .insert(&RuntimeKey::Int(key), &mut unlimited(), &mut work())
                    .unwrap();
            } else {
                let position = expected.iter().position(|v| *v == key);
                let removed = actual
                    .remove(&RuntimeKey::Int(key), &mut unlimited(), &mut work())
                    .unwrap();
                assert_eq!(position.is_some(), removed.is_some());
                if let Some(index) = position {
                    expected.remove(index);
                    actual = removed.unwrap();
                }
            }
            let keys = actual.keys(&mut unlimited(), &mut work()).unwrap();
            assert_eq!(
                keys,
                expected
                    .iter()
                    .map(|k| RuntimeKey::Int(*k))
                    .collect::<Vec<_>>(),
                "trace {trace}"
            );
            let reverse = set(&expected.iter().copied().rev().collect::<Vec<_>>());
            assert!(actual.equal(&reverse, &mut work()).unwrap());
        }
    }
}
