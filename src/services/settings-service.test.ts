import { describe, expect, it } from "vitest";
import { defaultAppSettings, normalizeAppSettings } from "./settings-service";
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

  it("keeps web mock client log events as no-op and blocks opening local directories", async () => {
    await expect(
      webSettingsClient.reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message: "failed",
        source: "test",
      }),
    ).resolves.toBeUndefined();

    await expect(webSettingsClient.openLogDirectory()).rejects.toThrow("desktop runtime");
  });
});
