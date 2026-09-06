// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from "vitest";
import "../i18n";
import { defaultAppSettings, normalizeAppSettings, validateSettingValue } from "./settings-service";
import { webSettingsClient } from "./web-settings-client";

describe("cliTerminalTheme setting", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("defaults to dark on a fresh install and when an older store lacks the field", () => {
    expect(defaultAppSettings.cliTerminalTheme).toBe("dark");
    expect(normalizeAppSettings({}).cliTerminalTheme).toBe("dark");
    expect(normalizeAppSettings({ theme: "minimal", fontSize: "16px" }).cliTerminalTheme).toBe("dark");
  });

  it("keeps a valid stored value and falls back safely from an invalid one", () => {
    expect(normalizeAppSettings({ cliTerminalTheme: "light" }).cliTerminalTheme).toBe("light");
    expect(normalizeAppSettings({ cliTerminalTheme: "solarized" }).cliTerminalTheme).toBe("dark");
    expect(normalizeAppSettings({ cliTerminalTheme: 1 }).cliTerminalTheme).toBe("dark");
    // The fallback touches nothing else.
    expect(normalizeAppSettings({ cliTerminalTheme: "solarized", theme: "futuristic" }).theme).toBe("futuristic");
  });

  it("refuses an invalid write instead of coercing it", async () => {
    expect(() => validateSettingValue("cliTerminalTheme", "solarized" as never)).toThrow("cliTerminalTheme");
    await webSettingsClient.saveSetting({ key: "cliTerminalTheme", value: "light" });
    await expect(webSettingsClient.saveSetting({ key: "cliTerminalTheme", value: "solarized" as never })).rejects.toThrow();
    await expect(webSettingsClient.getSettings()).resolves.toMatchObject({ cliTerminalTheme: "light" });
  });

  it("round-trips light and dark through the Web adapter and stays independent of the app theme", async () => {
    await webSettingsClient.saveSetting({ key: "theme", value: "futuristic" });
    await webSettingsClient.saveSetting({ key: "cliTerminalTheme", value: "light" });
    await expect(webSettingsClient.getSettings()).resolves.toMatchObject({ cliTerminalTheme: "light", theme: "futuristic" });

    await webSettingsClient.saveSetting({ key: "theme", value: "minimal" });
    await expect(webSettingsClient.getSettings()).resolves.toMatchObject({ cliTerminalTheme: "light", theme: "minimal" });

    await webSettingsClient.saveSetting({ key: "cliTerminalTheme", value: "dark" });
    await expect(webSettingsClient.getSettings()).resolves.toMatchObject({ cliTerminalTheme: "dark", theme: "minimal" });
  });
});
