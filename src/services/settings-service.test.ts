import { describe, expect, it } from "vitest";
import { i18n } from "../i18n";
import { defaultAppSettings, normalizeAppSettings, normalizeNetworkProxyBypass } from "./settings-service";
import { webSettingsClient } from "./web-settings-client";

describe("settings-service", () => {
  it("normalizes logging settings with defaults", () => {
    const settings = normalizeAppSettings({
      applicationLanguage: "en",
      fontSize: "16px",
      theme: "minimal",
      defaultFolderPath: "D:/workspace",
      logDirectory: "D:/logs",
      loggingPolicy: {
        retentionDays: 30,
        archiveEnabled: true,
        redactionEnabled: true,
        levels: ["error", "warn", "info", "debug"],
        canOpenDirectory: true,
      },
    });

    expect(settings.logDirectory).toBe("D:/logs");
    expect(settings.launchOnStartup).toBe(false);
    expect(settings.loggingPolicy.retentionDays).toBe(30);
    expect(settings.loggingPolicy.canOpenDirectory).toBe(true);
    expect(settings.loggingPolicy.levels).toEqual(["error", "warn", "info", "debug"]);
  });

  it("falls back for invalid logging policy values", () => {
    const settings = normalizeAppSettings({
      loggingPolicy: {
        retentionDays: "bad",
        archiveEnabled: "bad",
        redactionEnabled: "bad",
        levels: ["trace"],
        canOpenDirectory: "bad",
      },
    });

    expect(settings.loggingPolicy).toEqual(defaultAppSettings.loggingPolicy);
  });

  it("normalizes network proxy settings with defaults", () => {
    const settings = normalizeAppSettings({
      networkProxyUrl: "socks5://127.0.0.1:1080",
      networkProxyBypass: " localhost, 127.0.0.1 ::1 ",
    });

    expect(settings.networkProxyUrl).toBe("socks5://127.0.0.1:1080");
    expect(settings.networkProxyBypass).toBe("localhost,127.0.0.1,::1");
    expect(normalizeNetworkProxyBypass("localhost 127.0.0.1")).toBe("localhost,127.0.0.1");
  });

  it("falls back for invalid network proxy settings", () => {
    const settings = normalizeAppSettings({
      networkProxyUrl: "ftp://127.0.0.1:21",
      networkProxyBypass: "localhost\nbad",
    });

    expect(settings.networkProxyUrl).toBe(defaultAppSettings.networkProxyUrl);
    expect(settings.networkProxyBypass).toBe(defaultAppSettings.networkProxyBypass);
  });

  it("keeps web mock client log events as no-op and blocks opening local directories", async () => {
    await i18n.changeLanguage("en");

    await expect(
      webSettingsClient.reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message: "failed",
        source: "test",
      }),
    ).resolves.toBeUndefined();

    await expect(webSettingsClient.openLogDirectory()).rejects.toThrow("desktop runtime");
    await expect(webSettingsClient.openDatabaseDirectory()).rejects.toThrow("desktop runtime");
    await expect(webSettingsClient.testNetworkProxy({ url: "http://127.0.0.1:7890", bypass: "" })).rejects.toThrow(
      "desktop runtime",
    );
    await expect(webSettingsClient.scanNetworkProxies()).rejects.toThrow("desktop runtime");
  });

  it("preserves launch-on-startup shape in the web mock adapter", async () => {
    const dataInfo = await webSettingsClient.getDataManagementInfo();

    await expect(webSettingsClient.setLaunchOnStartup(true)).rejects.toThrow("desktop runtime");
    expect((await webSettingsClient.getSettings()).launchOnStartup).toBe(false);
    expect(dataInfo.canOpenDirectory).toBe(false);
    expect(dataInfo.databasePath).toContain("localStorage");
  });
});
