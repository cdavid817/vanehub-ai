import { describe, expect, it } from "vitest";
import { normalizeServiceError } from "./service-error";
import {
  ContextQualityServiceError,
  normalizeContextQualityError,
} from "./context-quality-error";

describe("context quality safe errors", () => {
  it("maps native validation strings to bounded context error codes", () => {
    expect(normalizeContextQualityError("validation error: Context quality range must be 7, 30, or 90 days."))
      .toMatchObject({ code: "invalid-range" });
    expect(normalizeContextQualityError("validation error: Context quality cursor is invalid."))
      .toMatchObject({ code: "invalid-cursor" });
    expect(normalizeContextQualityError("storage error: database unavailable"))
      .toMatchObject({ code: "unavailable" });
  });

  it("survives the shared runtime adapter normalization", () => {
    const error = new ContextQualityServiceError("invalid-cursor", "Cursor is invalid.");
    expect(normalizeServiceError(error)).toBe(error);
  });
});
