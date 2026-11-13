// Model-v1 logical materialization; no physical heap/RSS or scratch accounting.
const svrMaxLogicalSize = 18446744073709551615n;
function svrSize(value) {
  if (value < 0n || value > svrMaxLogicalSize) throw new Error("logical materialization size overflow");
  return value;
}
function svrBytes(text) { return BigInt(Buffer.byteLength(text, "utf8")); }
function svrLabels(type, fields) {
  let bytes = svrBytes(type);
  for (const field of fields) bytes = svrSize(bytes + svrBytes(field));
  return bytes;
}
function svrMaterializationEnabled() { return svrNodeRemaining !== null || svrTextRemaining !== null; }
function svrRequireMaterialization(nodes, text) {
  svrSize(nodes); svrSize(text);
  if (svrNodeRemaining !== null && nodes > svrNodeRemaining) throw new Error("execution value node allowance exceeded");
  if (svrTextRemaining !== null && text > svrTextRemaining) throw new Error("execution text materialization allowance exceeded");
}
function svrMaterialize(nodes, text) {
  if (!svrMaterializationEnabled()) return;
  svrRequireMaterialization(nodes, text);
  if (svrNodeRemaining !== null) svrNodeRemaining -= nodes;
  if (svrTextRemaining !== null) svrTextRemaining -= text;
}
function svrFootprint(value) {
  let nodes = 0n, text = 0n, display = 0n;
  const pending = [{kind: 0, value}];
  while (pending.length) {
    const frame = pending.pop();
    if (frame.kind === 1) {
      if (frame.index < frame.value.length) {
        const item = frame.value[frame.index++];
        pending.push(frame, {kind: 0, value: item});
      }
      continue;
    }
    if (frame.kind === 2) {
      if (frame.index < frame.value.fieldOrder.length) {
        const name = frame.value.fieldOrder[frame.index++];
        const length = svrBytes(name);
        text = svrSize(text + length); display = svrSize(display + length + 2n);
        pending.push(frame, {kind: 0, value: frame.value.fields[name]});
      }
      continue;
    }
    const value = frame.value;
    nodes = svrSize(nodes + 1n);
    if (value === undefined) continue;
    if (typeof value === "string") { const length = svrBytes(value); text = svrSize(text + length); display = svrSize(display + length); }
    else if (typeof value === "boolean") display = svrSize(display + (value ? 4n : 5n));
    else if (typeof value === "bigint") {
      if (value < -9223372036854775808n || value > 9223372036854775807n) throw new Error("invalid runtime value footprint");
      display = svrSize(display + svrBytes(String(value)));
    } else if (typeof value === "number") display = svrSize(display + svrBytes(svrFormatFloat(value)));
    else if (Array.isArray(value)) {
      display = svrSize(display + 2n + BigInt(Math.max(value.length - 1, 0)) * 2n);
      pending.push({kind: 1, value, index: 0});
    } else if (value !== null && typeof value === "object" && Object.prototype.hasOwnProperty.call(value, "__svrStruct")) {
      const record = value.__svrStruct;
      const length = svrBytes(record.typeName);
      text = svrSize(text + length);
      display = svrSize(display + length + 5n + BigInt(Math.max(record.fieldOrder.length - 1, 0)) * 2n);
      pending.push({kind: 2, value: record, index: 0});
    } else throw new Error("invalid runtime value footprint");
  }
  return {nodes, text, display};
}
function svrAdmitSnapshot(value) {
  if (svrMaterializationEnabled()) { const size = svrFootprint(value); svrMaterialize(size.nodes, size.text); }
}
function svrIngress(value) { svrAdmitSnapshot(value); return value; }
function svrUnit() { svrMaterialize(1n, 0n); return undefined; }
function svrReturn(stack) { return stack.length ? stack.pop() : svrUnit(); }
function svrEscapedBytes(text) {
  let bytes = 0n;
  for (const character of text) {
    const point = character.codePointAt(0);
    bytes = svrSize(bytes + (point <= 31 || (point >= 127 && point <= 159) ? 8n : svrBytes(character)));
  }
  return bytes;
}

function svrGrowthLength(left, right) {
  const length = BigInt(left) + BigInt(right);
  if (length < 0n || length > 4294967295n) throw new Error("array result length exceeds limit");
  return Number(length);
}
