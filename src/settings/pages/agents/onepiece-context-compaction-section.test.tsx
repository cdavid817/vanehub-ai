// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { webSettingsClient } from "../../../services/web-settings-client";
import { renderWithAppProviders } from "../../../test/render";
import { SettingsProvider } from "../../settings-provider";
import { OnePieceContextCompactionSection } from "./onepiece-context-compaction-section";

function renderSection() {
  return renderWithAppProviders(
    <SettingsProvider>
      <OnePieceContextCompactionSection />
    </SettingsProvider>,
  );
}

describe("OnePieceContextCompactionSection", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("shows the default enabled preference and persists a disabled choice", async () => {
    const { user } = renderSection();
    const toggle = await screen.findByRole("switch", { name: "自动上下文压缩" });

    expect(toggle.getAttribute("aria-checked")).toBe("true");
    await user.click(toggle);

    await waitFor(() => expect(toggle.getAttribute("aria-checked")).toBe("false"));
    expect(JSON.parse(window.localStorage.getItem("vanehub.appSettings") ?? "{}"))
      .toMatchObject({ automaticContextCompactionEnabled: false });
  });

  it("restores a disabled preference and describes subsequent-generation scope", async () => {
    window.localStorage.setItem(
      "vanehub.appSettings",
      JSON.stringify({ automaticContextCompactionEnabled: false }),
    );

    renderSection();

    expect((await screen.findByRole("switch", { name: "自动上下文压缩" }))
      .getAttribute("aria-checked")).toBe("false");
    expect(screen.getByText(/仅应用于后续 OnePiece 生成/)).toBeTruthy();
  });

  it("disables the control and shows saving feedback while persistence is pending", async () => {
    let resolveSave: ((value: Awaited<ReturnType<typeof webSettingsClient.saveSetting>>) => void) | undefined;
    vi.spyOn(webSettingsClient, "saveSetting").mockImplementation(() => new Promise((resolve) => {
      resolveSave = resolve;
    }));
    const { user } = renderSection();
    const toggle = await screen.findByRole("switch", { name: "自动上下文压缩" });

    await user.click(toggle);

    expect(toggle).toHaveProperty("disabled", true);
    expect(screen.getByRole("status").textContent).toContain("正在保存压缩设置");
    resolveSave?.({ ...(await webSettingsClient.getSettings()), automaticContextCompactionEnabled: false });
    await waitFor(() => expect(toggle).toHaveProperty("disabled", false));
  });

  it("restores the previous value and displays a save failure", async () => {
    vi.spyOn(webSettingsClient, "saveSetting").mockRejectedValueOnce(new Error("storage unavailable"));
    const { user } = renderSection();
    const toggle = await screen.findByRole("switch", { name: "自动上下文压缩" });

    await user.click(toggle);

    expect((await screen.findByRole("alert")).textContent).toContain("storage unavailable");
    expect(toggle.getAttribute("aria-checked")).toBe("true");
  });
});
