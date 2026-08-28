// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { settingsService } from "../services/runtime-settings-client";
import { defaultAppSettings } from "../services/settings-service";
import { loadLocaleResource } from "../i18n/supported-locales";
import type { AppSettings } from "../types/settings";
import { SettingsProvider, useSettings } from "./settings-provider";

vi.mock("../services/runtime-settings-client", () => ({
  settingsService: {
    getSettings: vi.fn(),
    getNodeInfo: vi.fn(),
    reportClientLogEvent: vi.fn(),
    saveSetting: vi.fn(),
    subscribeSettingsEvents: vi.fn(),
  },
}));

interface HydrationSnapshot {
  contextFontSize: AppSettings["fontSize"];
  documentFontSize: string;
  error: string | null;
  language: string;
  theme: string | undefined;
}

function HydratedSurface({ onRender }: { onRender: (snapshot: HydrationSnapshot) => void }) {
  const { error, saveSetting, settings } = useSettings();
  onRender({
    contextFontSize: settings.fontSize,
    documentFontSize: document.documentElement.style.fontSize,
    error,
    language: i18n.language,
    theme: document.documentElement.dataset.theme,
  });
  return (
    <div data-testid="hydrated-surface">
      ready
      <button type="button" onClick={() => void saveSetting("applicationLanguage", "ko")}>switch</button>
    </div>
  );
}

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, reject, resolve };
}

