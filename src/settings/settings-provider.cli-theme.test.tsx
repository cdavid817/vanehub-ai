// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { settingsService } from "../services/runtime-settings-client";
import { defaultAppSettings } from "../services/settings-service";
import { SettingsProvider, useSettings } from "./settings-provider";

vi.mock("../services/runtime-settings-client", () => ({
  settingsService: {
    getSettings: vi.fn(),
    getNodeInfo: vi.fn(),
    reportClientLogEvent: vi.fn(),
    saveSetting: vi.fn(),
    setLaunchOnStartup: vi.fn(),
    subscribeSettingsEvents: vi.fn(),
  },
}));

function Surface() {
  const { error, resetSettings, saveSetting, settings } = useSettings();
  return (
    <div data-testid="surface" data-cli={settings.cliTerminalTheme} data-error={error ?? ""}>
      <button type="button" onClick={() => void saveSetting("cliTerminalTheme", "light").catch(() => undefined)}>cli-light</button>
      <button type="button" onClick={() => void saveSetting("theme", "futuristic").catch(() => undefined)}>app-futuristic</button>
      <button type="button" onClick={() => void resetSettings().catch(() => undefined)}>reset</button>
    </div>
  );
}

describe("SettingsProvider CLI terminal theme", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    delete document.documentElement.dataset.cliTerminalTheme;
    delete document.documentElement.dataset.theme;
    vi.mocked(settingsService.getNodeInfo).mockResolvedValue({ available: false, path: null, version: null, reason: null });
    vi.mocked(settingsService.subscribeSettingsEvents).mockResolvedValue(() => undefined);
    vi.mocked(settingsService.reportClientLogEvent).mockResolvedValue(undefined);
  });

  it("applies the loaded value as a root attribute separate from the app theme", async () => {
    vi.mocked(settingsService.getSettings).mockResolvedValue({ ...defaultAppSettings, cliTerminalTheme: "light", theme: "minimal" });
    render(<SettingsProvider><Surface /></SettingsProvider>);
    await screen.findByTestId("surface");
    expect(document.documentElement.dataset.cliTerminalTheme).toBe("light");
    expect(document.documentElement.dataset.theme).toBe("minimal");
  });

  it("changing the app theme leaves the CLI theme alone, and vice versa", async () => {
    vi.mocked(settingsService.getSettings).mockResolvedValue({ ...defaultAppSettings, cliTerminalTheme: "dark", theme: "minimal" });
    vi.mocked(settingsService.saveSetting).mockImplementation(async (input) => ({ ...defaultAppSettings, theme: "minimal", cliTerminalTheme: "dark", [input.key]: input.value }));
    render(<SettingsProvider><Surface /></SettingsProvider>);
    await screen.findByTestId("surface");

    fireEvent.click(screen.getByRole("button", { name: "app-futuristic" }));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("futuristic"));
    expect(document.documentElement.dataset.cliTerminalTheme).toBe("dark");
    expect(vi.mocked(settingsService.saveSetting).mock.calls.map(([input]) => input.key)).toEqual(["theme"]);

    vi.mocked(settingsService.saveSetting).mockImplementation(async (input) => ({ ...defaultAppSettings, theme: "futuristic", [input.key]: input.value }));
    fireEvent.click(screen.getByRole("button", { name: "cli-light" }));
    await waitFor(() => expect(document.documentElement.dataset.cliTerminalTheme).toBe("light"));
    expect(document.documentElement.dataset.theme).toBe("futuristic");
  });

  it("rolls the attribute and the context value back when the save is rejected", async () => {
    vi.mocked(settingsService.getSettings).mockResolvedValue({ ...defaultAppSettings, cliTerminalTheme: "dark" });
    vi.mocked(settingsService.saveSetting).mockRejectedValue(new Error("storage refused"));
    render(<SettingsProvider><Surface /></SettingsProvider>);
    await screen.findByTestId("surface");

    fireEvent.click(screen.getByRole("button", { name: "cli-light" }));
    await waitFor(() => expect(screen.getByTestId("surface").dataset.error).toBe("storage refused"));
    expect(document.documentElement.dataset.cliTerminalTheme).toBe("dark");
    expect(screen.getByTestId("surface").dataset.cli).toBe("dark");
  });

  it("returns to dark on reset without touching unrelated keys", async () => {
    vi.mocked(settingsService.getSettings).mockResolvedValue({ ...defaultAppSettings, cliTerminalTheme: "light" });
    vi.mocked(settingsService.saveSetting).mockImplementation(async (input) => ({ ...defaultAppSettings, [input.key]: input.value }));
    render(<SettingsProvider><Surface /></SettingsProvider>);
    await screen.findByTestId("surface");
    expect(document.documentElement.dataset.cliTerminalTheme).toBe("light");

    fireEvent.click(screen.getByRole("button", { name: "reset" }));
    await waitFor(() => expect(document.documentElement.dataset.cliTerminalTheme).toBe("dark"));
    const keys = vi.mocked(settingsService.saveSetting).mock.calls.map(([input]) => input.key);
    expect(keys).toContain("cliTerminalTheme");
    expect(keys).not.toContain("customInstructionsAboutUser");
    expect(keys).not.toContain("memoryEnabled");
  });
});
