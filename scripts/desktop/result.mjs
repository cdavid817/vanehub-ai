export const RESULT_STATUSES = ["PASSED", "FAILED", "BLOCKED", "NOT RUN", "NOT REQUIRED"];

export function verificationExitCode(status) {
  if (!RESULT_STATUSES.includes(status)) throw new TypeError(`Unknown verification status: ${status}`);
  return status === "FAILED" || status === "BLOCKED" ? 1 : 0;
}

export function createLayerResult({ layer, status, reason, ...details }) {
  if (!RESULT_STATUSES.includes(status)) throw new TypeError(`Unknown verification status: ${status}`);
  if (status === "NOT REQUIRED" && !reason) throw new TypeError("NOT REQUIRED results must include a reason.");
  return { layer, status, ...(reason ? { reason } : {}), ...details };
}
