// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { i18n } from "../i18n";
import { settingsService } from "../services/runtime-settings-client";
import { defaultAppSettings } from "../services/settings-service";
import type { AppSettings } from "../types/settings";
import { SettingsProvider, useSettings } from "./settings-provider";

vi.mock("../services/runtime-settings-client", () => ({
  settingsService: {
    getSettings: vi.fn(),
    getNodeInfo: vi.fn(),
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
  const { error, settings } = useSettings();
  onRender({
    contextFontSize: settings.fontSize,
    documentFontSize: document.documentElement.style.fontSize,
    error,
    language: i18n.language,
    theme: document.documentElement.dataset.theme,
  });
  return <div data-testid="hydrated-surface">ready</div>;
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
    document.documentElement.style.removeProperty("font-size");
    delete document.documentElement.dataset.theme;
    await i18n.changeLanguage("zh-CN");
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
      error: "settings unavailable",
      language: defaultAppSettings.applicationLanguage,
      theme: defaultAppSettings.theme,
    });
  });
});
