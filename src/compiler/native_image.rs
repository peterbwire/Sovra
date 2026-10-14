//! Private, versioned IR image for the standalone executable prototype.
//! This is an implementation detail, not a stable serialization format.

use super::ir::{Instruction, IrFunction, IrProgram, Literal};

const MAGIC: &[u8] = b"SVRIR\0\x01";
const MAX_IMAGE: usize = 32 * 1024 * 1024;
const MAX_ITEMS: usize = 1_000_000;

/// Encode checked IR for embedding in a native runner.
pub fn encode(program: &IrProgram) -> Result<Vec<u8>, String> {
    let mut out = MAGIC.to_vec();
    count(&mut out, program.functions.len())?;
    for function in &program.functions {
        string(&mut out, &function.name)?;
        count(&mut out, function.parameters.len())?;
        for parameter in &function.parameters {
            string(&mut out, parameter)?;
        }
        count(&mut out, function.instructions.len())?;
        for instruction in &function.instructions {
            use Instruction::*;
            match instruction {
                LoadLiteral(value) => {
                    out.push(0);
                    match value {
                        Literal::Integer(s) => {
                            out.push(0);
                            string(&mut out, s)?;
                        }
                        Literal::Float(s) => {
                            out.push(1);
                            string(&mut out, s)?;
                        }
                        Literal::Boolean(v) => {
                            out.push(2);
                            out.push(u8::from(*v));
                        }
                        Literal::String(s) => {
                            out.push(3);
                            string(&mut out, s)?;
                        }
                    }
                }
                LoadName(s) => {
                    out.push(1);
                    string(&mut out, s)?;
                }
                StoreName(s) => {
                    out.push(2);
                    string(&mut out, s)?;
                }
                StoreIndex => out.push(3),
                WidenFloat => out.push(4),
                WidenFloatArray { depth } => {
                    out.push(5);
                    count(&mut out, *depth)?;
                }
                Binary(s) => {
                    out.push(6);
                    string(&mut out, s)?;
                }
                MakeArray { length } => {
                    out.push(7);
                    count(&mut out, *length)?;
                }
                MakeStruct { type_name, fields } => {
                    out.push(8);
                    string(&mut out, type_name)?;
                    count(&mut out, fields.len())?;
                    for field in fields {
                        string(&mut out, field)?;
                    }
                }
                LoadField(s) => {
                    out.push(9);
                    string(&mut out, s)?;
                }
                Index => out.push(10),
                Jump { target } => {
                    out.push(11);
                    count(&mut out, *target)?;
                }
                JumpIfFalse { target } => {
                    out.push(12);
                    count(&mut out, *target)?;
                }
                Call { name, arguments } => {
                    out.push(13);
                    string(&mut out, name)?;
                    count(&mut out, *arguments)?;
                }
                Return => out.push(14),
                Pop => out.push(15),
            }
            if out.len() > MAX_IMAGE {
                return Err("native IR image exceeds 32 MiB".into());
            }
        }
    }
    if out.len() > MAX_IMAGE {
        return Err("native IR image exceeds 32 MiB".into());
    }
    Ok(out)
}

