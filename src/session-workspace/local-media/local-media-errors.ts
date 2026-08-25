import {
  localMediaErrorCodes,
  type LocalMediaErrorCode,
} from "../../types/local-media";

const KNOWN = new Set<string>(localMediaErrorCodes);

/**
 * Recover the stable code from a rejected service call.
 *
 * The command contract serializes an error as a bare string, and that string is exactly the code.
 * Anything unrecognized -- a transport failure, a bug -- becomes `ENGINE_UNAVAILABLE` rather than
 * being shown raw: the composer renders errors from locale keys, and a passthrough message would
 * either display an English internal string or render as a missing translation.
 */
export function localMediaErrorCodeFrom(error: unknown): LocalMediaErrorCode {
  const raw =
    typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : String(error ?? "");
  // `ServiceError` prefixes nothing, but a thrown `Error` renders as its message; both reduce to
  // the trailing token, which is where the code sits either way.
  const candidate = raw.trim().split(/\s+/u).pop() ?? "";
  return KNOWN.has(candidate) ? (candidate as LocalMediaErrorCode) : "ENGINE_UNAVAILABLE";
}

/** The locale key for a code, derived the same way the native side derives it. */
export function localMediaMessageKey(code: LocalMediaErrorCode): string {
  const camel = code
    .toLowerCase()
    .split("_")
    .map((part, index) => (index === 0 ? part : part.charAt(0).toUpperCase() + part.slice(1)))
    .join("");
  return `localMedia.errors.${camel}`;
}

/**
 * Whether the same action is worth offering again without the user changing anything.
 *
 * A busy engine clears on its own; a missing model does not, and offering "retry" for it wastes
 * the one interaction the user has.
 */
export function isRetryable(code: LocalMediaErrorCode): boolean {
  return (
    code === "ENGINE_BUSY" ||
    code === "WORKER_CRASHED" ||
    code === "WORKER_START_FAILED" ||
    code === "TEMP_STORAGE_FAILED" ||
    code === "AUDIO_CAPTURE_START_FAILED"
  );
}

/**
 * Outcomes the composer presents as information rather than failure.
 *
 * These leave the draft untouched and must not render as an error: the engine ran correctly and
 * found nothing, which is a fact about the input, not a fault.
 */
export function isInformationalOutcome(code: LocalMediaErrorCode): boolean {
  return code === "NO_TEXT_DETECTED" || code === "NO_SPEECH_DETECTED";
}
