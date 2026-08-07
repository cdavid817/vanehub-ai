// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { SettingsProvider } from "../../settings-provider";
import { CustomInstructionsSection } from "./custom-instructions-section";

describe("CustomInstructionsSection", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("saves a field's draft value on blur", async () => {
    const user = userEvent.setup();
    render(
      <SettingsProvider>
        <CustomInstructionsSection />
      </SettingsProvider>,
    );
    const field = await screen.findByLabelText("回复风格");

    await user.click(field);
    await user.type(field, "Always answer in Chinese.");
    await user.tab();

    await waitFor(() => {
      const stored = JSON.parse(window.localStorage.getItem("vanehub.appSettings") ?? "{}");
      expect(stored.customInstructionsStyleRules).toBe("Always answer in Chinese.");
    });
  });

  it("shows the character count and marks it over the limit", async () => {
    const user = userEvent.setup();
    render(
      <SettingsProvider>
        <CustomInstructionsSection />
      </SettingsProvider>,
    );
    const field = await screen.findByLabelText("关于你");
    const longValue = "x".repeat(3001);

    await user.click(field);
    await user.paste(longValue);

    expect(screen.getByText("3001 / 3000")).toBeTruthy();
  });

  it("does not persist a value typed over the character limit on blur", async () => {
    const user = userEvent.setup();
    render(
      <SettingsProvider>
        <CustomInstructionsSection />
      </SettingsProvider>,
    );
    const field = await screen.findByLabelText("关于你");

    await user.click(field);
    await user.paste("x".repeat(3001));
    await user.tab();

    const stored = JSON.parse(window.localStorage.getItem("vanehub.appSettings") ?? "{}");
    expect(stored.customInstructionsAboutUser ?? "").toBe("");
  });

  it("toggles the enabled switch", async () => {
    const user = userEvent.setup();
    render(
      <SettingsProvider>
        <CustomInstructionsSection />
      </SettingsProvider>,
    );
    const toggle = await screen.findByRole("switch", { name: "启用自定义指令" });
    expect(toggle.getAttribute("aria-checked")).toBe("true");

    await user.click(toggle);

    await waitFor(() => expect(toggle.getAttribute("aria-checked")).toBe("false"));
  });
});