/// Decode a private IR image, rejecting truncation and unknown versions.
pub fn decode(bytes: &[u8]) -> Result<IrProgram, String> {
    if bytes.len() > MAX_IMAGE || !bytes.starts_with(MAGIC) {
        return Err("invalid or unsupported native IR image".into());
    }
    let mut reader = Reader {
        bytes,
        at: MAGIC.len(),
    };
    let mut functions = Vec::new();
    for _ in 0..reader.count()? {
        let name = reader.string()?;
        let mut parameters = Vec::new();
        for _ in 0..reader.count()? {
            parameters.push(reader.string()?);
        }
        let mut instructions = Vec::new();
        for _ in 0..reader.count()? {
            use Instruction::*;
            let instruction = match reader.byte()? {
                0 => LoadLiteral(match reader.byte()? {
                    0 => Literal::Integer(reader.string()?),
                    1 => Literal::Float(reader.string()?),
                    2 => match reader.byte()? {
                        0 => Literal::Boolean(false),
                        1 => Literal::Boolean(true),
                        _ => return Err("invalid native IR boolean".into()),
                    },
                    3 => Literal::String(reader.string()?),
                    _ => return Err("unknown native IR literal".into()),
                }),
                1 => LoadName(reader.string()?),
                2 => StoreName(reader.string()?),
                3 => StoreIndex,
                4 => WidenFloat,
                5 => WidenFloatArray {
                    depth: reader.count()?,
                },
                6 => Binary(reader.string()?),
                7 => MakeArray {
                    length: reader.count()?,
                },
                8 => {
                    let type_name = reader.string()?;
                    let mut fields = Vec::new();
                    for _ in 0..reader.count()? {
                        fields.push(reader.string()?);
                    }
                    MakeStruct { type_name, fields }
                }
                9 => LoadField(reader.string()?),
                10 => Index,
                11 => Jump {
                    target: reader.count()?,
                },
                12 => JumpIfFalse {
                    target: reader.count()?,
                },
                13 => Call {
                    name: reader.string()?,
                    arguments: reader.count()?,
                },
                14 => Return,
                15 => Pop,
                _ => return Err("unknown native IR instruction".into()),
            };
            instructions.push(instruction);
        }
        functions.push(IrFunction {
            name,
            parameters,
            instructions,
        });
    }
    if reader.at != bytes.len() {
        return Err("trailing native IR data".into());
    }
    let program = IrProgram { functions };
    super::ir::validate_declarations(&program)?;
    let function_arities: std::collections::HashMap<_, _> = program
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function.parameters.len()))
        .collect();
    for function in &program.functions {
        for instruction in &function.instructions {
            match instruction {
                Instruction::Call { name, arguments } => {
                    let expected = super::stdlib::lookup(name)
                        .map(|builtin| builtin.parameters.len())
                        .or_else(|| function_arities.get(name.as_str()).copied())
                        .ok_or_else(|| format!("unknown native IR call target `{name}`"))?;
                    if *arguments != expected {
                        return Err(format!(
                            "native IR call `{name}` expects {expected} arguments, found {arguments}"
                        ));
                    }
                }
                Instruction::Jump { target } | Instruction::JumpIfFalse { target }
                    if *target > function.instructions.len() =>
                {
                    return Err("native IR jump target out of bounds".into());
                }
                Instruction::WidenFloatArray { depth: 0 } => {
                    return Err("native IR array widening depth must be positive".into());
                }
                Instruction::Binary(operator)
                    if !matches!(
                        operator.as_str(),
                        "+" | "-" | "*" | "/" | "==" | "!=" | "<" | "<=" | ">" | ">="
                    ) =>
                {
                    return Err("unknown native IR binary operator".into());
                }
                Instruction::MakeStruct { fields, .. } => {
                    let mut seen = std::collections::HashSet::new();
                    if fields.iter().any(|field| !seen.insert(field)) {
                        return Err("duplicate native IR record field".into());
                    }
                }
                _ => {}
            }
        }
    }
    Ok(program)
}

