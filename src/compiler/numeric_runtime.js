// Value-operation helpers emitted with the JavaScript backend.
function svrCompareStrings(left, right) {
  let leftIndex = 0;
  let rightIndex = 0;
  while (leftIndex < left.length && rightIndex < right.length) {
    const leftPoint = left.codePointAt(leftIndex);
    const rightPoint = right.codePointAt(rightIndex);
    if (leftPoint !== rightPoint) return leftPoint < rightPoint ? -1 : 1;
    leftIndex += leftPoint > 0xffff ? 2 : 1;
    rightIndex += rightPoint > 0xffff ? 2 : 1;
  }
  return leftIndex < left.length ? 1 : rightIndex < right.length ? -1 : 0;
}

function svrWidenFloat(value) {
  if (typeof value !== "bigint" && typeof value !== "number") {
    throw new Error("Float conversion expects Int or Float");
  }
  return Number(value);
}

function svrWidenFloatArray(value, depth) {
  if (depth === 0) throw new Error("array widening depth must be positive");
  // Conversion builds new arrays so widening never changes an Int source binding.
  const result = [];
  const pending = [[value, result, depth]];
  while (pending.length) {
    const [source, destination, remaining] = pending.pop();
    if (!Array.isArray(source)) throw new Error("array widening expects Array");
    for (let index = 0; index < source.length; index++) {
      if (remaining === 1) {
        destination.push(svrWidenFloat(source[index]));
      } else {
        const nested = [];
        destination.push(nested);
      }
    }
    if (remaining > 1) {
      for (let index = source.length - 1; index >= 0; index--) {
        pending.push([source[index], destination[index], remaining - 1]);
      }
    }
  }
  return result;
}

function svrCheckedInt(value) {
  if (value < -9223372036854775808n || value > 9223372036854775807n) {
    throw new Error("integer overflow");
  }
  return value;
}

function svrValueEqual(left, right) {
  const pending = [[left, right]];
  while (pending.length) {
    const [a, b] = pending.pop();
    if ((typeof a === "bigint" && typeof b === "number") ||
        (typeof a === "number" && typeof b === "bigint")) {
      if (Number(a) !== Number(b)) return false;
    } else if (Array.isArray(a) && Array.isArray(b)) {
      if (a.length !== b.length) return false;
      for (let index = 0; index < a.length; index++) pending.push([a[index], b[index]]);
    } else if (a !== null && b !== null && typeof a === "object" && typeof b === "object" &&
        Object.prototype.hasOwnProperty.call(a, "__svrStruct") &&
        Object.prototype.hasOwnProperty.call(b, "__svrStruct")) {
      const first = a.__svrStruct;
      const second = b.__svrStruct;
      if (first.typeName !== second.typeName || first.fieldOrder.length !== second.fieldOrder.length)
        return false;
      for (const name of first.fieldOrder) {
        if (!Object.prototype.hasOwnProperty.call(second.fields, name)) return false;
        pending.push([first.fields[name], second.fields[name]]);
      }
    } else if (a !== b) return false;
  }
  return true;
}

function svrBinary(operator, left, right) {
  // Rust UTF-8 ordering follows scalar values, not JavaScript UTF-16 code units.
  if (typeof left === "string" && typeof right === "string" &&
      (operator === "<" || operator === "<=" || operator === ">" || operator === ">=")) {
    left = svrCompareStrings(left, right);
    right = 0;
  }
  if ((typeof left === "bigint" && typeof right === "number") ||
      (typeof left === "number" && typeof right === "bigint")) {
    left = Number(left);
    right = Number(right);
  }
  if (operator === "/" && (right === 0 || right === 0n)) {
    throw new Error("division by zero");
  }
  // IR is public: reject host-language coercions even when source typing is bypassed.
  const numeric = typeof left === typeof right &&
    (typeof left === "bigint" || typeof left === "number");
  const concatenation = operator === "+" &&
    typeof left === "string" && typeof right === "string";
  if (operator !== "==" && operator !== "!=" && !numeric && !concatenation) {
    throw new Error("unsupported runtime operation `" + operator + "`");
  }
  let result;
  switch (operator) {
    case "+": result = left + right; break;
    case "-": result = left - right; break;
    case "*": result = left * right; break;
    case "/": result = left / right; break;
    case "==": return svrValueEqual(left, right);
    case "!=": return !svrValueEqual(left, right);
    case "<": return left < right;
    case "<=": return left <= right;
    case ">": return left > right;
    case ">=": return left >= right;
    default: throw new Error("unsupported runtime operation `" + operator + "`");
  }
  return typeof result === "bigint" ? svrCheckedInt(result) : result;
}
