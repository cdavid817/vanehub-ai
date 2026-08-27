// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { LspServerTestResult, LspWorkspaceTrust } from "../../../types/lsp";
import { lspTestConfiguration } from "../../../test/lsp-fixtures";
import { LspServerTestPanel } from "./lsp-server-test-panel";
import { LspWorkspaceTrustPanel } from "./lsp-workspace-trust-panel";

const successfulTest: LspServerTestResult = {
  server: "rust_analyzer",
  phases: ["discovery", "spawn", "initialize", "cleanup"].map((phase) => ({
    phase: phase as "discovery" | "spawn" | "initialize" | "cleanup",
    status: "succeeded" as const,
    reasonCode: null,
  })),
  negotiatedCapabilities: null,
};

describe("LSP trust and server-test panels", () => {
  beforeEach(async () => activateAppLanguage("zh-CN"));

  it("loads trusted workspaces through AgentService and explains the OS permission boundary", async () => {
    const listLspWorkspaceTrust = vi.fn(async (): Promise<LspWorkspaceTrust[]> => [{
      canonicalRoot: "D:/code/trusted-project",
      trusted: true,
      revision: 2,
    }]);
    const service = createAgentServiceDouble({ listLspWorkspaceTrust });

    renderWithAppProviders(<LspWorkspaceTrustPanel service={service} />);

    expect(screen.getByText(/使用你的操作系统权限运行/)).toBeTruthy();
    expect(screen.getByText(/它不是操作系统沙箱/)).toBeTruthy();
    expect(await screen.findByText("D:/code/trusted-project")).toBeTruthy();
    const rootInput = screen.getByRole("textbox", { name: "本地工作区绝对路径" });
    const descriptionId = rootInput.getAttribute("aria-describedby");
    expect(descriptionId).toBe("lsp-trust-boundary");
    expect(document.getElementById(descriptionId ?? "")?.textContent).toContain("不是操作系统沙箱");
    expect(screen.getByRole("button", { name: /撤销信任.*D:\/code\/trusted-project/ })).toBeTruthy();
    expect(listLspWorkspaceTrust).toHaveBeenCalledOnce();
  });

  it("grants workspace trust using only the keyboard", async () => {
    const updateLspWorkspaceTrust = vi.fn(async ({ canonicalRoot, trusted }: {
      canonicalRoot: string;
      trusted: boolean;
    }): Promise<LspWorkspaceTrust> => ({ canonicalRoot, trusted, revision: 1 }));
    const service = createAgentServiceDouble({
      listLspWorkspaceTrust: async () => [],
      updateLspWorkspaceTrust,
    });
    const { user } = renderWithAppProviders(<LspWorkspaceTrustPanel service={service} />);

    await user.tab();
    const rootInput = screen.getByRole("textbox", { name: "本地工作区绝对路径" });
    expect(document.activeElement).toBe(rootInput);
    await user.keyboard("D:/code/keyboard-project");
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "信任工作区" }));
    await user.keyboard("[Enter]");

    await waitFor(() => expect(updateLspWorkspaceTrust).toHaveBeenCalledWith({
      canonicalRoot: "D:/code/keyboard-project",
      trusted: true,
    }));
  });

  it("grants and revokes workspace trust, refreshing the service-backed list", async () => {
    let records: LspWorkspaceTrust[] = [];
    const listLspWorkspaceTrust = vi.fn(async () => records);
    const updateLspWorkspaceTrust = vi.fn(async ({ canonicalRoot, trusted }: {
      canonicalRoot: string;
      trusted: boolean;
    }): Promise<LspWorkspaceTrust> => {
      const record = { canonicalRoot, trusted, revision: 1 };
      records = trusted ? [record] : [];
      return record;
    });
    const service = createAgentServiceDouble({ listLspWorkspaceTrust, updateLspWorkspaceTrust });
    const { user } = renderWithAppProviders(<LspWorkspaceTrustPanel service={service} />);

    await user.type(await screen.findByRole("textbox", { name: "本地工作区绝对路径" }), "D:/code/new-project");
    await user.click(screen.getByRole("button", { name: "信任工作区" }));

    await waitFor(() => expect(updateLspWorkspaceTrust).toHaveBeenCalledWith({
      canonicalRoot: "D:/code/new-project",
      trusted: true,
    }));
    expect(await screen.findByText("D:/code/new-project")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: /撤销信任.*D:\/code\/new-project/ }));
    await waitFor(() => expect(updateLspWorkspaceTrust).toHaveBeenLastCalledWith({
      canonicalRoot: "D:/code/new-project",
      trusted: false,
    }));
    expect(await screen.findByText("尚未信任任何工作区使用 LSP。")).toBeTruthy();
    expect(listLspWorkspaceTrust.mock.calls.length).toBeGreaterThanOrEqual(3);
  });

  it("shows a safe trust loading error and retries the service query", async () => {
    const listLspWorkspaceTrust = vi.fn()
      .mockRejectedValueOnce(new Error("D:/private/secret"))
      .mockResolvedValueOnce([]);
    const service = createAgentServiceDouble({ listLspWorkspaceTrust });
    const { user } = renderWithAppProviders(<LspWorkspaceTrustPanel service={service} />);

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("无法加载语言服务器设置。");
    expect(alert.textContent).not.toContain("private");
    await user.click(screen.getByRole("button", { name: "重试" }));

    expect(await screen.findByText("尚未信任任何工作区使用 LSP。")).toBeTruthy();
    expect(listLspWorkspaceTrust).toHaveBeenCalledTimes(2);
  });

  it("shows successful isolated server-test phases", async () => {
    const testLspServer = vi.fn(async () => successfulTest);
    const service = createAgentServiceDouble({
      getLspConfiguration: async () => lspTestConfiguration(),
      testLspServer,
    });
    const { user } = renderWithAppProviders(<LspServerTestPanel service={service} />);

    await user.click(await screen.findByRole("button", { name: /测试服务器.*Rust/ }));

    const status = await screen.findByRole("status");
    expect(status.textContent).toContain("语言服务器测试成功。");
    expect(within(status).getAllByText("成功")).toHaveLength(4);
    expect(testLspServer).toHaveBeenCalledWith("rust");
  });

  it("hides rejected error details and provides a retry with safe phase feedback", async () => {
    const failedTest: LspServerTestResult = {
      server: "typescript_language_server",
      phases: [
        { phase: "discovery", status: "succeeded", reasonCode: null },
        { phase: "spawn", status: "failed", reasonCode: "spawn_failed" },
        { phase: "initialize", status: "skipped", reasonCode: null },
        { phase: "cleanup", status: "skipped", reasonCode: null },
      ],
      negotiatedCapabilities: null,
    };
    const testLspServer = vi.fn()
      .mockRejectedValueOnce(new Error("C:/private/token"))
      .mockResolvedValueOnce(failedTest);
    const service = createAgentServiceDouble({
      getLspConfiguration: async () => lspTestConfiguration(),
      testLspServer,
    });
    const { user } = renderWithAppProviders(<LspServerTestPanel service={service} />);

    await user.click(await screen.findByRole("button", { name: /测试服务器.*TypeScript/ }));
    const transportAlert = await screen.findByRole("alert");
    expect(transportAlert.textContent).toContain("语言服务器测试未成功完成。");
    expect(transportAlert.textContent).not.toContain("private");

    await user.click(screen.getByRole("button", { name: /重试.*TypeScript/ }));
    const phaseAlert = await screen.findByRole("alert");
    expect(phaseAlert.textContent).toContain("无法启动服务器进程。");
    expect(testLspServer).toHaveBeenCalledTimes(2);
  });
});
