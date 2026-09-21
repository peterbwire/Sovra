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

function svrCheckedInt(value) {
  if (value < -9223372036854775808n || value > 9223372036854775807n) {
    throw new Error("integer overflow");
  }
  return value;
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
  let result;
  switch (operator) {
    case "+": result = left + right; break;
    case "-": result = left - right; break;
    case "*": result = left * right; break;
    case "/": result = left / right; break;
    case "==": return left === right;
    case "!=": return left !== right;
    case "<": return left < right;
    case "<=": return left <= right;
    case ">": return left > right;
    case ">=": return left >= right;
    default: throw new Error("unsupported runtime operation `" + operator + "`");
  }
  return typeof result === "bigint" ? svrCheckedInt(result) : result;
}
