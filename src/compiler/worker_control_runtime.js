// Opt-in Node Worker transport v1. This is not a process or host-I/O sandbox.
const svrWorkerHost = require("node:worker_threads");
if (svrWorkerHost.isMainThread) throw new Error("JavaScript control requires a Node worker");
const svrEnvelopeProvided = svrWorkerHost.workerData != null && Object.prototype.hasOwnProperty.call(svrWorkerHost.workerData, "sovraControl");
const svrEnvelope = svrWorkerHost.workerData?.sovraControl;
let svrCancellation = null;
if (svrCancellationEnabled) {
  if (svrEnvelope === null || typeof svrEnvelope !== "object" || Array.isArray(svrEnvelope)
      || Object.keys(svrEnvelope).length !== 2
      || !Object.prototype.hasOwnProperty.call(svrEnvelope, "version")
      || !Object.prototype.hasOwnProperty.call(svrEnvelope, "cancellation")
      || svrEnvelope.version !== 1
      || !(svrEnvelope.cancellation instanceof SharedArrayBuffer)
      || svrEnvelope.cancellation.byteLength !== 4
      || svrEnvelope.cancellation.growable === true) {
    throw new Error("invalid JavaScript worker control");
  }
  svrCancellation = new Int32Array(svrEnvelope.cancellation);
} else if (svrEnvelopeProvided) {
  throw new Error("invalid JavaScript worker control");
}
function svrCheckpoint() {
  if (svrCancellation !== null && Atomics.load(svrCancellation, 0) !== 0) throw new Error("execution cancelled");
  if (svrElapsedAllowance !== null && process.hrtime.bigint() - svrControlOrigin >= svrElapsedAllowance) throw new Error("execution elapsed time allowance exceeded");
}
function svrWorkerReturn(stack) {
  if (stack.length) return stack.pop();
  svrCheckpoint();
  return svrUnit();
}
