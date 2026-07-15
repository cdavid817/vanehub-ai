export type ServiceErrorCode = "validation" | "not-found" | "unsupported-runtime" | "runtime" | "unknown";

export class ServiceError extends Error {
  readonly code: ServiceErrorCode;
  readonly cause: unknown;

  constructor(code: ServiceErrorCode, message: string, cause?: unknown) {
    super(message);
    this.name = "ServiceError";
    this.code = code;
    this.cause = cause;
  }
}

export function unsupportedRuntimeError(message: string) {
  return new ServiceError("unsupported-runtime", message);
}

export function normalizeServiceError(error: unknown): ServiceError {
  if (error instanceof ServiceError) return error;

  const message = error instanceof Error ? error.message : String(error);
  const normalized = message.toLowerCase();
  const code: ServiceErrorCode = normalized.includes("validation")
    ? "validation"
    : normalized.includes("not found")
      ? "not-found"
      : normalized.includes("unsupported")
        ? "unsupported-runtime"
        : error instanceof Error
          ? "runtime"
          : "unknown";

  return new ServiceError(code, message, error);
}

export function withServiceErrorNormalization<T extends object>(service: T): T {
  return new Proxy(service, {
    get(target, property, receiver) {
      const value = Reflect.get(target, property, receiver);
      if (typeof value !== "function") return value;

      return (...args: unknown[]) => {
        try {
          const result = value.apply(target, args);
          if (result && typeof result.then === "function") {
            return result.catch((error: unknown) => Promise.reject(normalizeServiceError(error)));
          }
          return result;
        } catch (error) {
          throw normalizeServiceError(error);
        }
      };
    },
  });
}
