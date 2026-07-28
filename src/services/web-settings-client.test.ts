// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from "vitest";
import { webSettingsClient } from "./web-settings-client";

describe("web-settings-client", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("persists a setting to localStorage and restores it through the Web adapter", async () => {
    const savedSettings = await webSettingsClient.saveSetting({
      key: "fontSize",
      value: "12px",
    });

    expect(savedSettings.fontSize).toBe("12px");
    await expect(webSettingsClient.getSettings()).resolves.toMatchObject({
      fontSize: "12px",
    });
  });
});
