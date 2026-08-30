import { describe, expect, it } from "vitest";
import { DESTINATION_LIFECYCLE } from "./destination-lifecycle";

describe("DESTINATION_LIFECYCLE", () => {
  it("keeps Sessions alive as draft-only, for its in-progress composer draft", () => {
    expect(DESTINATION_LIFECYCLE.sessions.keepAlive).toBe("draft-only");
  });

  it("defaults every other domain destination to never, matching their real unmount-on-switch behavior", () => {
    for (const destination of ["projects", "runs", "plan", "quality"] as const) {
      expect(DESTINATION_LIFECYCLE[destination].keepAlive).toBe("never");
    }
  });

  it("declares no always exception without a documented reason", () => {
    expect(Object.values(DESTINATION_LIFECYCLE).some((policy) => policy.keepAlive === "always")).toBe(false);
  });
});
