import { describe, expect, it } from "vitest";
import { activateAppLanguage } from "../i18n";
import { appLanguages } from "../i18n/supported-locales";
import { defaultAppSettings, normalizeAppSettings, normalizeNetworkProxyBypass } from "./settings-service";
import { webSettingsClient } from "./web-settings-client";

describe("settings-service", () => {
  it.each(appLanguages)("accepts and preserves the supported locale %s", (applicationLanguage) => {
    expect(normalizeAppSettings({ applicationLanguage }).applicationLanguage).toBe(applicationLanguage);
  });

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

  it("defaults personalization settings to empty custom instructions and memory fully enabled", () => {
    const settings = normalizeAppSettings({});

    expect(settings.customInstructionsAboutUser).toBe("");
    expect(settings.customInstructionsStyleRules).toBe("");
    expect(settings.customInstructionsEnabled).toBe(true);
    expect(settings.memoryEnabled).toBe(true);
    expect(settings.memoryToolAssistedChatsEnabled).toBe(true);
    expect(settings.automaticContextCompactionEnabled).toBe(true);
    expect(settings.contextQualityRetentionDays).toBe(30);
  });

  it("keeps a reported personalization revision and refuses an unusable one", () => {
    // The page echoes this back on save. Accepting a negative or fractional value would send the
    // native side something no policy revision could equal, turning every save into a conflict.
    expect(normalizeAppSettings({ personalizationRevision: 12 }).personalizationRevision).toBe(12);
    for (const unusable of [-1, 1.5, "12", null, undefined, Number.NaN]) {
      expect(normalizeAppSettings({ personalizationRevision: unusable }).personalizationRevision).toBe(0);
    }
  });

  it("normalizes custom instructions and memory preference settings", () => {
    const settings = normalizeAppSettings({
      customInstructionsAboutUser: "Works on VaneHub AI.",
      customInstructionsStyleRules: "Always answer in Chinese.",
      customInstructionsEnabled: false,
      memoryEnabled: false,
      memoryToolAssistedChatsEnabled: false,
      automaticContextCompactionEnabled: false,
      contextQualityRetentionDays: 90,
    });

    expect(settings.customInstructionsAboutUser).toBe("Works on VaneHub AI.");
    expect(settings.customInstructionsStyleRules).toBe("Always answer in Chinese.");
    expect(settings.customInstructionsEnabled).toBe(false);
    expect(settings.memoryEnabled).toBe(false);
    expect(settings.memoryToolAssistedChatsEnabled).toBe(false);
    expect(settings.automaticContextCompactionEnabled).toBe(false);
    expect(settings.contextQualityRetentionDays).toBe(90);
  });

  it("accepts only bounded context quality retention options", () => {
    expect(normalizeAppSettings({ contextQualityRetentionDays: 7 }).contextQualityRetentionDays).toBe(7);
    expect(normalizeAppSettings({ contextQualityRetentionDays: 90 }).contextQualityRetentionDays).toBe(90);
    expect(normalizeAppSettings({ contextQualityRetentionDays: 14 }).contextQualityRetentionDays).toBe(30);
    expect(normalizeAppSettings({ contextQualityRetentionDays: "30" }).contextQualityRetentionDays).toBe(30);
  });

  it("falls back to empty custom-instruction fields when a field exceeds the character limit", () => {
    const settings = normalizeAppSettings({
      customInstructionsAboutUser: "x".repeat(3001),
      customInstructionsStyleRules: "x".repeat(3000),
    });

    expect(settings.customInstructionsAboutUser).toBe("");
    expect(settings.customInstructionsStyleRules).toBe("x".repeat(3000));
  });

  it("counts custom-instruction fields by Unicode code point, matching the Rust backend's char count instead of UTF-16 length", () => {
    // U+1F600 is a single Unicode scalar value but occupies a UTF-16 surrogate pair (String.length
    // === 2) — a naive `.length` check would wrongly treat 3000 of these as 6000 and reject them.
    const emoji = "\u{1F600}";
    const atLimit = emoji.repeat(3000);
    expect(atLimit.length).toBe(6000);

    const settings = normalizeAppSettings({ customInstructionsAboutUser: atLimit });

    expect(settings.customInstructionsAboutUser).toBe(atLimit);
  });

  it("keeps web mock client log events as no-op and blocks opening local directories", async () => {
    await activateAppLanguage("en");

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
