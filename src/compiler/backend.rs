//! Backends for inspecting and compiling M10 IR.

use std::fmt::Write;

use crate::compiler::ir::{Instruction, IrFunction, IrProgram, Literal, MAX_CALL_DEPTH};

/// Render IR as a stable human-readable build artifact.
pub fn render(program: &IrProgram) -> String {
    let mut output = String::new();
    for function in &program.functions {
        let _ = writeln!(output, "function {}:", function.name);
        for instruction in &function.instructions {
            let _ = writeln!(output, "  {}", instruction_text(instruction));
        }
    }
    output
}

fn instruction_text(instruction: &Instruction) -> String {
    match instruction {
        Instruction::LoadLiteral(value) => format!("load {}", literal_text(value)),
        Instruction::LoadName(name) => format!("load-name {name}"),
        Instruction::StoreName(name) => format!("store-name {name}"),
        Instruction::StoreIndex => "store-index".to_owned(),
        Instruction::WidenFloat => "widen-float".to_owned(),
        Instruction::WidenFloatArray { depth } => format!("widen-float-array {depth}"),
        Instruction::Binary(operator) => format!("binary {operator}"),
        Instruction::MakeArray { length } => format!("make-array {length}"),
        Instruction::MakeStruct { type_name, fields } => {
            format!("make-struct {type_name} {}", fields.join(" "))
        }
        Instruction::LoadField(field) => format!("load-field {field}"),
        Instruction::Index => "index".to_owned(),
        Instruction::Jump { target } => format!("jump {target}"),
        Instruction::JumpIfFalse { target } => format!("jump-if-false {target}"),
        Instruction::Call { name, arguments } => format!("call {name} {arguments}"),
        Instruction::Return => "return".to_owned(),
        Instruction::Pop => "pop".to_owned(),
    }
}

/// Render IR as portable JavaScript.
pub fn render_javascript(program: &IrProgram) -> String {
    if let Err(error) = crate::compiler::ir::validate_declarations(program) {
        return format!("\"use strict\";\nthrow new Error({});\n", js_string(&error));
    }
    let mut output = String::new();
    let _ = writeln!(output, "\"use strict\";");
    let _ = writeln!(output);
    let _ = writeln!(output, "const svrFs = require(\"node:fs\");");
    let _ = writeln!(output, "const svrPath = require(\"node:path\");");
    let _ = writeln!(output, "const svrArgv = process.argv.slice(2);");
    let _ = writeln!(output, "const svrFunctions = Object.create(null);");
    let _ = writeln!(output, "let svrCallDepth = 0;");
    let _ = writeln!(output, "let svrExitCode = 0;");
    output.push_str(include_str!("numeric_runtime.js"));
    output.push_str(
        r#"
let svrInputBuffer = Buffer.alloc(0);
let svrInputEof = false;
function svrReadLine() {
  while (true) {
    const newline = svrInputBuffer.indexOf(10);
    if (newline !== -1 || svrInputEof) {
      if (newline === -1 && svrInputBuffer.length === 0) {
        return { __svrStruct: { typeName: "std::InputLine",
          fields: { eof: true, text: "" }, fieldOrder: ["eof", "text"] } };
      }
      const length = newline === -1 ? svrInputBuffer.length : newline;
      let bytes = svrInputBuffer.subarray(0, length);
      svrInputBuffer = newline === -1 ? Buffer.alloc(0) : svrInputBuffer.subarray(newline + 1);
      if (newline !== -1 && bytes.at(-1) === 13) bytes = bytes.subarray(0, bytes.length - 1);
      if (bytes.length > 1048576) throw new Error("standard input line exceeds 1048576 bytes");
      let text;
      try { text = new TextDecoder("utf-8", { fatal: true }).decode(bytes); }
      catch { throw new Error("standard input is not valid UTF-8"); }
      return { __svrStruct: { typeName: "std::InputLine",
        fields: { eof: false, text }, fieldOrder: ["eof", "text"] } };
    }
    if (svrInputBuffer.length > 1048577) throw new Error("standard input line exceeds 1048576 bytes");
    const chunk = Buffer.allocUnsafe(4096);
    let count;
    try { count = svrFs.readSync(0, chunk, 0, chunk.length, null); }
    catch (error) { throw new Error("standard input read failed: " + (error.code ?? error.message)); }
    if (count === 0) svrInputEof = true;
    else svrInputBuffer = Buffer.concat([svrInputBuffer, chunk.subarray(0, count)]);
  }
}
const svrMaxTextBytes = 16 * 1024 * 1024;
let svrTempSequence = 0;
function svrFileError(error) {
  if (error && error.code === "ENOENT") return "not_found";
  if (error && (error.code === "EACCES" || error.code === "EPERM")) return "permission_denied";
  if (error && (error.code === "EINVAL" || error.code === "ENAMETOOLONG" ||
      error.code === "ERR_INVALID_ARG_VALUE")) return "invalid_path";
  return "io";
}
function svrResult(typeName, fields) {
  return { __svrStruct: { typeName, fields, fieldOrder: Object.keys(fields) } };
}
function svrReadText(path) {
  const fail = error => svrResult("std::TextRead", { ok: false, text: "", error });
  if (typeof path !== "string") throw new Error("std::read_text expects a String path");
  if (!path || path.includes("\0")) return fail("invalid_path");
  let fd;
  try {
    fd = svrFs.openSync(path, "r");
    const chunks = [];
    let length = 0;
    while (true) {
      const chunk = Buffer.allocUnsafe(8192);
      const count = svrFs.readSync(fd, chunk, 0, chunk.length, null);
      if (count === 0) break;
      length += count;
      if (length > svrMaxTextBytes) return fail("too_large");
      chunks.push(chunk.subarray(0, count));
    }
    let text;
    try { text = new TextDecoder("utf-8", { fatal: true }).decode(Buffer.concat(chunks)); }
    catch { return fail("invalid_utf8"); }
    return svrResult("std::TextRead", { ok: true, text, error: "" });
  } catch (error) { return fail(svrFileError(error)); }
  finally { if (fd !== undefined) svrFs.closeSync(fd); }
}
function svrWriteText(path, text) {
  const fail = error => svrResult("std::TextWrite", { ok: false, error });
  if (typeof path !== "string" || typeof text !== "string")
    throw new Error("std::write_text expects String path and text");
  if (!path || path.includes("\0") || !svrPath.basename(path)) return fail("invalid_path");
  const bytes = Buffer.from(text, "utf-8");
  if (bytes.length > svrMaxTextBytes) return fail("too_large");
  let fd;
  let temporary;
  let created = false;
  try {
    for (let attempt = 0; attempt < 16; attempt++) {
      temporary = svrPath.join(svrPath.dirname(path),
        svrPath.basename(path) + ".svr-tmp-" + process.pid + "-" + (svrTempSequence++));
      try { fd = svrFs.openSync(temporary, "wx", 0o600); created = true; break; }
      catch (error) { if (error.code !== "EEXIST") throw error; }
    }
    if (!created) return fail("io");
    let written = 0;
    while (written < bytes.length) {
      const count = svrFs.writeSync(fd, bytes, written, bytes.length - written);
      if (count <= 0) throw new Error("short write");
      written += count;
    }
    svrFs.fsyncSync(fd);
    svrFs.closeSync(fd);
    fd = undefined;
    svrFs.renameSync(temporary, path);
    return svrResult("std::TextWrite", { ok: true, error: "" });
  } catch (error) {
    if (fd !== undefined) { try { svrFs.closeSync(fd); } catch {} }
    if (created) { try { svrFs.unlinkSync(temporary); } catch {} }
    return fail(svrFileError(error));
  }
}
function svrWriteLine(value) {
  const bytes = Buffer.from(svrDisplay(value) + "\n", "utf-8");
  let written = 0;
  while (written < bytes.length) {
    let count;
    try { count = svrFs.writeSync(1, bytes, written, bytes.length - written); }
    catch (error) { throw new Error("standard output write failed: " + (error.code ?? error.message)); }
    if (count <= 0) throw new Error("standard output write failed");
    written += count;
  }
}
function svrDisplay(value) {
  if (value === undefined) return "";
  if (typeof value === "number") return svrFormatFloat(value);
  if (Array.isArray(value)) return "[" + value.map(svrDisplay).join(", ") + "]";
  if (value !== null && typeof value === "object" &&
      Object.prototype.hasOwnProperty.call(value, "__svrStruct")) {
    const record = value.__svrStruct;
    return record.typeName + " { " +
      record.fieldOrder.map(name => name + ": " + svrDisplay(record.fields[name])).join(", ") +
      " }";
  }
  return String(value);
}
function svrFormatFloat(value) {
  if (Number.isNaN(value)) return "NaN";
  if (value === Infinity) return "inf";
  if (value === -Infinity) return "-inf";
  if (Object.is(value, -0)) return "-0";
  const text = String(value);
  const marker = text.indexOf("e");
  if (marker === -1) return text;
  const negative = text.startsWith("-");
  const sign = negative ? "-" : "";
  const mantissa = text.slice(negative ? 1 : 0, marker);
  const exponent = Number(text.slice(marker + 1));
  const dot = mantissa.indexOf(".");
  const point = (dot === -1 ? mantissa.length : dot) + exponent;
  const digits = mantissa.replace(".", "");
  if (point <= 0) return sign + "0." + "0".repeat(-point) + digits;
  if (point >= digits.length) return sign + digits + "0".repeat(point - digits.length);
  return sign + digits.slice(0, point) + "." + digits.slice(point);
}
function svrCopy(value) {
  function shell(source) {
    if (Array.isArray(source)) return [];
    if (source !== null && typeof source === "object" &&
        Object.prototype.hasOwnProperty.call(source, "__svrStruct")) {
      const record = source.__svrStruct;
      return { __svrStruct: { typeName: record.typeName,
        fields: Object.create(null), fieldOrder: [...record.fieldOrder] } };
    }
    return source;
  }
  const result = shell(value);
  if (result === value) return value;
  const pending = [[value, result]];
  while (pending.length) {
    const [source, target] = pending.pop();
    if (Array.isArray(source)) {
      for (const item of source) {
        const copy = shell(item);
        target.push(copy);
        if (copy !== item) pending.push([item, copy]);
      }
    } else {
      const sourceFields = source.__svrStruct.fields;
      const targetFields = target.__svrStruct.fields;
      for (const name of source.__svrStruct.fieldOrder) {
        const item = sourceFields[name];
        const copy = shell(item);
        targetFields[name] = copy;
        if (copy !== item) pending.push([item, copy]);
      }
    }
  }
  return result;
}
"#,
    );
    let _ = writeln!(output);
    for (index, function) in program.functions.iter().enumerate() {
        render_js_function(&mut output, index, function);
    }
    for (index, function) in program.functions.iter().enumerate() {
        let _ = writeln!(
            output,
            "svrFunctions[{}] = {};",
            js_string(&function.name),
            js_function_name(index)
        );
    }
    let _ = writeln!(output);
    let _ = writeln!(
        output,
        "if (!svrFunctions.main) throw new Error(\"entry function `main` was not found\");"
    );
    let _ = writeln!(output, "svrFunctions.main();");
    let _ = writeln!(output, "process.exitCode = svrExitCode;");
    output
}

