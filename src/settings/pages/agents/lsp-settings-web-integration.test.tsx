// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import { webAgentClient } from "../../../services/web-agent-client";
import { resetWebLspMockStateForTest } from "../../../services/web-lsp-client";
import { renderWithAppProviders } from "../../../test/render";
import type { LspConfiguration } from "../../../types/lsp";
import { LspConfigurationSection } from "./lsp-configuration-section";
import { LspRuntimeStatusPanel } from "./lsp-runtime-status-panel";
import { LspServerTestPanel } from "./lsp-server-test-panel";
import { LspWorkspaceTrustPanel } from "./lsp-workspace-trust-panel";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

const enabledRustConfiguration: LspConfiguration = {
  enabled: true,
  languages: [
    {
      language: "rust",
      enabled: true,
      executableOverride: null,
      initializationOptions: {},
    },
    {
      language: "typescript_javascript",
      enabled: false,
      executableOverride: null,
      initializationOptions: {},
    },
  ],
};

describe("LSP settings Web integration", () => {
  beforeEach(async () => {
    invokeMock.mockClear();
    resetWebLspMockStateForTest();
    await activateAppLanguage("zh-CN");
  });

  afterEach(() => {
    expect(invokeMock).not.toHaveBeenCalled();
    vi.restoreAllMocks();
  });

  it("persists configuration through the Web service across UI remounts", async () => {
    const firstRender = renderWithAppProviders(
      <LspConfigurationSection service={webAgentClient} />,
    );

    await firstRender.user.click(await screen.findByRole("checkbox", {
      name: /^启用 LSP 集成/,
    }));
    await firstRender.user.click(screen.getByRole("checkbox", {
      name: "启用 Rust 语言服务器",
    }));
    await firstRender.user.click(screen.getByRole("button", { name: "保存 LSP 配置" }));
    expect((await screen.findByRole("status")).textContent).toContain("LSP 配置已保存。");

    firstRender.unmount();
    renderWithAppProviders(<LspConfigurationSection service={webAgentClient} />);

    expect((await screen.findByRole("checkbox", {
      name: /^启用 LSP 集成/,
    }) as HTMLInputElement).checked).toBe(true);
    expect((screen.getByRole("checkbox", {
      name: "启用 Rust 语言服务器",
    }) as HTMLInputElement).checked).toBe(true);
  });

  it("refreshes deterministic status as workspace trust is granted and revoked", async () => {
    await webAgentClient.saveLspConfiguration(enabledRustConfiguration);
    const { user } = renderWithAppProviders(
      <>
        <LspWorkspaceTrustPanel service={webAgentClient} />
        <LspRuntimeStatusPanel service={webAgentClient} />
      </>,
    );

    expect(await screen.findByText("当前没有活动的语言服务器实例。")).toBeTruthy();
    await user.type(screen.getByRole("textbox", {
      name: "本地工作区绝对路径",
    }), "D:/code/web-integration");
    await user.click(screen.getByRole("button", { name: "信任工作区" }));

    const rustStatus = await screen.findByRole("article", { name: "Rust rust_analyzer" });
    expect(await within(rustStatus).findByText("正在启动")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "刷新状态" }));
    expect(await within(rustStatus).findByText("正在初始化")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "刷新状态" }));
    expect(await within(rustStatus).findByText("就绪")).toBeTruthy();

    await user.click(screen.getByRole("button", {
      name: "撤销信任 D:/code/web-integration",
    }));
    expect(await screen.findByText("当前没有活动的语言服务器实例。")).toBeTruthy();
    expect(await screen.findByText("尚未信任任何工作区使用 LSP。")).toBeTruthy();
  });

  it("repeats isolated Web server tests with the same deterministic result", async () => {
    const testServer = vi.spyOn(webAgentClient, "testLspServer");
    const { user } = renderWithAppProviders(
      <LspServerTestPanel service={webAgentClient} />,
    );
    const rustTestButton = screen.getByRole("button", { name: /测试服务器.*Rust/ });

    await user.click(rustTestButton);
    const firstStatus = await screen.findByRole("status");
    const firstResult = firstStatus.textContent;
    expect(within(firstStatus).getAllByText("成功")).toHaveLength(4);

    await user.click(rustTestButton);
    await waitFor(() => expect(testServer).toHaveBeenCalledTimes(2));
    const secondStatus = screen.getByRole("status");
    expect(secondStatus.textContent).toBe(firstResult);
    expect(within(secondStatus).getAllByText("成功")).toHaveLength(4);
  });
});
