import { beforeEach, describe, expect, it, vi } from "vitest";
import { defaultAppSettings } from "./settings-service";

const { invoke, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import { tauriSettingsClient } from "./tauri-settings-client";

describe("tauri-settings-client", () => {
  beforeEach(() => {
    invoke.mockReset();
    listen.mockReset();
  });

  it.each(["zh-CN", "en", "zh-TW", "ja", "ko"] as const)(
    "preserves the %s locale through the native adapter contract",
    async (applicationLanguage) => {
      invoke.mockResolvedValue({ ...defaultAppSettings, applicationLanguage });

      await expect(
        tauriSettingsClient.saveSetting({ key: "applicationLanguage", value: applicationLanguage }),
      ).resolves.toMatchObject({ applicationLanguage });
      expect(invoke).toHaveBeenCalledWith("save_setting", {
        input: { key: "applicationLanguage", value: applicationLanguage },
      });
    },
  );
});