fn render_js_function(output: &mut String, index: usize, function: &IrFunction) {
    let _ = write!(output, "function {}(", js_function_name(index));
    for parameter_index in 0..function.parameters.len() {
        if parameter_index > 0 {
            let _ = write!(output, ", ");
        }
        let _ = write!(output, "svr_arg_{parameter_index}");
    }
    let _ = writeln!(output, ") {{");
    let _ = writeln!(
        output,
        "  if (svrCallDepth >= {MAX_CALL_DEPTH}) throw new Error({});",
        js_string(&format!(
            "maximum call depth of {MAX_CALL_DEPTH} exceeded in `{}`",
            function.name
        ))
    );
    let _ = writeln!(
        output,
        "  if (arguments.length !== {}) throw new Error({} + arguments.length);",
        function.parameters.len(),
        js_string(&format!(
            "function `{}` expects {} argument(s), found ",
            function.name,
            function.parameters.len()
        ))
    );
    let _ = writeln!(output, "  svrCallDepth++;");
    let _ = writeln!(output, "  try {{");
    let _ = writeln!(output, "  const stack = [];");
    let _ = writeln!(output, "  const names = Object.create(null);");
    for (parameter_index, parameter) in function.parameters.iter().enumerate() {
        let _ = writeln!(
            output,
            "  names[{}] = svrCopy(svr_arg_{parameter_index});",
            js_string(parameter),
        );
    }
    let _ = writeln!(output, "  let pc = 0;");
    let _ = writeln!(output, "  while (pc < {}) {{", function.instructions.len());
    let _ = writeln!(output, "    switch (pc) {{");
    for (pc, instruction) in function.instructions.iter().enumerate() {
        render_js_instruction(output, pc, instruction, function.instructions.len());
    }
    let _ = writeln!(
        output,
        "      default: throw new Error(\"jump target out of bounds\");"
    );
    let _ = writeln!(output, "    }}");
    let _ = writeln!(output, "  }}");
    let _ = writeln!(output, "  return stack.length ? stack.pop() : undefined;");
    let _ = writeln!(output, "  }} finally {{ svrCallDepth--; }}");
    let _ = writeln!(output, "}}");
    let _ = writeln!(output);
}

