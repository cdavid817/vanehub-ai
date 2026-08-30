import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { resetLegacyIdDiagnosticsForTests, warnUnmappedLegacyId } from "./legacy-id-diagnostics";

let warnSpy: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  resetLegacyIdDiagnosticsForTests();
  warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
});

afterEach(() => {
  warnSpy.mockRestore();
});

test("warns once for an unmapped legacy id, naming the category and id", () => {
  warnUnmappedLegacyId("destination", "mission-control-v1");

  expect(warnSpy).toHaveBeenCalledTimes(1);
  expect(warnSpy.mock.calls[0]?.[0]).toContain("destination");
  expect(warnSpy.mock.calls[0]?.[0]).toContain("mission-control-v1");
});

test("dedupes repeated warnings for the same category and id", () => {
  warnUnmappedLegacyId("session-tab", "chat");
  warnUnmappedLegacyId("session-tab", "chat");
  warnUnmappedLegacyId("session-tab", "chat");

  expect(warnSpy).toHaveBeenCalledTimes(1);
});

test("does not dedupe across different categories or different ids", () => {
  warnUnmappedLegacyId("destination", "loops");
  warnUnmappedLegacyId("session-tab", "loops");
  warnUnmappedLegacyId("destination", "goals");

  expect(warnSpy).toHaveBeenCalledTimes(3);
});

test("truncates an unexpectedly long id rather than logging it verbatim", () => {
  const longId = "x".repeat(500);

  warnUnmappedLegacyId("destination", longId);

  const message = warnSpy.mock.calls[0]?.[0] as string;
  expect(message.length).toBeLessThan(longId.length);
  expect(message).toContain("…");
});
