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

  it("persists every supported application locale through the unchanged setting contract", async () => {
    for (const applicationLanguage of ["zh-CN", "en", "zh-TW", "ja", "ko"] as const) {
      const savedSettings = await webSettingsClient.saveSetting({
        key: "applicationLanguage",
        value: applicationLanguage,
      });
      expect(savedSettings.applicationLanguage).toBe(applicationLanguage);
      await expect(webSettingsClient.getSettings()).resolves.toMatchObject({ applicationLanguage });
    }
  });
});
