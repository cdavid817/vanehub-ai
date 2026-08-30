// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";
import { hasSeenLegacyRouteHint, markLegacyRouteHintSeen } from "./legacy-route-hint";

afterEach(() => localStorage.clear());

describe("legacy route hint dismissal", () => {
  it("has not been seen before it is marked", () => {
    expect(hasSeenLegacyRouteHint()).toBe(false);
  });

  it("stays seen across separate calls once marked", () => {
    markLegacyRouteHintSeen();
    expect(hasSeenLegacyRouteHint()).toBe(true);
    expect(hasSeenLegacyRouteHint()).toBe(true);
  });
});
