import { describe, expect, it } from "vitest";
import { formatDiagnosticSummary } from "./diagnostic-field";

describe("formatDiagnosticSummary", () => {
  it("renders one label: value line per field, in order", () => {
    expect(formatDiagnosticSummary(
      [
        { label: "Version", value: "2.1.237" },
        { label: "Status", value: "ready" },
      ],
      "unavailable",
    )).toBe("Version: 2.1.237\nStatus: ready");
  });

  it("marks a null value unavailable instead of omitting or inventing it", () => {
    expect(formatDiagnosticSummary(
      [{ label: "Last checked", value: null }],
      "unavailable",
    )).toBe("Last checked: unavailable");
  });

  it("returns an empty string for no fields, not a stray newline", () => {
    expect(formatDiagnosticSummary([], "unavailable")).toBe("");
  });
});
