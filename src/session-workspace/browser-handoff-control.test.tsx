// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../test/render";
import { BrowserHandoffControl } from "./browser-handoff-control";

describe("BrowserHandoffControl", () => {
  beforeEach(async () => activateAppLanguage("zh-CN"));

  it("shows explicit paused and resumed states around human control", async () => {
    const beginBrowserHandoff = vi.fn(async () => ({
      operationId: "browser-1",
      state: "human_control" as const,
      ownershipToken: "owner-1",
      updatedAt: "2026-08-14T00:00:00Z",
    }));
    const resumeBrowserAutomation = vi.fn(async () => ({
      operationId: "browser-1",
      state: "resuming" as const,
      ownershipToken: "owner-1",
      updatedAt: "2026-08-14T00:01:00Z",
    }));
    const service = createAgentServiceDouble({ beginBrowserHandoff, resumeBrowserAutomation });
    const { user } = renderWithAppProviders(<BrowserHandoffControl operationId="browser-1" operationStatus="running" service={service} />);

    await user.click(screen.getByRole("button", { name: "接管浏览器" }));
    expect(await screen.findByText("你控制浏览器期间，自动化已暂停。")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "恢复自动化" }));
    await waitFor(() => expect(resumeBrowserAutomation).toHaveBeenCalledWith("browser-1", "owner-1"));
    expect(await screen.findByText("人工控制已结束，正在恢复自动化。")).toBeTruthy();
  });

  it("loads an existing handoff when the operation awaits a human", async () => {
    const getBrowserHandoff = vi.fn(async () => ({
      operationId: "browser-2",
      state: "awaiting_human" as const,
      ownershipToken: "owner-2",
      updatedAt: "2026-08-14T00:00:00Z",
    }));
    const service = createAgentServiceDouble({ getBrowserHandoff });
    renderWithAppProviders(<BrowserHandoffControl operationId="browser-2" operationStatus="awaiting_human" service={service} />);

    expect(await screen.findByText("你控制浏览器期间，自动化已暂停。")).toBeTruthy();
    expect(getBrowserHandoff).toHaveBeenCalledWith("browser-2");
  });
});
