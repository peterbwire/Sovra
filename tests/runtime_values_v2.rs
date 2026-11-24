//! Versioned enum values, snapshot ownership and complete enum fuel events.
use sovra::compiler::{
    execution::{MaterializationLimits as Limits, MATERIALIZATION_MODEL_VERSION},
    interpreter::{StructValue, Value},
    parser::Parser,
    v2::{
        check_enum_types, CheckedEnumTypes, EnumSource, EnumTypeLimits, RuntimeValueError as Error,
        RuntimeValueFuel as Fuel, RuntimeValueV2 as V, VALUE_MODEL_V2,
    },
};
fn fuel(nodes: u64, text: u64) -> Fuel {
    Fuel::new(Limits {
        max_value_nodes: Some(nodes),
        max_text_bytes: Some(text),
    })
}
fn unlimited() -> Fuel {
    Fuel::new(Limits::UNLIMITED)
}
fn env(context: &str, declaration: &str) -> CheckedEnumTypes {
    check_enum_types(
        "o",
        "c",
        &Parser::new().parse_source(context).unwrap(),
        &[EnumSource {
            label: "enum",
            source: declaration,
        }],
        EnumTypeLimits::DEFAULT,
    )
    .unwrap()
}
#[test]
fn enum_wrapper_transfers_payload_and_charges_owned_unicode_labels_once() {
    let env = env("", "enum Choice{Text(String)}");
    let id = env.find("Choice").unwrap();
    let mut budget = fuel(2, 16);
    let payload = V::string("λ😀", &mut budget).unwrap();
    let value = V::enum_value(&env, id, 0, Some(payload), &mut budget).unwrap();
    assert_eq!(
        budget.remaining(),
        Limits {
            max_value_nodes: Some(0),
            max_text_bytes: Some(0)
        }
    );
    let size = value.logical_footprint().unwrap();
    assert_eq!(size.value_nodes, 2);
    assert_eq!(size.owned_text_bytes, 16);
    assert_eq!(size.display_bytes, 20);
    assert_eq!(
        value.render(&mut unlimited(), None).unwrap(),
        "Choice::Text(λ😀)"
    );
    assert_eq!(value.enum_tag(), Some((id, 0)));
    assert_eq!(MATERIALIZATION_MODEL_VERSION, 1);
    assert_eq!(VALUE_MODEL_V2, 2);
}
#[test]
fn nullary_and_unit_payloads_have_distinct_arity_and_extraction_cost() {
    let env = env("", "enum E{None,Unit(Unit)}");
    let id = env.find("E").unwrap();
    let mut budget = unlimited();
    let none = V::enum_value(&env, id, 0, None, &mut budget).unwrap();
    let unit = V::unit(&mut budget).unwrap();
    let with_unit = V::enum_value(&env, id, 1, Some(unit), &mut budget).unwrap();
    assert_eq!(none.logical_footprint().unwrap().value_nodes, 1);
    assert_eq!(with_unit.logical_footprint().unwrap().value_nodes, 2);
    assert!(!none.structurally_equal(&with_unit));
    let mut extraction = fuel(1, 0);
    assert!(matches!(
        none.payload_snapshot(id, 0, &mut extraction),
        Err(Error::PayloadArity)
    ));
    assert!(with_unit.payload_snapshot(id, 1, &mut extraction).is_ok());
    assert_eq!(extraction.remaining().max_value_nodes, Some(0));
    assert!(matches!(
        V::enum_value(&env, id, 1, None, &mut budget),
        Err(Error::PayloadArity)
    ));
}
#[test]
fn copies_admit_both_dimensions_atomically_before_copying_and_never_refund() {
    let env = env("", "enum E{Text(String)}");
    let id = env.find("E").unwrap();
    let payload = V::string("λ", &mut unlimited()).unwrap();
    let value = V::enum_value(&env, id, 0, Some(payload), &mut unlimited()).unwrap();
    let size = value.logical_footprint().unwrap();
    let mut budget = fuel(size.value_nodes, size.owned_text_bytes - 1);
    let before = budget.remaining();
    assert!(matches!(value.snapshot(&mut budget), Err(Error::TextLimit)));
    assert_eq!(budget.remaining(), before);
    let mut both = fuel(0, 0);
    assert!(matches!(value.snapshot(&mut both), Err(Error::NodeLimit)));
    assert_eq!(both.remaining(), fuel(0, 0).remaining());
    let mut exact = fuel(size.value_nodes, size.owned_text_bytes);
    let copy = value.snapshot(&mut exact).unwrap();
    assert!(value.structurally_equal(&copy));
    drop(copy);
    assert_eq!(exact.remaining(), fuel(0, 0).remaining());
    let mut payload = fuel(1, 2);
    assert_eq!(
        value
            .payload_snapshot(id, 0, &mut payload)
            .unwrap()
            .as_str(),
        Some("λ")
    );
    assert_eq!(payload.remaining(), fuel(0, 0).remaining());
}
#[test]
fn numeric_widening_is_free_through_arrays_but_records_require_valid_nominal_fields() {
    let env = env("struct R{x:Float}", "enum E{Rows([[Float]]),Record(R)}");
    let id = env.find("E").unwrap();
    let old = Value::Array(vec![Value::Array(vec![Value::Int(7)])]);
    let mut budget = fuel(4, 5); // 3 imported nodes + wrapper, E/Rows labels
    let array = V::from_legacy(&old, &mut budget).unwrap();
    let value = V::enum_value(&env, id, 0, Some(array), &mut budget).unwrap();
    assert_eq!(budget.remaining(), fuel(0, 0).remaining());
    let payload = value.payload_snapshot(id, 0, &mut unlimited()).unwrap();
    let row = payload.array_item_snapshot(0, &mut unlimited()).unwrap();
    assert_eq!(
        row.array_item_snapshot(0, &mut unlimited())
            .unwrap()
            .as_float(),
        Some(7.0)
    );
    let valid = V::record(
        "R",
        vec![("x", V::float(7.0, &mut unlimited()).unwrap())],
        &mut unlimited(),
    )
    .unwrap();
    let valid = V::enum_value(&env, id, 1, Some(valid), &mut unlimited()).unwrap();
    assert_eq!(
        valid
            .payload_snapshot(id, 1, &mut unlimited())
            .unwrap()
            .field_snapshot("x", &mut unlimited())
            .unwrap()
            .as_float(),
        Some(7.0)
    );
    let record = Value::Struct(Box::new(StructValue {
        type_name: "R".into(),
        fields: vec![("x".into(), Value::Int(7))],
    }));
    let record = V::from_legacy(&record, &mut unlimited()).unwrap();
    let mut denied = fuel(0, 0);
    assert!(matches!(
        V::enum_value(&env, id, 1, Some(record), &mut denied),
        Err(Error::TypeMismatch)
    ));
    assert_eq!(denied.remaining(), fuel(0, 0).remaining());
}
#[test]
fn foreign_enums_wrong_tags_and_record_disguises_fail_before_materialization() {
    let a = env("", "enum E{End,Child(E)}");
    let b = env("", "enum E{End,Child(E)}");
    let id = a.find("E").unwrap();
    let foreign = b.find("E").unwrap();
    let value = V::enum_value(&a, id, 0, None, &mut unlimited()).unwrap();
    let mut denied = fuel(0, 0);
    assert!(matches!(
        V::enum_value(&b, id, 0, None, &mut denied),
        Err(Error::UnavailableEnum)
    ));
    assert!(matches!(
        value.payload_snapshot(foreign, 0, &mut denied),
        Err(Error::WrongVariant)
    ));
    assert!(matches!(
        V::enum_value(&b, foreign, 1, Some(value), &mut denied),
        Err(Error::TypeMismatch)
    ));
    let fake = V::record("E", vec![], &mut unlimited()).unwrap();
    assert!(matches!(
        V::enum_value(&a, id, 1, Some(fake), &mut denied),
        Err(Error::TypeMismatch)
    ));
    assert_eq!(denied.remaining(), fuel(0, 0).remaining());
}
#[test]
fn legacy_import_preserves_footprints_and_record_equality_ignores_field_order() {
    let old = Value::Struct(Box::new(StructValue {
        type_name: "R".into(),
        fields: vec![
            (
                "a".into(),
                Value::Array(vec![Value::String("λ".into()), Value::Bool(true)]),
            ),
            ("b".into(), Value::Float(2.0)),
        ],
    }));
    let value = V::from_legacy(&old, &mut unlimited()).unwrap();
    assert_eq!(
        value.logical_footprint().unwrap(),
        old.logical_footprint().unwrap()
    );
    assert_eq!(
        value.render(&mut unlimited(), None).unwrap(),
        "R { a: [λ, true], b: 2 }"
    );
    let record = V::record(
        "R",
        vec![
            ("b", V::float(2.0, &mut unlimited()).unwrap()),
            ("a", value.field_snapshot("a", &mut unlimited()).unwrap()),
        ],
        &mut unlimited(),
    )
    .unwrap();
    assert!(value.structurally_equal(&record));
    assert!(V::int(2, &mut unlimited())
        .unwrap()
        .structurally_equal(&V::float(2.0, &mut unlimited()).unwrap()));
}
#[test]
fn display_cap_precedes_fuel_and_exact_string_result_cost_is_charged() {
    let value = V::string("λ😀", &mut unlimited()).unwrap();
    let mut budget = fuel(1, 6);
    assert!(matches!(
        value.render(&mut budget, Some(5)),
        Err(Error::DisplayLimit)
    ));
    assert_eq!(budget.remaining(), fuel(1, 6).remaining());
    assert_eq!(value.render(&mut budget, Some(6)).unwrap(), "λ😀");
    assert_eq!(budget.remaining(), fuel(0, 0).remaining());
    let mut zero = fuel(0, 0);
    assert!(matches!(
        value.render(&mut zero, None),
        Err(Error::NodeLimit)
    ));
    assert_eq!(zero.remaining(), fuel(0, 0).remaining());
}
#[test]
fn transferred_containers_charge_only_wrapper_labels_and_validation_has_priority() {
    let mut budget = fuel(4, 2);
    let a = V::int(1, &mut budget).unwrap();
    let b = V::int(2, &mut budget).unwrap();
    let array = V::array(vec![a, b], &mut budget).unwrap();
    let record = V::record("R", vec![("x", array)], &mut budget).unwrap();
    assert_eq!(record.logical_footprint().unwrap().value_nodes, 4);
    assert_eq!(budget.remaining(), fuel(0, 0).remaining());
    let duplicate = Value::Struct(Box::new(StructValue {
        type_name: "R".into(),
        fields: vec![("x".into(), Value::Unit), ("x".into(), Value::Unit)],
    }));
    assert!(matches!(
        V::from_legacy(&duplicate, &mut fuel(0, 0)),
        Err(Error::TypeMismatch)
    ));
}
#[test]
fn deep_enums_snapshot_equality_display_extraction_and_ordinary_drop_are_iterative() {
    let env = env("", "enum E{End,Next(E)}");
    let id = env.find("E").unwrap();
    let mut budget = unlimited();
    let mut value = V::enum_value(&env, id, 0, None, &mut budget).unwrap();
    for _ in 0..16_000 {
        value = V::enum_value(&env, id, 1, Some(value), &mut budget).unwrap();
    }
    assert_eq!(value.logical_footprint().unwrap().value_nodes, 16_001);
    let copy = value.snapshot(&mut budget).unwrap();
    assert!(value.structurally_equal(&copy));
    let payload = value.payload_snapshot(id, 1, &mut budget).unwrap();
    assert_eq!(payload.logical_footprint().unwrap().value_nodes, 16_000);
    let text = value.render(&mut budget, None).unwrap();
    assert!(text.starts_with("E::Next(E::Next("));
    assert!(text.ends_with(')'));
    drop((value, copy, payload));
    let records = env_record();
    let mut legacy = Value::Struct(Box::new(StructValue {
        type_name: "R".into(),
        fields: vec![("next".into(), Value::Array(vec![]))],
    }));
    for _ in 0..16_000 {
        legacy = Value::Struct(Box::new(StructValue {
            type_name: "R".into(),
            fields: vec![("next".into(), Value::Array(vec![legacy]))],
        }));
    }
    let imported = V::from_legacy(&legacy, &mut budget).unwrap();
    assert_eq!(
        imported.logical_footprint().unwrap(),
        legacy.logical_footprint().unwrap()
    );
    let wrapped = V::enum_value(
        &records,
        records.find("Tree").unwrap(),
        0,
        Some(imported),
        &mut budget,
    )
    .unwrap();
    assert_eq!(wrapped.logical_footprint().unwrap().value_nodes, 32_003);
    legacy.drop_iterative();
    drop(wrapped);
}
fn env_record() -> CheckedEnumTypes {
    env("struct R{next:[R]}", "enum Tree{Record(R)}")
}
