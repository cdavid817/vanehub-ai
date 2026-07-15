import { describe, expect, it } from "vitest";
import { ServiceError, normalizeServiceError, unsupportedRuntimeError, withServiceErrorNormalization } from "./service-error";

describe("service error normalization", () => {
  it("preserves existing ServiceError instances", () => {
    const error = unsupportedRuntimeError("Desktop runtime is required");

    expect(normalizeServiceError(error)).toBe(error);
    expect(error.code).toBe("unsupported-runtime");
  });

  it("maps thrown strings into typed service errors", () => {
    const error = normalizeServiceError("validation error: bad input");

    expect(error).toBeInstanceOf(ServiceError);
    expect(error.code).toBe("validation");
  });

  it("normalizes rejected adapter method errors", async () => {
    const service = withServiceErrorNormalization({
      async fail() {
        throw new Error("item not found");
      },
    });

    await expect(service.fail()).rejects.toMatchObject({ code: "not-found" });
  });
});