fn count(out: &mut Vec<u8>, value: usize) -> Result<(), String> {
    if value > MAX_ITEMS {
        return Err("native IR count exceeds limit".into());
    }
    let value = u32::try_from(value).map_err(|_| "native IR count overflow")?;
    out.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn string(out: &mut Vec<u8>, value: &str) -> Result<(), String> {
    count(out, value.len())?;
    if out
        .len()
        .checked_add(value.len())
        .map_or(true, |length| length > MAX_IMAGE)
    {
        return Err("native IR image exceeds 32 MiB".into());
    }
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, length: usize) -> Result<&[u8], String> {
        let end = self
            .at
            .checked_add(length)
            .ok_or("native IR length overflow")?;
        let slice = self
            .bytes
            .get(self.at..end)
            .ok_or("truncated native IR image")?;
        self.at = end;
        Ok(slice)
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn count(&mut self) -> Result<usize, String> {
        let value =
            u32::from_le_bytes(self.take(4)?.try_into().map_err(|_| "invalid count")?) as usize;
        if value > MAX_ITEMS {
            return Err("native IR count exceeds limit".into());
        }
        Ok(value)
    }
    fn string(&mut self) -> Result<String, String> {
        let length = self.count()?;
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| "invalid native IR UTF-8".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_all_instruction_forms() {
        let program = IrProgram {
            functions: vec![
                IrFunction {
                    name: "main".into(),
                    parameters: vec!["x".into()],
                    instructions: vec![
                        LoadLiteral(Literal::Integer("2".into())),
                        LoadLiteral(Literal::Float("2.5".into())),
                        LoadLiteral(Literal::Boolean(true)),
                        LoadLiteral(Literal::String("é".into())),
                        LoadName("x".into()),
                        StoreName("x".into()),
                        StoreIndex,
                        WidenFloat,
                        WidenFloatArray { depth: 2 },
                        Binary("+".into()),
                        MakeArray { length: 1 },
                        MakeStruct {
                            type_name: "Point".into(),
                            fields: vec!["x".into()],
                        },
                        LoadField("x".into()),
                        Index,
                        Jump { target: 0 },
                        JumpIfFalse { target: 1 },
                        Call {
                            name: "f".into(),
                            arguments: 1,
                        },
                        Return,
                        Pop,
                    ],
                },
                IrFunction {
                    name: "f".into(),
                    parameters: vec!["value".into()],
                    instructions: vec![],
                },
            ],
        };
        assert_eq!(decode(&encode(&program).unwrap()).unwrap(), program);
    }

    use Instruction::*;

    #[test]
    fn rejects_corrupt_images() {
        let program = IrProgram { functions: vec![] };
        let image = encode(&program).unwrap();
        assert!(decode(&image[..image.len() - 1]).is_err());
        let mut trailing = image.clone();
        trailing.push(0);
        assert!(decode(&trailing).is_err());
        let mut version = image;
        version[6] = 2;
        assert!(decode(&version).is_err());
        let invalid_jump = IrProgram {
            functions: vec![IrFunction {
                name: "main".into(),
                parameters: vec![],
                instructions: vec![Jump { target: 2 }],
            }],
        };
        assert!(decode(&encode(&invalid_jump).unwrap()).is_err());
    }

    #[test]
    fn encoder_rejects_images_its_decoder_cannot_read() {
        let overlong_name = IrProgram {
            functions: vec![IrFunction {
                name: "x".repeat(MAX_ITEMS + 1),
                parameters: vec![],
                instructions: vec![],
            }],
        };
        assert!(encode(&overlong_name).is_err());

        let oversized_depth = IrProgram {
            functions: vec![IrFunction {
                name: "main".into(),
                parameters: vec![],
                instructions: vec![WidenFloatArray {
                    depth: MAX_ITEMS + 1,
                }],
            }],
        };
        assert!(encode(&oversized_depth).is_err());
    }

    #[test]
    fn decoder_rejects_invalid_instruction_payloads_before_execution() {
        for instruction in [
            Binary("unexpected".into()),
            WidenFloatArray { depth: 0 },
            MakeStruct {
                type_name: "Point".into(),
                fields: vec!["x".into(), "x".into()],
            },
        ] {
            let program = IrProgram {
                functions: vec![IrFunction {
                    name: "main".into(),
                    parameters: vec![],
                    instructions: vec![instruction],
                }],
            };
            assert!(decode(&encode(&program).unwrap()).is_err());
        }
    }

    #[test]
    fn decoder_accepts_every_runtime_binary_operator() {
        for operator in ["+", "-", "*", "/", "==", "!=", "<", "<=", ">", ">="] {
            let program = IrProgram {
                functions: vec![IrFunction {
                    name: "main".into(),
                    parameters: vec![],
                    instructions: vec![Binary(operator.into())],
                }],
            };
            assert_eq!(decode(&encode(&program).unwrap()).unwrap(), program);
        }
    }

    #[test]
    fn decoder_rejects_missing_calls_and_wrong_arity_before_execution() {
        for (name, arguments) in [("missing", 0), ("std::print", 0), ("helper", 1)] {
            let program = IrProgram {
                functions: vec![
                    IrFunction {
                        name: "main".into(),
                        parameters: vec![],
                        instructions: vec![Call {
                            name: name.into(),
                            arguments,
                        }],
                    },
                    IrFunction {
                        name: "helper".into(),
                        parameters: vec![],
                        instructions: vec![],
                    },
                ],
            };
            assert!(decode(&encode(&program).unwrap()).is_err(), "{name}");
        }
    }

    #[test]
    fn decoder_preserves_bare_print_alias() {
        let program = IrProgram {
            functions: vec![IrFunction {
                name: "main".into(),
                parameters: vec![],
                instructions: vec![Call {
                    name: "print".into(),
                    arguments: 1,
                }],
            }],
        };
        assert_eq!(decode(&encode(&program).unwrap()).unwrap(), program);
    }
}
