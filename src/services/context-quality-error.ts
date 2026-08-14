import type { ContextQualityErrorCode, ContextQualitySafeError } from "../types/context-quality";

const errorCodes: ContextQualityErrorCode[] = ["invalid-range", "invalid-cursor", "unavailable"];

export class ContextQualityServiceError extends Error implements ContextQualitySafeError {
  constructor(readonly code: ContextQualityErrorCode, message: string, readonly cause?: unknown) {
    super(message);
    this.name = "ContextQualityServiceError";
  }
}

export function isContextQualityServiceError(error: unknown): error is ContextQualityServiceError {
  return error instanceof ContextQualityServiceError
    || (typeof error === "object" && error !== null
      && "code" in error && errorCodes.includes(error.code as ContextQualityErrorCode)
      && "message" in error && typeof error.message === "string");
}

export function normalizeContextQualityError(error: unknown): ContextQualityServiceError {
  if (isContextQualityServiceError(error)) return error;
  const message = error instanceof Error ? error.message : String(error);
  const normalized = message.toLowerCase();
  const code: ContextQualityErrorCode = normalized.includes("range must be")
    ? "invalid-range"
    : normalized.includes("cursor") || normalized.includes("page size")
      ? "invalid-cursor"
      : "unavailable";
  return new ContextQualityServiceError(code, message, error);
}
