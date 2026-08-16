import { describe, expect, it } from "vitest";
import { webDesktopUpdateClient } from "./web-desktop-update";

describe("web desktop updater", () => {
  it("preserves adapter lifecycle without native side effects", async () => {
    const checked = await webDesktopUpdateClient.check();
    expect(checked.operationId).toMatch(/^web-update-/);
    expect(checked.snapshot.phase).toBe("available");
    const installed = await webDesktopUpdateClient.install();
    expect(installed.snapshot.phase).toBe("ready-to-restart");
  });
});