fn render_js_instruction(
    output: &mut String,
    pc: usize,
    instruction: &Instruction,
    instruction_count: usize,
) {
    let _ = writeln!(output, "      case {pc}: {{");
    match instruction {
        Instruction::LoadLiteral(value) => {
            let _ = writeln!(output, "        stack.push({});", js_literal(value));
        }
        Instruction::LoadName(name) => {
            let _ = writeln!(
                output,
                "        if (!Object.prototype.hasOwnProperty.call(names, {})) throw new Error({});",
                js_string(name),
                js_string(&format!("runtime name `{name}` was not found"))
            );
            let _ = writeln!(
                output,
                "        stack.push(svrCopy(names[{}]));",
                js_string(name)
            );
        }
        Instruction::StoreName(name) => {
            render_stack_guard(output, 1, "store");
            let _ = writeln!(output, "        names[{}] = stack.pop();", js_string(name));
        }
        Instruction::StoreIndex => {
            render_stack_guard(output, 3, "store-index");
            let _ = writeln!(output, "        {{");
            let _ = writeln!(output, "          const value = stack.pop();");
            let _ = writeln!(output, "          const index = stack.pop();");
            let _ = writeln!(output, "          const target = stack.pop();");
            let _ = writeln!(output, "          if (!Array.isArray(target) || typeof index !== \"bigint\") throw new Error(\"index assignment requires an Array and Int index\");");
            let _ = writeln!(output, "          if (index < 0n || index >= BigInt(target.length)) throw new Error(\"array index out of bounds\");");
            let _ = writeln!(
                output,
                "          target[Number(index)] = value; stack.push(target);"
            );
            let _ = writeln!(output, "        }}");
        }
        Instruction::WidenFloat => {
            render_stack_guard(output, 1, "widening");
            let _ = writeln!(output, "        stack.push(svrWidenFloat(stack.pop()));");
        }
        Instruction::WidenFloatArray { depth } => {
            render_stack_guard(output, 1, "array widening");
            let _ = writeln!(
                output,
                "        stack.push(svrWidenFloatArray(stack.pop(), {depth}));"
            );
        }
        Instruction::Binary(operator) => {
            render_stack_guard(output, 2, "binary operator");
            let _ = writeln!(output, "        {{");
            let _ = writeln!(output, "          const right = stack.pop();");
            let _ = writeln!(output, "          const left = stack.pop();");
            let _ = writeln!(
                output,
                "          stack.push(svrBinary({}, left, right));",
                js_string(operator)
            );
            let _ = writeln!(output, "        }}");
        }
        Instruction::MakeArray { length } => {
            render_stack_guard(output, *length, "array literal");
            let _ = writeln!(
                output,
                "        {{
          const items = stack.splice(stack.length - {length});
          stack.push(items);
        }}"
            );
        }
        Instruction::MakeStruct { type_name, fields } => {
            render_stack_guard(output, fields.len(), "struct literal");
            let _ = writeln!(output, "        {{");
            let _ = writeln!(
                output,
                "          const values = stack.splice(stack.length - {});",
                fields.len()
            );
            let _ = writeln!(
                output,
                "          const recordFields = Object.create(null);"
            );
            let _ = writeln!(output, "          const fieldOrder = [];");
            for (index, field) in fields.iter().enumerate() {
                let _ = writeln!(
                    output,
                    "          if (Object.prototype.hasOwnProperty.call(recordFields, {})) throw new Error({});",
                    js_string(field),
                    js_string(&format!("duplicate field in struct `{type_name}`"))
                );
                let _ = writeln!(
                    output,
                    "          recordFields[{}] = values[{index}]; fieldOrder.push({});",
                    js_string(field),
                    js_string(field)
                );
            }
            let _ = writeln!(
                output,
                "          stack.push({{ __svrStruct: {{ typeName: {}, fields: recordFields, fieldOrder }} }});",
                js_string(type_name)
            );
            let _ = writeln!(output, "        }}");
        }
        Instruction::LoadField(field) => {
            render_stack_guard(output, 1, "field access");
            let _ = writeln!(output, "        {{");
            let _ = writeln!(output, "          const recordValue = stack.pop();");
            let _ = writeln!(output, "          if (recordValue === null || typeof recordValue !== \"object\" || !Object.prototype.hasOwnProperty.call(recordValue, \"__svrStruct\")) throw new Error(\"field access requires a struct value\");");
            let _ = writeln!(
                output,
                "          const recordFields = recordValue.__svrStruct.fields;"
            );
            let _ = writeln!(output, "          if (!Object.prototype.hasOwnProperty.call(recordFields, {})) throw new Error({});", js_string(field), js_string(&format!("struct value has no field `{field}`")));
            let _ = writeln!(
                output,
                "          stack.push(svrCopy(recordFields[{}]));",
                js_string(field)
            );
            let _ = writeln!(output, "        }}");
        }
        Instruction::Index => {
            render_stack_guard(output, 2, "index");
            let _ = writeln!(output, "        {{");
            let _ = writeln!(output, "          const arrayIndex = stack.pop();");
            let _ = writeln!(output, "          const arrayTarget = stack.pop();");
            let _ = writeln!(output, "          if (!Array.isArray(arrayTarget) || typeof arrayIndex !== \"bigint\") throw new Error(\"index requires an Array and Int index\");");
            let _ = writeln!(output, "          if (arrayIndex < 0n || arrayIndex >= BigInt(arrayTarget.length)) throw new Error(\"array index out of bounds\");");
            let _ = writeln!(
                output,
                "          stack.push(svrCopy(arrayTarget[Number(arrayIndex)]));"
            );
            let _ = writeln!(output, "        }}");
        }
        Instruction::Jump { target } => {
            let _ = writeln!(
                output,
                "        if ({target} > {instruction_count}) throw new Error(\"jump target out of bounds\");"
            );
            let _ = writeln!(output, "        pc = {target}; break;");
        }
        Instruction::JumpIfFalse { target } => {
            render_stack_guard(output, 1, "jump-if-false");
            let _ = writeln!(
                output,
                "        if ({target} > {instruction_count}) throw new Error(\"jump target out of bounds\");"
            );
            let _ = writeln!(output, "        {{");
            let _ = writeln!(output, "          const condition = stack.pop();");
            let _ = writeln!(output, "          if (typeof condition !== \"boolean\") throw new Error(\"if condition must be Bool\");");
            let _ = writeln!(
                output,
                "          if (condition) pc += 1; else pc = {target};"
            );
            let _ = writeln!(output, "        }}");
            let _ = writeln!(output, "        break;");
        }
        Instruction::Call { name, arguments } => render_js_call(output, name, *arguments),
        Instruction::Return => {
            let _ = writeln!(
                output,
                "        return stack.length ? stack.pop() : undefined;"
            );
        }
        Instruction::Pop => {
            render_stack_guard(output, 1, "pop");
            let _ = writeln!(output, "        stack.pop();");
        }
    }
    if !matches!(
        instruction,
        Instruction::Jump { .. } | Instruction::JumpIfFalse { .. } | Instruction::Return
    ) {
        let _ = writeln!(output, "        pc += 1;");
        let _ = writeln!(output, "        break;");
    }
    let _ = writeln!(output, "      }}");
}

fn render_js_call(output: &mut String, name: &str, arguments: usize) {
    render_stack_guard(output, arguments, "call");
    if let Some(function) = crate::compiler::stdlib::lookup(name) {
        if arguments != function.parameters.len() {
            let _ = writeln!(
                output,
                "  throw new Error({});",
                js_string(&format!(
                    "{name} expects exactly {} argument(s)",
                    function.parameters.len()
                ))
            );
            return;
        }
    }
    let _ = writeln!(output, "  {{");
    let _ = writeln!(
        output,
        "    const args = stack.splice(stack.length - {arguments});"
    );
    match name {
        "print" | "std::print" | "std::println" => {
            let _ = writeln!(output, "    svrWriteLine(args[0]);");
            let _ = writeln!(output, "    stack.push(undefined);");
        }
        "std::arg_count" => {
            let _ = writeln!(output, "    stack.push(BigInt(svrArgv.length));");
        }
        "std::arg" => {
            let _ = writeln!(output, "    if (typeof args[0] !== \"bigint\") throw new Error(\"std::arg expects an Int index\");");
            let _ = writeln!(output, "    if (args[0] < 0n || args[0] >= BigInt(svrArgv.length)) throw new Error(\"program argument index out of bounds\");");
            let _ = writeln!(output, "    stack.push(svrArgv[Number(args[0])]);");
        }
        "std::set_exit_code" => {
            let _ = writeln!(output, "    if (typeof args[0] !== \"bigint\") throw new Error(\"std::set_exit_code expects an Int code\");");
            let _ = writeln!(output, "    if (args[0] < 0n || args[0] > 125n) throw new Error(\"exit code must be between 0 and 125\");");
            let _ = writeln!(output, "    svrExitCode = Number(args[0]);");
            let _ = writeln!(output, "    stack.push(undefined);");
        }
        "std::read_line" => {
            let _ = writeln!(output, "    stack.push(svrReadLine());");
        }
        "std::read_text" => {
            let _ = writeln!(output, "    stack.push(svrReadText(args[0]));");
        }
        "std::write_text" => {
            let _ = writeln!(output, "    stack.push(svrWriteText(args[0], args[1]));");
        }
        "std::split_once" => {
            let _ = writeln!(output, "    if (typeof args[0] !== \"string\" || typeof args[1] !== \"string\") throw new Error(\"std::split_once expects String arguments\");");
            let _ = writeln!(output, "    const index = args[0].indexOf(args[1]);");
            let _ = writeln!(output, "    stack.push(svrResult(\"std::SplitOnce\", {{ found: index !== -1, before: index === -1 ? args[0] : args[0].slice(0, index), after: index === -1 ? \"\" : args[0].slice(index + args[1].length) }}));");
        }
        "std::lines_unique" => {
            let _ = writeln!(output, "    if (typeof args[0] !== \"string\") throw new Error(\"std::lines_unique expects a String argument\");");
            let _ = writeln!(
                output,
                "    const lines = args[0] === \"\" ? [] : args[0].split(\"\\n\");"
            );
            let _ = writeln!(output, "    if (args[0].endsWith(\"\\n\")) lines.pop();");
            let _ = writeln!(
                output,
                "    stack.push(new Set(lines).size === lines.length);"
            );
        }
        "std::parse_int" => {
            let _ = writeln!(output, "    if (typeof args[0] !== \"string\") throw new Error(\"std::parse_int expects a String argument\");");
            let _ = writeln!(
                output,
                "    const digits = args[0].startsWith(\"-\") ? args[0].length - 1 : args[0].length;"
            );
            let _ = writeln!(
                output,
                "    const valid = digits <= 19 && /^-?(?:0|[1-9][0-9]*)$/.test(args[0]);"
            );
            let _ = writeln!(output, "    const parsed = valid ? BigInt(args[0]) : 0n;");
            let _ = writeln!(output, "    const ok = valid && parsed >= -9223372036854775808n && parsed <= 9223372036854775807n;");
            let _ = writeln!(
                output,
                "    stack.push(svrResult(\"std::ParsedInt\", {{ ok, value: ok ? parsed : 0n }}));"
            );
        }
        "std::len" => {
            let _ = writeln!(output, "    if (typeof args[0] !== \"string\") throw new Error(\"std::len expects a String argument\");");
            let _ = writeln!(
                output,
                "    stack.push(BigInt(new TextEncoder().encode(args[0]).length));"
            );
        }
        "std::to_string" => {
            let _ = writeln!(output, "    stack.push(svrDisplay(args[0]));");
        }
        _ => {
            let _ = writeln!(
                output,
                "    const callee = svrFunctions[{}];",
                js_string(name)
            );
            let _ = writeln!(
                output,
                "    if (!callee) throw new Error({});",
                js_string(&format!("runtime function `{name}` was not found"))
            );
            let _ = writeln!(output, "    stack.push(callee(...args));");
        }
    }
    let _ = writeln!(output, "  }}");
}

