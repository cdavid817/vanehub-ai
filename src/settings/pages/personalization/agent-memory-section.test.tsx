// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { SettingsProvider } from "../../settings-provider";
import { renderWithAppProviders } from "../../../test/render";
import { AgentMemorySection } from "./agent-memory-section";

function renderSection() {
  return renderWithAppProviders(
    <SettingsProvider>
      <AgentMemorySection />
    </SettingsProvider>,
  );
}

describe("AgentMemorySection", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("reads no memory at all to render the policy toggles", async () => {
    renderSection();

    await screen.findByRole("switch", { name: "启用记忆" });

    // The panel used to load every memory in full to draw a list beside these two switches, so
    // opening the page cost whatever the user had ever saved. The list is its own panel now.
    expect(screen.queryByTestId("personalization-memory-list")).toBeNull();
    expect(screen.queryByTestId("personalization-memory-filters")).toBeNull();
  });

  it("offers both toggles with their stored values", async () => {
    renderSection();

    const master = await screen.findByRole("switch", { name: "启用记忆" });
    const sub = screen.getByRole("switch", { name: "从工具辅助的会话中记忆" });

    expect(master.getAttribute("aria-checked")).toBe("true");
    expect(sub.getAttribute("aria-checked")).toBe("true");
  });

  it("disables the tool-assisted toggle when memory is off", async () => {
    renderSection();

    const master = await screen.findByRole("switch", { name: "启用记忆" });
    await userEvent.click(master);

    await waitFor(() => {
      expect(master.getAttribute("aria-checked")).toBe("false");
    });
    // A sub-toggle that stayed usable would suggest extraction still happens with memory off.
    expect(screen.getByRole("switch", { name: "从工具辅助的会话中记忆" }).hasAttribute("disabled")).toBe(true);
  });
});
