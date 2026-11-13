//! Read-only logical measurements, independent of allocator/layout and limit policy.
use sovra::compiler::interpreter::{StructValue, Value, ValueFootprint};
#[test]
fn primitive_empty_and_unicode_sizes_keep_distinct_units() {
    for (value, expected) in [
        (
            Value::Unit,
            ValueFootprint {
                value_nodes: 1,
                owned_text_bytes: 0,
                display_bytes: 0,
            },
        ),
        (
            Value::String("λ😀".into()),
            ValueFootprint {
                value_nodes: 1,
                owned_text_bytes: 6,
                display_bytes: 6,
            },
        ),
        (
            Value::Array(vec![]),
            ValueFootprint {
                value_nodes: 1,
                owned_text_bytes: 0,
                display_bytes: 2,
            },
        ),
        (
            Value::Struct(Box::new(StructValue {
                type_name: "Empty".into(),
                fields: vec![],
            })),
            ValueFootprint {
                value_nodes: 1,
                owned_text_bytes: 5,
                display_bytes: 10,
            },
        ),
    ] {
        assert_eq!(value.logical_footprint(), Some(expected));
        assert_eq!(value.logical_footprint(), Some(expected));
    }
}
fn packet() -> Value {
    Value::Struct(Box::new(StructValue {
        type_name: "Packet".into(),
        fields: vec![
            (
                "items".into(),
                Value::Array(vec![Value::String("λ".into()), Value::String("😀".into())]),
            ),
            ("ok".into(), Value::Bool(true)),
        ],
    }))
}
#[test]
fn record_labels_payloads_and_display_sizes_match_actual_engines() {
    use sovra::compiler::{backend, compile_program, interpreter, parser::Parser};
    let value = packet();
    let expected = ValueFootprint {
        value_nodes: 5,
        owned_text_bytes: 19,
        display_bytes: 38,
    };
    assert_eq!(value.logical_footprint(), Some(expected));
    assert_eq!(value.clone().logical_footprint(), Some(expected));
    let unicode = Value::Struct(Box::new(StructValue {
        type_name: "Δ".into(),
        fields: vec![("😀".into(), Value::String("λ".into()))],
    }));
    assert_eq!(
        unicode.logical_footprint(),
        Some(ValueFootprint {
            value_nodes: 2,
            owned_text_bytes: 8,
            display_bytes: 15
        })
    );
    // Raw Rust/IR metadata can carry Unicode labels independently of source grammar.
    use sovra::compiler::ir::{Instruction, IrFunction, IrProgram, Literal};
    let raw = IrProgram {
        functions: vec![IrFunction {
            name: "main".into(),
            parameters: vec![],
            instructions: vec![
                Instruction::LoadLiteral(Literal::String("λ".into())),
                Instruction::MakeStruct {
                    type_name: "Δ".into(),
                    fields: vec!["😀".into()],
                },
                Instruction::Call {
                    name: "std::to_string".into(),
                    arguments: 1,
                },
                Instruction::Call {
                    name: "std::len".into(),
                    arguments: 1,
                },
                Instruction::Call {
                    name: "print".into(),
                    arguments: 1,
                },
            ],
        }],
    };
    assert_eq!(interpreter::run(&raw).unwrap(), ["15"]);
    let raw_node = std::process::Command::new("node")
        .args(["--eval", &backend::render_javascript(&raw)])
        .output()
        .unwrap();
    assert!(
        raw_node.status.success(),
        "{}",
        String::from_utf8_lossy(&raw_node.stderr)
    );
    assert_eq!(String::from_utf8(raw_node.stdout).unwrap().trim(), "15");
    let parsed=Parser::new().parse_source("struct Packet{items:[String],ok:Bool}fn main(){print(std::len(std::to_string(Packet{items:[\"λ\",\"😀\"],ok:true})));}").unwrap();
    let (ir, _) = compile_program(&parsed).unwrap();
    assert_eq!(
        interpreter::run(&ir).unwrap(),
        [expected.display_bytes.to_string()]
    );
    let node = std::process::Command::new("node")
        .args(["--eval", &backend::render_javascript(&ir)])
        .output()
        .unwrap();
    assert!(
        node.status.success(),
        "{}",
        String::from_utf8_lossy(&node.stderr)
    );
    assert_eq!(
        String::from_utf8(node.stdout).unwrap().trim(),
        expected.display_bytes.to_string()
    );
}
#[test]
fn independent_aggregate_overflow_returns_no_partial_measurement() {
    let measured = packet().logical_footprint().unwrap();
    let sum = measured.checked_add(measured).unwrap();
    assert_eq!(
        sum,
        ValueFootprint {
            value_nodes: 10,
            owned_text_bytes: 38,
            display_bytes: 76
        }
    );
    for maximum in [
        ValueFootprint {
            value_nodes: u64::MAX,
            ..ValueFootprint::default()
        },
        ValueFootprint {
            owned_text_bytes: u64::MAX,
            ..ValueFootprint::default()
        },
        ValueFootprint {
            display_bytes: u64::MAX,
            ..ValueFootprint::default()
        },
    ] {
        assert_eq!(
            maximum.checked_add(ValueFootprint::default()),
            Some(maximum)
        );
        assert!(maximum.checked_add(measured).is_none());
    }
}
#[test]
fn wide_values_are_measured_without_building_the_display_string() {
    let value = Value::Array((0..50000).map(|_| Value::Unit).collect());
    assert_eq!(
        value.logical_footprint(),
        Some(ValueFootprint {
            value_nodes: 50001,
            owned_text_bytes: 0,
            display_bytes: 100000
        })
    );
    value.drop_iterative();
}
#[test]
fn footprint_child() {
    let Ok(kind) = std::env::var("SOVRA_FOOTPRINT_CHILD") else {
        return;
    };
    let depth = 16000;
    let mut value = Value::Int(7);
    for index in 0..depth {
        value = if kind == "records" && index % 2 == 0 {
            Value::Struct(Box::new(StructValue {
                type_name: "Node".into(),
                fields: vec![("next".into(), value)],
            }))
        } else {
            Value::Array(vec![value])
        };
    }
    let records = if kind == "records" { depth / 2 } else { 0 };
    let expected = ValueFootprint {
        value_nodes: depth + 1,
        owned_text_bytes: records * 8,
        display_bytes: 1 + records * 15 + (depth - records) * 2,
    };
    assert_eq!(value.logical_footprint(), Some(expected));
    let copy = value.clone();
    assert_eq!(copy.logical_footprint(), Some(expected));
    copy.drop_iterative();
    value.drop_iterative();
    println!("footprint-success");
}
#[test]
fn deep_measurements_do_not_use_recursive_host_stack_traversal() {
    for kind in ["arrays", "records"] {
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "footprint_child", "--nocapture"])
            .env("SOVRA_FOOTPRINT_CHILD", kind)
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{kind}: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        assert!(String::from_utf8_lossy(&child.stdout).contains("footprint-success"));
    }
}