fn render_stack_guard(output: &mut String, required: usize, operation: &str) {
    let _ = writeln!(
        output,
        "  if (stack.length < {required}) throw new Error({});",
        js_string(&format!("stack underflow on {operation}"))
    );
}

fn literal_text(value: &Literal) -> String {
    match value {
        Literal::Integer(value) | Literal::Float(value) => value.clone(),
        Literal::Boolean(value) => value.to_string(),
        Literal::String(value) => format!("{value:?}"),
    }
}

fn js_literal(value: &Literal) -> String {
    match value {
        Literal::Integer(value) => match value.parse::<i64>() {
            Ok(value) => format!("BigInt({})", js_string(&value.to_string())),
            Err(_) => js_literal_error("invalid or out-of-range integer literal"),
        },
        Literal::Float(value) => match value.parse::<f64>() {
            Ok(value) if value.is_nan() => "NaN".into(),
            Ok(value) if value == f64::INFINITY => "Infinity".into(),
            Ok(value) if value == f64::NEG_INFINITY => "-Infinity".into(),
            Ok(value) => format!("Number({})", js_string(&value.to_string())),
            Err(_) => js_literal_error("invalid float literal"),
        },
        Literal::Boolean(value) => value.to_string(),
        Literal::String(value) => js_string(value),
    }
}

// Keep invalid IR errors at execution time; unreachable literals must not fail emission.
fn js_literal_error(message: &str) -> String {
    format!("(() => {{ throw new Error({}); }})()", js_string(message))
}

fn js_function_name(index: usize) -> String {
    format!("svr_fn_{index}")
}