describe("SettingsProvider hydration", () => {
  beforeEach(async () => {
    vi.resetAllMocks();
    vi.mocked(settingsService.getNodeInfo).mockResolvedValue({
      available: false,
      path: null,
      reason: "not available",
      version: null,
    });
    vi.mocked(settingsService.subscribeSettingsEvents).mockResolvedValue(() => undefined);
    vi.mocked(settingsService.reportClientLogEvent).mockResolvedValue(undefined);
    document.documentElement.style.removeProperty("font-size");
    delete document.documentElement.dataset.theme;
    await activateAppLanguage("zh-CN");
  });

  afterEach(() => {
    document.documentElement.style.removeProperty("font-size");
    delete document.documentElement.dataset.theme;
  });

  it("does not render children until persisted settings are applied", async () => {
    const pendingSettings = deferred<AppSettings>();
    const onRender = vi.fn<(snapshot: HydrationSnapshot) => void>();
    vi.mocked(settingsService.getSettings).mockReturnValue(pendingSettings.promise);

    render(
      <SettingsProvider>
        <HydratedSurface onRender={onRender} />
      </SettingsProvider>,
    );

    expect(screen.queryByTestId("hydrated-surface")).toBeNull();
    expect(onRender).not.toHaveBeenCalled();

    pendingSettings.resolve({
      ...defaultAppSettings,
      applicationLanguage: "en",
      fontSize: "12px",
      theme: "minimal",
    });

    await screen.findByTestId("hydrated-surface");
    await waitFor(() => expect(onRender).toHaveBeenCalledTimes(1));
    expect(onRender).toHaveBeenLastCalledWith({
      contextFontSize: "12px",
      documentFontSize: "12px",
      error: null,
      language: "en",
      theme: "minimal",
    });
  });

  it("applies defaults and exposes the load error before rendering children", async () => {
    const onRender = vi.fn<(snapshot: HydrationSnapshot) => void>();
    vi.mocked(settingsService.getSettings).mockRejectedValue(new Error("settings unavailable"));

    render(
      <SettingsProvider>
        <HydratedSurface onRender={onRender} />
      </SettingsProvider>,
    );

    await screen.findByTestId("hydrated-surface");
    await waitFor(() => expect(onRender).toHaveBeenCalledTimes(1));
    expect(onRender).toHaveBeenLastCalledWith({
      contextFontSize: defaultAppSettings.fontSize,
      documentFontSize: defaultAppSettings.fontSize,
      error: "无法加载应用设置，已恢复默认设置。",
      language: defaultAppSettings.applicationLanguage,
      theme: defaultAppSettings.theme,
    });
    expect(settingsService.reportClientLogEvent).toHaveBeenCalledWith({
      level: "error",
      kind: "critical-operation-failure",
      message: "settings unavailable",
      source: "SettingsProvider.loadSettings",
    });
  });

  it("waits for an optional locale resource before rendering children", async () => {
    const pendingLocale = deferred<void>();
    const onRender = vi.fn<(snapshot: HydrationSnapshot) => void>();
    vi.mocked(settingsService.getSettings).mockResolvedValue({
      ...defaultAppSettings,
      applicationLanguage: "ja",
    });
    const activateLanguage = vi.fn(async (language: AppSettings["applicationLanguage"]) => {
      if (language === "ja") await pendingLocale.promise;
      await activateAppLanguage(language);
    });

    render(
      <SettingsProvider activateLanguage={activateLanguage}>
        <HydratedSurface onRender={onRender} />
      </SettingsProvider>,
    );

    await waitFor(() => expect(activateLanguage).toHaveBeenCalledWith("ja"));
    expect(screen.queryByTestId("hydrated-surface")).toBeNull();
    pendingLocale.resolve();

    await screen.findByTestId("hydrated-surface");
    expect(i18n.language).toBe("ja");
    expect(document.documentElement.lang).toBe("ja");
  });

  it("falls back to localized defaults when an optional locale resource fails", async () => {
    const onRender = vi.fn<(snapshot: HydrationSnapshot) => void>();
    vi.mocked(settingsService.getSettings).mockResolvedValue({
      ...defaultAppSettings,
      applicationLanguage: "ja",
    });
    i18n.removeResourceBundle("ja", "translation");
    const activateLanguage = vi.fn((language: AppSettings["applicationLanguage"]) => activateAppLanguage(
      language,
      async (requestedLanguage) => {
        if (requestedLanguage === "ja") throw new Error("optional chunk unavailable");
        return loadLocaleResource(requestedLanguage);
      },
    ));

    render(
      <SettingsProvider activateLanguage={activateLanguage}>
        <HydratedSurface onRender={onRender} />
      </SettingsProvider>,
    );

    await screen.findByTestId("hydrated-surface");
    await waitFor(() => expect(onRender).toHaveBeenCalled());
    expect(i18n.language).toBe(defaultAppSettings.applicationLanguage);
    expect(onRender.mock.lastCall?.[0].error).toContain("ja");
    expect(onRender.mock.lastCall?.[0].error).not.toContain("optional chunk unavailable");
  });

  it("saves against the revision the screen was rendered from, not a fresher one", async () => {
    // The whole point of the compatibility window's concurrency check. Sending a revision re-read
    // at save time would accept every write and silently revert another screen's edit.
    const onRender = vi.fn<(snapshot: HydrationSnapshot) => void>();
    vi.mocked(settingsService.getSettings).mockResolvedValue({ ...defaultAppSettings, personalizationRevision: 9 });
    vi.mocked(settingsService.saveSetting).mockResolvedValue({
      ...defaultAppSettings,
      applicationLanguage: "ko",
      personalizationRevision: 10,
    });

    render(
      <SettingsProvider>
        <HydratedSurface onRender={onRender} />
      </SettingsProvider>,
    );

    await screen.findByTestId("hydrated-surface");
    fireEvent.click(screen.getByRole("button", { name: "switch" }));

    await waitFor(() => expect(settingsService.saveSetting).toHaveBeenCalled());
    expect(settingsService.saveSetting).toHaveBeenCalledWith({
      key: "applicationLanguage",
      value: "ko",
      expectedPersonalizationRevision: 9,
    });
  });

  it("switches immediately and keeps the language returned by persistence", async () => {
    const onRender = vi.fn<(snapshot: HydrationSnapshot) => void>();
    vi.mocked(settingsService.getSettings).mockResolvedValue(defaultAppSettings);
    vi.mocked(settingsService.saveSetting).mockResolvedValue({
      ...defaultAppSettings,
      applicationLanguage: "ko",
    });

    render(
      <SettingsProvider>
        <HydratedSurface onRender={onRender} />
      </SettingsProvider>,
    );

    await screen.findByTestId("hydrated-surface");
    fireEvent.click(screen.getByRole("button", { name: "switch" }));

    await waitFor(() => expect(i18n.language).toBe("ko"));
    expect(document.documentElement.lang).toBe("ko");
    // The revision travels on every save, not only on personalization keys: the provider has no
    // per-key knowledge, and the native side ignores it for keys the policy does not own.
    expect(settingsService.saveSetting).toHaveBeenCalledWith({
      key: "applicationLanguage",
      value: "ko",
      expectedPersonalizationRevision: 0,
    });
  });
});