fn js_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            // Fixed-width escapes cannot absorb a following digit (unlike \0).
            '\u{0000}'..='\u{001f}' | '\u{007f}'..='\u{009f}' | '\u{2028}' | '\u{2029}' => {
                let _ = write!(output, "\\u{:04x}", character as u32);
            }
            _ => output.push(character),
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_exit_status_ir_reports_the_same_runtime_error() {
        assert_ir_failure_matches(vec![
            Instruction::LoadLiteral(Literal::String("bad".into())),
            Instruction::Call {
                name: "std::set_exit_code".into(),
                arguments: 1,
            },
        ]);
        for code in ["-1", "126"] {
            assert_ir_failure_matches(vec![
                Instruction::LoadLiteral(Literal::Integer(code.into())),
                Instruction::Call {
                    name: "std::set_exit_code".into(),
                    arguments: 1,
                },
            ]);
        }
    }

    #[test]
    fn module_local_records_and_aliases_retain_distinct_types_in_both_engines() {
        let source = include_str!("../../examples/scoped-types/main.svr");
        let program = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .unwrap();
        let ir = crate::compiler::ir::lower_program(&program).unwrap();
        let expected = ["1.5", "1.5", "1", "root"];
        assert_eq!(crate::compiler::interpreter::run(&ir).unwrap(), expected);
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn unusual_ir_function_names_preserve_calls_and_errors() {
        for name in ["quote\"slash\\line\nλ", "__proto__", "constructor", ""] {
            let instructions = vec![Instruction::Call {
                name: name.into(),
                arguments: 0,
            }];
            assert_ir_failure_matches(instructions.clone());
            let program = IrProgram {
                functions: vec![
                    IrFunction {
                        name: "main".into(),
                        parameters: vec![],
                        instructions,
                    },
                    IrFunction {
                        name: name.into(),
                        parameters: vec![],
                        instructions: vec![
                            Instruction::LoadLiteral(Literal::String("called".into())),
                            Instruction::Call {
                                name: "print".into(),
                                arguments: 1,
                            },
                        ],
                    },
                ],
            };
            let expected = crate::compiler::interpreter::run(&program).unwrap();
            let output = execute_javascript(&render_javascript(&program));
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .lines()
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }

    #[test]
    fn distinct_ir_parameter_names_preserve_argument_identity() {
        let parameters: Vec<String> = [
            "a-b",
            "a_b",
            "λ",
            "Ж",
            "",
            "arguments",
            "__proto__",
            "constructor",
            "quote\"\\\n",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let mut main = Vec::new();
        let mut body = Vec::new();
        for (index, parameter) in parameters.iter().enumerate() {
            main.push(Instruction::LoadLiteral(Literal::Integer(
                index.to_string(),
            )));
            body.push(Instruction::LoadName(parameter.clone()));
            body.push(Instruction::Call {
                name: "print".into(),
                arguments: 1,
            });
            body.push(Instruction::Pop);
        }
        main.push(Instruction::Call {
            name: "inspect".into(),
            arguments: parameters.len(),
        });
        let program = IrProgram {
            functions: vec![
                IrFunction {
                    name: "main".into(),
                    parameters: vec![],
                    instructions: main,
                },
                IrFunction {
                    name: "inspect".into(),
                    parameters,
                    instructions: body,
                },
            ],
        };
        let expected = crate::compiler::interpreter::run(&program).unwrap();
        let output = execute_javascript(&render_javascript(&program));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn duplicate_ir_declarations_fail_in_both_engines() {
        for name in ["main", "helper", "module::helper"] {
            let function = IrFunction {
                name: name.into(),
                parameters: vec![],
                instructions: vec![],
            };
            let mut functions = vec![function.clone(), function];
            if name != "main" {
                functions.insert(
                    0,
                    IrFunction {
                        name: "main".into(),
                        parameters: vec![],
                        instructions: vec![],
                    },
                );
            }
            let program = IrProgram { functions };
            let expected = format!("duplicate IR function `{name}`");
            assert_eq!(
                crate::compiler::interpreter::run(&program).unwrap_err(),
                expected
            );
            let output = execute_javascript(&render_javascript(&program));
            assert!(!output.status.success());
            assert!(String::from_utf8_lossy(&output.stderr).contains(&expected));
        }
        let program = IrProgram {
            functions: vec![
                IrFunction {
                    name: "main".into(),
                    parameters: vec![],
                    instructions: vec![],
                },
                IrFunction {
                    name: "unused".into(),
                    parameters: vec!["x".into(), "x".into()],
                    instructions: vec![],
                },
            ],
        };
        let expected = "duplicate IR parameter `x` in function `unused`";
        assert_eq!(
            crate::compiler::interpreter::run(&program).unwrap_err(),
            expected
        );
        let output = execute_javascript(&render_javascript(&program));
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }

    #[test]
    fn binary_runtime_type_matrix_matches_interpreter() {
        let values = [
            Some(Literal::Integer("2".into())),
            Some(Literal::Float("3.0".into())),
            Some(Literal::Boolean(true)),
            Some(Literal::String("x".into())),
            None,
            Some(Literal::Integer("0".into())),
        ];
        let mut script = include_str!("numeric_runtime.js").to_owned();
        for operator in [
            "+", "-", "*", "/", "==", "!=", "<", "<=", ">", ">=", "unknown",
        ] {
            for left in &values {
                for right in &values {
                    let mut instructions = Vec::new();
                    for value in [left, right] {
                        instructions.push(match value {
                            Some(value) => Instruction::LoadLiteral(value.clone()),
                            None => Instruction::Call {
                                name: "unit".into(),
                                arguments: 0,
                            },
                        });
                    }
                    instructions.push(Instruction::Binary(operator.into()));
                    instructions.push(Instruction::Call {
                        name: "print".into(),
                        arguments: 1,
                    });
                    let program = IrProgram {
                        functions: vec![
                            IrFunction {
                                name: "main".into(),
                                parameters: vec![],
                                instructions,
                            },
                            IrFunction {
                                name: "unit".into(),
                                parameters: vec![],
                                instructions: vec![],
                            },
                        ],
                    };
                    let (expected, failure) = match crate::compiler::interpreter::run(&program) {
                        Ok(output) => (output[0].clone(), false),
                        Err(error) => (error, true),
                    };
                    let operand = |value: &Option<Literal>| {
                        value.as_ref().map(js_literal).unwrap_or("undefined".into())
                    };
                    let _ = writeln!(script, "{{ let actual, failed = false; try {{ actual = String(svrBinary({}, {}, {})); }} catch (error) {{ failed = true; actual = error.message; }} if (failed !== {failure} || actual !== {}) throw Error({}); }}",
                        js_string(operator), operand(left), operand(right), js_string(&expected),
                        js_string(&format!("mismatch: {left:?} {operator} {right:?}")));
                }
            }
        }
        let output = execute_javascript(&script);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn malformed_numeric_ir_literals_match_interpreter() {
        for value in [
            "",
            " ",
            "1.5",
            "0x10",
            "1_000",
            "9223372036854775808",
            "-9223372036854775809",
        ] {
            assert_ir_failure_matches(vec![Instruction::LoadLiteral(Literal::Integer(
                value.into(),
            ))]);
        }
        for value in ["", " ", "0x10", "1_000", "1.2.3", "hello"] {
            assert_ir_failure_matches(vec![Instruction::LoadLiteral(Literal::Float(value.into()))]);
        }
    }

    #[test]
    fn valid_numeric_ir_literals_preserve_rust_parsing() {
        for (value, expected) in [
            ("+001", "1"),
            ("-9223372036854775808", "-9223372036854775808"),
        ] {
            let output = execute_javascript(&format!(
                "console.log(String({}));",
                js_literal(&Literal::Integer(value.into()))
            ));
            assert!(output.status.success());
            assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), expected);
        }
        for (value, predicate) in [
            ("inf", "value === Infinity"),
            ("-inf", "value === -Infinity"),
            ("NaN", "Number.isNaN(value)"),
            ("-0.0", "Object.is(value, -0)"),
            ("1e2", "value === 100"),
        ] {
            let output = execute_javascript(&format!(
                "const value = {}; if (!({predicate})) throw Error('wrong value');",
                js_literal(&Literal::Float(value.into()))
            ));
            assert!(
                output.status.success(),
                "{value}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn builtin_ir_arity_and_len_types_match_interpreter() {
        for name in [
            "print",
            "std::print",
            "std::println",
            "std::len",
            "std::to_string",
        ] {
            for count in [0, 2] {
                let mut instructions =
                    vec![Instruction::LoadLiteral(Literal::String("x".into())); count];
                instructions.push(Instruction::Call {
                    name: name.into(),
                    arguments: count,
                });
                assert_ir_failure_matches(instructions);
            }
        }
        for value in [
            Literal::Integer("1".into()),
            Literal::Float("1.0".into()),
            Literal::Boolean(true),
        ] {
            assert_ir_failure_matches(vec![
                Instruction::LoadLiteral(value),
                Instruction::Call {
                    name: "std::len".into(),
                    arguments: 1,
                },
            ]);
        }
        assert_ir_failure_matches(vec![
            Instruction::LoadLiteral(Literal::String("x".into())),
            Instruction::Call {
                name: "print".into(),
                arguments: 1,
            },
            Instruction::Call {
                name: "std::len".into(),
                arguments: 1,
            },
        ]);
    }

    #[test]
    fn text_file_builtin_ir_errors_match_interpreter_without_touching_files() {
        for (name, arguments) in [
            ("std::read_text", 0),
            ("std::read_text", 2),
            ("std::write_text", 0),
            ("std::write_text", 1),
        ] {
            let mut instructions =
                vec![Instruction::LoadLiteral(Literal::String("unused".into())); arguments];
            instructions.push(Instruction::Call {
                name: name.into(),
                arguments,
            });
            assert_ir_failure_matches(instructions);
        }
        assert_ir_failure_matches(vec![
            Instruction::LoadLiteral(Literal::Integer("1".into())),
            Instruction::Call {
                name: "std::read_text".into(),
                arguments: 1,
            },
        ]);
        for arguments in [
            [
                Literal::Integer("1".into()),
                Literal::String("content".into()),
            ],
            [
                Literal::String("unused".into()),
                Literal::Integer("1".into()),
            ],
        ] {
            assert_ir_failure_matches(vec![
                Instruction::LoadLiteral(arguments[0].clone()),
                Instruction::LoadLiteral(arguments[1].clone()),
                Instruction::Call {
                    name: "std::write_text".into(),
                    arguments: 2,
                },
            ]);
        }
    }

    #[test]
    fn javascript_array_reads_and_mutation_match_interpreter() {
        let source = "fn mutate() { let mut values = [1, 2]; values[0] = 9; values[1] = 8; print(values[0]); print(values[1]); }
            fn main() { print(mutate()); }";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .expect("valid syntax");
        let ir = crate::compiler::ir::lower_program(&program).expect("valid program");
        let expected = crate::compiler::interpreter::run(&ir).expect("interpreter execution");
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );

        assert_ir_failure_matches(vec![
            Instruction::MakeArray { length: 0 },
            Instruction::LoadLiteral(Literal::Integer("-1".into())),
            Instruction::Index,
        ]);
        assert_ir_failure_matches(vec![
            Instruction::MakeArray { length: 0 },
            Instruction::LoadLiteral(Literal::Integer("-1".into())),
            Instruction::LoadLiteral(Literal::Integer("1".into())),
            Instruction::StoreIndex,
        ]);
    }

    #[test]
    fn compound_bindings_and_nested_array_reads_are_value_snapshots() {
        let source = r#"
            fn main() {
                let original = [[1], [2]];
                let mut copy = original;
                copy[0] = [9];
                print(original[0][0]);
                print(copy[0][0]);
                let mut element = original[1];
                element[0] = 7;
                print(original[1][0]);
                print(element[0]);
            }
        "#;
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .unwrap();
        let program = crate::compiler::ir::lower_program(&parsed).unwrap();
        assert_eq!(
            crate::compiler::interpreter::run(&program).unwrap(),
            ["1", "9", "2", "7"]
        );
        let output = execute_javascript(&render_javascript(&program));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            ["1", "9", "2", "7"]
        );
    }

    #[test]
    fn compound_equality_is_recursive_and_nominal_in_both_engines() {
        let source = r#"
            struct Point { x: Int, y: Int }
            struct Other { x: Int, y: Int }
            fn main() {
                print([1, 2] == [1, 2]);
                print([[1], [2]] != [[1], [3]]);
                let point = Point { x: 1, y: 2 };
                print(point == Point { y: 2, x: 1 });
                print(point != Point { x: 1, y: 3 });
            }
        "#;
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .unwrap();
        let program = crate::compiler::ir::lower_program(&parsed).unwrap();
        let expected = ["true", "true", "true", "true"];
        assert_eq!(
            crate::compiler::interpreter::run(&program).unwrap(),
            expected
        );
        let output = execute_javascript(&render_javascript(&program));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn array_argument_and_result_boundaries_do_not_alias_the_caller() {
        let program = IrProgram {
            functions: vec![
                IrFunction {
                    name: "main".into(),
                    parameters: vec![],
                    instructions: vec![
                        Instruction::LoadLiteral(Literal::Integer("1".into())),
                        Instruction::MakeArray { length: 1 },
                        Instruction::StoreName("original".into()),
                        Instruction::LoadName("original".into()),
                        Instruction::Call {
                            name: "change".into(),
                            arguments: 1,
                        },
                        Instruction::Call {
                            name: "print".into(),
                            arguments: 1,
                        },
                        Instruction::Pop,
                        Instruction::LoadName("original".into()),
                        Instruction::LoadLiteral(Literal::Integer("0".into())),
                        Instruction::Index,
                        Instruction::Call {
                            name: "print".into(),
                            arguments: 1,
                        },
                    ],
                },
                IrFunction {
                    name: "change".into(),
                    parameters: vec!["items".into()],
                    instructions: vec![
                        Instruction::LoadName("items".into()),
                        Instruction::LoadLiteral(Literal::Integer("0".into())),
                        Instruction::LoadLiteral(Literal::Integer("9".into())),
                        Instruction::StoreIndex,
                        Instruction::StoreName("items".into()),
                        Instruction::LoadName("items".into()),
                        Instruction::LoadLiteral(Literal::Integer("0".into())),
                        Instruction::Index,
                        Instruction::Return,
                    ],
                },
            ],
        };
        let expected = ["9", "1"];
        assert_eq!(
            crate::compiler::interpreter::run(&program).unwrap(),
            expected
        );
        let output = execute_javascript(&render_javascript(&program));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn javascript_record_construction_access_and_display_match_interpreter() {
        let source = "
            struct Point { x: Int, y: Int }
            struct Marker { label: String, point: Point }
            fn origin() -> Marker {
                return Marker { label: \"home\", point: Point { x: 4, y: 7 } }
            }
            fn main() {
                let marker = origin();
                print(marker.point.x);
                print(std::to_string(marker));
            }
        ";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .expect("valid syntax");
        let ir = crate::compiler::ir::lower_program(&program).expect("valid record types");
        let expected = crate::compiler::interpreter::run(&ir).expect("interpreter execution");
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            expected,
            ["4", "Marker { label: home, point: Point { x: 4, y: 7 } }"]
        );

        assert_ir_failure_matches(vec![
            Instruction::LoadLiteral(Literal::Integer("1".into())),
            Instruction::LoadField("x".into()),
        ]);
        assert_ir_failure_matches(vec![
            Instruction::LoadLiteral(Literal::Integer("1".into())),
            Instruction::MakeStruct {
                type_name: "Point".into(),
                fields: vec!["x".into()],
            },
            Instruction::LoadField("missing".into()),
        ]);
    }

    #[test]
    fn logical_operators_short_circuit_and_control_flow_matches_interpreter() {
        let source = "
            fn right() -> Bool { print(\"rhs\"); return true }
            fn main() {
                print(false && right())
                print(true || right())
                print(true && right())
                print(false || right())
                print(false || true && false)
                let mut index = 0
                while (index < 2) {
                    print(index)
                    index = index + 1
                }
                if (index == 2) { print(\"done\") } else { print(\"bad\") }
            }
        ";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .expect("valid syntax");
        let ir = crate::compiler::ir::lower_program(&program).expect("valid types");
        let expected = crate::compiler::interpreter::run(&ir).expect("interpreter execution");
        assert_eq!(
            expected,
            ["false", "true", "rhs", "true", "rhs", "true", "false", "0", "1", "done"]
        );
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn float_aliases_preserve_widening_at_typed_boundaries() {
        let source = "
            type Decimal = Float;
            type PreciseDecimal = Decimal;
            fn show(value: PreciseDecimal) {
                print(std::to_string(value))
            }
            fn rounded() -> Decimal {
                return 9007199254740993
            }
            fn main() {
                let local: PreciseDecimal = 9007199254740993
                print(std::to_string(local))
                show(9007199254740993)
                print(std::to_string(rounded()))
            }
        ";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .expect("valid syntax");
        let ir = crate::compiler::ir::lower_program(&program).expect("valid types");
        let expected = ["9007199254740992", "9007199254740992", "9007199254740992"];
        assert_eq!(
            crate::compiler::interpreter::run(&ir).expect("interpreter execution"),
            expected
        );
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn malformed_array_widening_reports_matching_runtime_errors() {
        for instructions in [
            vec![Instruction::WidenFloatArray { depth: 1 }],
            vec![
                Instruction::MakeArray { length: 0 },
                Instruction::WidenFloatArray { depth: 0 },
            ],
            vec![
                Instruction::LoadLiteral(Literal::Integer("1".into())),
                Instruction::WidenFloatArray { depth: 1 },
            ],
            vec![
                Instruction::LoadLiteral(Literal::String("bad".into())),
                Instruction::MakeArray { length: 1 },
                Instruction::WidenFloatArray { depth: 1 },
            ],
            vec![
                Instruction::LoadLiteral(Literal::Integer("1".into())),
                Instruction::MakeArray { length: 1 },
                Instruction::WidenFloatArray { depth: usize::MAX },
            ],
            vec![
                Instruction::MakeArray { length: 0 },
                Instruction::MakeArray { length: 1 },
                Instruction::WidenFloatArray { depth: 1 },
            ],
        ] {
            assert_ir_failure_matches(instructions);
        }
    }

    #[test]
    fn whole_array_replacement_preserves_numeric_element_types() {
        let source = r#"
            fn main() {
                let ints = [3, 5]
                let mut floats = [0.0]
                floats = ints
                print(floats[0] / 2)
                print(floats[1] / 2)
                print(ints[0] / 2)
                floats = []
                print(floats)
                floats = [7]
                print(floats[0] / 2)
                let mut rows = [[1.0], [3]]
                print(rows[1][0] / 2)
                rows[0] = [5]
                print(rows[0][0] / 2)
                rows = [[7], [9]]
                print(rows[1][0] / 2)
            }
        "#;
        let program = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .unwrap();
        let ir = crate::compiler::ir::lower_program(&program).unwrap();
        let expected = ["1.5", "2.5", "1", "[]", "3.5", "1.5", "2.5", "4.5"];
        assert_eq!(crate::compiler::interpreter::run(&ir).unwrap(), expected);
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn inferred_record_fields_and_array_copies_preserve_float_widening() {
        let source = r#"
            type Decimal = Float;
            struct Measurement { amount: Decimal }
            struct Box { measurement: Measurement }
            fn measure() -> Measurement { return Measurement { amount: 3 } }
            fn inspect(box: Box) {
                let mut value = box.measurement.amount
                value = 3
                print(value / 2)
            }
            fn main() {
                let box = Box { measurement: measure() }
                inspect(box)
                let mut value = measure().amount
                value = 3
                print(value / 2)
                let mut mixed = [1, box.measurement.amount]
                mixed[0] = 3
                print(mixed[0] / 2)
                let mut copy = mixed
                copy[0] = 5
                print(copy[0] / 2)
                let mut literal = [1, 3.0]
                literal[0] = 3
                print(literal[0] / 2)
                let mut indexed = [measure()][0].amount
                indexed = 3
                print(indexed / 2)
            }
        "#;
        let program = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .unwrap();
        let ir = crate::compiler::ir::lower_program(&program).unwrap();
        let expected = ["1.5", "1.5", "1.5", "2.5", "1.5", "1.5"];
        assert_eq!(crate::compiler::interpreter::run(&ir).unwrap(), expected);
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn reassignment_preserves_float_widening() {
        let source = "
            type Decimal = Float;
            struct Measurement { amount: Decimal }
            fn main() {
                let mut value: Decimal = 3
                value = 3
                print(value / 2)

                let mut inferred = 3.0
                inferred = 3
                print(inferred / 2)

                let mut values = [3.0]
                values[0] = 3
                print(values[0] / 2)

                let mixed = [1, 3.0, 5]
                print(mixed[0] / 2)
                print(mixed[1] / 2)
                print(mixed[2] / 2)

                let measurement = Measurement { amount: 3 }
                print(measurement.amount / 2)
            }
        ";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .expect("valid syntax");
        let ir = crate::compiler::ir::lower_program(&program).expect("valid types");
        let expected = ["1.5", "1.5", "1.5", "0.5", "1.5", "2.5", "1.5"];
        assert_eq!(
            crate::compiler::interpreter::run(&ir).expect("interpreter execution"),
            expected
        );
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn shadowed_branch_bindings_do_not_overwrite_outer_values() {
        let source = "
            fn main() {
                let value = 1
                if (true) {
                    let value = 2
                    print(value)
                }
                print(value)

                let count = 1
                let mut iterations = 0
                while (iterations < 1) {
                    let count = 20
                    print(count)
                    iterations = iterations + 1
                }
                print(count)
            }
        ";
        let program = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .expect("valid syntax");
        let ir = crate::compiler::ir::lower_program(&program).expect("valid types");
        let expected = ["2", "1", "20", "1"];
        assert_eq!(
            crate::compiler::interpreter::run(&ir).expect("interpreter execution"),
            expected
        );
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn conditional_jump_validation_matches_for_taken_and_untaken_branches() {
        assert_ir_failure_matches(vec![
            Instruction::LoadLiteral(Literal::Boolean(true)),
            Instruction::JumpIfFalse { target: 3 },
        ]);
        assert_ir_failure_matches(vec![
            Instruction::LoadLiteral(Literal::Boolean(false)),
            Instruction::JumpIfFalse { target: 3 },
        ]);
        let program = IrProgram {
            functions: vec![IrFunction {
                name: "main".into(),
                parameters: vec![],
                instructions: vec![
                    Instruction::LoadLiteral(Literal::Boolean(false)),
                    Instruction::JumpIfFalse { target: 2 },
                ],
            }],
        };
        assert!(crate::compiler::interpreter::run(&program).is_ok());
        let output = execute_javascript(&render_javascript(&program));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn assert_ir_failure_matches(instructions: Vec<Instruction>) {
        let program = IrProgram {
            functions: vec![IrFunction {
                name: "main".into(),
                parameters: vec![],
                instructions,
            }],
        };
        let expected = crate::compiler::interpreter::run(&program).unwrap_err();
        // Compare the error value before Node formats newlines for host stderr.
        let script = format!("try {{ (function() {{ {} }})(); }} catch (error) {{ if (error.message !== {}) throw error; process.exitCode = 42; }}",
            render_javascript(&program), js_string(&expected));
        let output = execute_javascript(&script);
        assert_eq!(
            output.status.code(),
            Some(42),
            "expected: {expected}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn lines_unique_rejects_malformed_ir_in_both_engines() {
        assert_ir_failure_matches(vec![
            Instruction::LoadLiteral(Literal::Integer("1".into())),
            Instruction::Call {
                name: "std::lines_unique".into(),
                arguments: 1,
            },
        ]);
        assert_ir_failure_matches(vec![Instruction::Call {
            name: "std::lines_unique".into(),
            arguments: 0,
        }]);
    }

    #[test]
    fn malformed_ir_stack_and_name_errors_match_interpreter() {
        for instructions in [
            vec![Instruction::Pop],
            vec![Instruction::StoreName("value".into())],
            vec![Instruction::WidenFloat],
            vec![Instruction::Binary("+".into())],
            vec![
                Instruction::LoadLiteral(Literal::Integer("1".into())),
                Instruction::Binary("+".into()),
            ],
            vec![Instruction::Call {
                name: "print".into(),
                arguments: 1,
            }],
            vec![Instruction::LoadName("missing".into())],
        ] {
            let program = IrProgram {
                functions: vec![IrFunction {
                    name: "main".into(),
                    parameters: vec![],
                    instructions,
                }],
            };
            let expected = crate::compiler::interpreter::run(&program).unwrap_err();
            let output = execute_javascript(&render_javascript(&program));
            assert!(!output.status.success(), "expected: {expected}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&expected),
                "expected: {expected}"
            );
        }
    }

    #[test]
    fn javascript_rejects_ir_call_arity_like_interpreter() {
        for count in [0, 2] {
            let mut instructions =
                vec![Instruction::LoadLiteral(Literal::Integer("1".into())); count];
            instructions.push(Instruction::Call {
                name: "identity".into(),
                arguments: count,
            });
            let program = IrProgram {
                functions: vec![
                    IrFunction {
                        name: "main".into(),
                        parameters: vec![],
                        instructions,
                    },
                    IrFunction {
                        name: "identity".into(),
                        parameters: vec!["value".into()],
                        instructions: vec![
                            Instruction::LoadName("value".into()),
                            Instruction::Return,
                        ],
                    },
                ],
            };
            let expected = crate::compiler::interpreter::run(&program).unwrap_err();
            let output = execute_javascript(&render_javascript(&program));
            assert!(!output.status.success(), "invalid arity must fail: {count}");
            assert!(String::from_utf8_lossy(&output.stderr).contains(&expected));
        }
        let program = IrProgram {
            functions: vec![IrFunction {
                name: "main".into(),
                parameters: vec!["value".into()],
                instructions: vec![],
            }],
        };
        let expected = crate::compiler::interpreter::run(&program).unwrap_err();
        let output = execute_javascript(&render_javascript(&program));
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(&expected));
    }

    fn execute_javascript(source: &str) -> std::process::Output {
        use std::io::Write as _;
        use std::process::{Command, Stdio};
        let mut child = Command::new("node")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Node.js is required for backend execution tests");
        child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(source.as_bytes())
            .expect("write JavaScript");
        child.wait_with_output().expect("wait for Node.js")
    }

    #[test]
    fn unicode_string_ordering_matches_interpreter() {
        let pairs = [
            ("😀", "\u{e000}"),
            ("\u{e000}", "😀"),
            ("😀a", "😀b"),
            ("", "😀"),
            ("😀", ""),
            ("", ""),
            ("😀", "😀a"),
            ("😀a", "😀"),
            ("é", "e\u{301}"),
            ("same", "same"),
        ];
        let mut source = String::from("fn main() {\n");
        for (left, right) in pairs {
            for operator in ["<", "<=", ">", ">=", "==", "!="] {
                source.push_str(&format!("print(\"{left}\" {operator} \"{right}\")\n"));
            }
        }
        source.push('}');
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(&source)
            .unwrap();
        let ir = crate::compiler::ir::lower_program(&parsed).unwrap();
        let expected = crate::compiler::interpreter::run(&ir).unwrap();
        assert_eq!(&expected[..4], ["false", "false", "true", "true"]);
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn private_helpers_match_interpreter_and_preserve_bare_calls() {
        let source = "fn value() -> Int { return 7 }
            mod math {
                fn value() -> Int { return 42 }
                fn helper() -> Float { return math::value() }
                export fn answer() -> Float { print(value()); return math::helper() }
            }
            fn main() { print(math::answer()) }";
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .unwrap();
        let ir = crate::compiler::ir::lower_program(&parsed).unwrap();
        assert!(ir
            .functions
            .iter()
            .any(|function| function.name == "math::helper"));
        let expected = crate::compiler::interpreter::run(&ir).unwrap();
        assert_eq!(expected, ["7", "42"]);
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn private_recursion_obeys_both_engine_depth_limits() {
        for body in [
            "fn first() { math::first() }",
            "fn first() { math::second() } fn second() { math::first() }",
        ] {
            let source = format!("mod math {{ {body} export fn start() {{ math::first() }} }} fn main() {{ math::start() }}");
            let parsed = crate::compiler::parser::Parser::new()
                .parse_source(&source)
                .unwrap();
            let ir = crate::compiler::ir::lower_program(&parsed).unwrap();
            let expected = crate::compiler::interpreter::run(&ir).unwrap_err();
            let script = format!("try {{ (function() {{ {} }})(); }} catch (error) {{ console.log(error.message); }}", render_javascript(&ir));
            let output = execute_javascript(&script);
            assert!(output.status.success());
            assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), expected);
        }
    }

    #[test]
    fn javascript_depth_recovers_after_returns_and_errors() {
        let source =
            "fn main() {} fn recurse() { recurse() } fn early() { return } fn fallthrough() {}";
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(source)
            .unwrap();
        let ir = crate::compiler::ir::lower_program(&parsed).unwrap();
        let mut script = render_javascript(&ir);
        script.push_str(r#"
            for (let i = 0; i < 300; i++) {
                svrFunctions.early();
                svrFunctions.fallthrough();
            }
            for (let i = 0; i < 2; i++) {
                try { svrFunctions.recurse(); throw new Error('unexpected success'); }
                catch (error) {
                    if (error.message !== 'maximum call depth of 256 exceeded in `recurse`') throw error;
                }
                svrFunctions.early();
                svrFunctions.fallthrough();
            }
            console.log('recovered');
        "#);
        let output = execute_javascript(&script);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "recovered"
        );
    }

    #[test]
    fn javascript_call_depth_matches_interpreter() {
        for count in [256, 257] {
            let mut functions = Vec::new();
            for index in 0..count {
                let name = if index == 0 {
                    "main".to_owned()
                } else {
                    format!("f{index}")
                };
                let instructions = if index + 1 < count {
                    vec![
                        Instruction::Call {
                            name: format!("f{}", index + 1),
                            arguments: 0,
                        },
                        Instruction::Return,
                    ]
                } else {
                    vec![
                        Instruction::LoadLiteral(Literal::String("reached".into())),
                        Instruction::Call {
                            name: "print".into(),
                            arguments: 1,
                        },
                        Instruction::Return,
                    ]
                };
                functions.push(IrFunction {
                    name,
                    parameters: vec![],
                    instructions,
                });
            }
            let ir = IrProgram { functions };
            let expected = crate::compiler::interpreter::run(&ir);
            let script = format!("try {{ (function() {{ {} }})(); }} catch (error) {{ console.log(error.message); }}", render_javascript(&ir));
            let output = execute_javascript(&script);
            assert!(output.status.success());
            let actual = String::from_utf8(output.stdout).unwrap();
            match expected {
                Ok(lines) => assert_eq!(actual.trim_end(), lines.join("\n")),
                Err(error) => assert_eq!(actual.trim_end(), error),
            }
        }
    }

    #[test]
    fn source_strings_survive_javascript_emission() {
        let source = "fn main() { print(\"\0".to_owned()
            + "123\"); print(\"line\\nquote\\\"slash\\\\\"); print(\"é😀\u{2028}\u{2029}\") }";
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(&source)
            .unwrap();
        let ir = crate::compiler::ir::lower_program(&parsed).unwrap();
        let expected = crate::compiler::interpreter::run(&ir).unwrap().join("\n") + "\n";
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }

    #[test]
    fn javascript_strings_match_interpreter() {
        use crate::compiler::ir::{Instruction, IrFunction, Literal};
        let values = vec![
            (0u8..=31).map(char::from).collect::<String>(),
            "\0".to_owned() + "123",
            "quotes: \" backslash: \\ tab: \t".to_owned(),
            "café 😀\u{2028}\u{2029}\u{007f}\u{0085}".to_owned(),
        ];
        let mut instructions = Vec::new();
        for value in values {
            instructions.push(Instruction::LoadLiteral(Literal::String(value)));
            instructions.push(Instruction::Call {
                name: "std::print".into(),
                arguments: 1,
            });
            instructions.push(Instruction::Pop);
        }
        let ir = IrProgram {
            functions: vec![IrFunction {
                name: "main".into(),
                parameters: vec![],
                instructions,
            }],
        };
        let expected = crate::compiler::interpreter::run(&ir).unwrap().join("\n") + "\n";
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }

    #[test]
    fn string_concatenation_matches_interpreter() {
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(include_str!("../../examples/strings/main.svr"))
            .expect("valid syntax");
        let ir = crate::compiler::ir::lower_program(&parsed).expect("inferred String types");
        let expected = crate::compiler::interpreter::run(&ir).expect("string execution");
        assert_eq!(expected, vec!["abc!", "2", "true"]);
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual = String::from_utf8(output.stdout).unwrap();
        assert_eq!(actual.lines().collect::<Vec<_>>(), expected);
    }

    #[test]
    fn numeric_javascript_matches_expected_output() {
        let parsed = crate::compiler::parser::Parser::new()
            .parse_source(include_str!("../../examples/numbers/main.svr"))
            .expect("valid syntax");
        let ir = crate::compiler::ir::lower_program(&parsed).expect("valid types");
        let output = execute_javascript(&render_javascript(&ir));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual = String::from_utf8(output.stdout).expect("UTF-8 output");
        assert_eq!(
            actual.lines().collect::<Vec<_>>(),
            include_str!("../../tests/fixtures/numbers.stdout")
                .lines()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn numeric_javascript_errors_match_interpreter() {
        let mut javascript = String::new();
        let mut expected = Vec::new();
        for expression in [
            "9223372036854775807 + 1",
            "(0 - 9223372036854775807 - 1) - 1",
            "9223372036854775807 * 2",
            "(0 - 9223372036854775807 - 1) / (0 - 1)",
            "1 / 0",
            "1.0 / 0",
            "1 / 0.0",
        ] {
            let parsed = crate::compiler::parser::Parser::new()
                .parse_source(&format!("fn main() {{ print({expression}) }}"))
                .expect("valid syntax");
            let ir = crate::compiler::ir::lower_program(&parsed).expect("valid types");
            expected.push(crate::compiler::interpreter::run(&ir).expect_err("runtime failure"));
            javascript.push_str("try { (function() {\n");
            javascript.push_str(&render_javascript(&ir));
            javascript.push_str("})(); console.log('unexpected success'); } catch (error) { console.log(error.message); }\n");
        }
        let output = execute_javascript(&javascript);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let actual = String::from_utf8(output.stdout).expect("UTF-8 output");
        assert_eq!(actual.lines().collect::<Vec<_>>(), expected);
    }

    #[test]
    fn renders_function_header() {
        let output = render(&IrProgram {
            functions: vec![crate::compiler::ir::IrFunction {
                name: "main".into(),
                parameters: Vec::new(),
                instructions: vec![Instruction::Return],
            }],
        });
        assert!(output.contains("function main:"));
        assert!(output.contains("return"));
    }

    #[test]
    fn renders_javascript_backend() {
        let output = render_javascript(&IrProgram {
            functions: vec![crate::compiler::ir::IrFunction {
                name: "main".into(),
                parameters: Vec::new(),
                instructions: vec![
                    Instruction::LoadLiteral(Literal::String("Hello".into())),
                    Instruction::Call {
                        name: "std::println".into(),
                        arguments: 1,
                    },
                ],
            }],
        });
        assert!(output.contains("function svr_fn_0()"));
        assert!(output.contains("svrFunctions[\"main\"] = svr_fn_0;"));
        assert!(output.contains("svrWriteLine(args[0])"));
    }
}
