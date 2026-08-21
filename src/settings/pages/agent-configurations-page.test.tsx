// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { getCliConfigPresets } from "../../config/cli-agent-provider-presets";
import { getOnePieceProviderPresets } from "../../config/onepiece-provider-presets";
import { createAgentServiceDouble, renderWithAppProviders } from "../../test/render";
import type { CliConfigAgentId } from "../../types/cli-agent-config";
import { AgentConfigurationsPage } from "./agent-configurations-page";

describe("AgentConfigurationsPage", () => {
  it("honors the originating Agent and isolates data when the grouped selector changes", async () => {
    const listProfiles = vi.fn(async () => []);
    const listPresets = vi.fn(async (agentId: CliConfigAgentId) => getCliConfigPresets(agentId));
    const getStatus = vi.fn(async (agentId: CliConfigAgentId) => ({ agentId, appliedProfileId: null, driftState: "detached" as const, resolvedPaths: [], lastAppliedAt: null, simulated: true, startupSync: { agentId, state: "unavailable" as const, imported: 0, updated: 0, skipped: 0, warnings: [], synchronizedAt: null, simulated: true } }));
    const selectAgent = vi.fn();
    const service = createAgentServiceDouble({
      listCliConfigProfiles: listProfiles,
      listCliConfigPresets: listPresets,
      getCliConfigStatus: getStatus,
      selectAgent,
    });
    const { user } = renderWithAppProviders(<AgentConfigurationsPage isActive navigationTarget={{ cliConfigAgentId: "codex-cli" }} onNavigate={vi.fn()} searchTerm="" service={service} />);

    await waitFor(() => expect(getStatus).toHaveBeenCalledWith("codex-cli"));
    const selector = screen.getByRole("navigation", { name: "配置目标 Agent" });
    expect(within(selector).getAllByRole("button")).toEqual([
      screen.getByRole("button", { name: "Claude Code" }),
      screen.getByRole("button", { name: "Codex CLI" }),
      screen.getByRole("button", { name: "OpenCode" }),
      screen.getByRole("button", { name: "Antigravity CLI" }),
      screen.getByRole("button", { name: "Gemini CLI" }),
      screen.getByRole("button", { name: "OnePiece" }),
    ]);
    expect(within(selector).getByText("托管 CLI Agent")).toBeTruthy();
    expect(within(selector).getByText("原生 Agent")).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "语言服务器智能" })).toBeNull();
    expect(screen.getByRole("button", { name: "Codex CLI" }).getAttribute("aria-current")).toBe("page");
    await user.click(screen.getByRole("button", { name: "OpenCode" }));
    await waitFor(() => expect(getStatus).toHaveBeenCalledWith("opencode"));
    expect(screen.getByRole("button", { name: "OpenCode" }).getAttribute("aria-current")).toBe("page");
    expect(selectAgent).not.toHaveBeenCalled();
  });

  it("keeps OnePiece provider configuration separate from registered-Agent management", async () => {
    const listOnePieceProviderProfiles = vi.fn(async () => ({ profiles: [], activeProfileId: null }));
    const service = createAgentServiceDouble({
      listOnePieceProviderProfiles,
      listOnePieceProviderPresets: async () => getOnePieceProviderPresets(),
    });

    const { user } = renderWithAppProviders(
      <AgentConfigurationsPage
        isActive
        navigationTarget={{ agentConfigAgentId: "onepiece" }}
        onNavigate={vi.fn()}
        searchTerm=""
        service={service}
      />,
    );

    await waitFor(() => expect(listOnePieceProviderProfiles).toHaveBeenCalledOnce());
    expect(screen.getByRole("button", { name: /OnePiece/ }).getAttribute("aria-current")).toBe("page");
    const onePiecePanel = screen.getByRole("region", { name: "OnePiece" });
    expect((await within(onePiecePanel).findByRole("tab", { name: "厂商配置" })).getAttribute("aria-selected")).toBe("true");
    expect(await within(onePiecePanel).findByRole("heading", { name: "API 提供商" })).toBeTruthy();
    expect(within(onePiecePanel).getByText(/尚未添加 API 提供商/)).toBeTruthy();
    await user.click(within(onePiecePanel).getAllByRole("button", { name: "新增配置" })[0]);
    const dialog = screen.getByRole("dialog", { name: "新增 OnePiece 配置" });
    expect(within(dialog).getByRole("heading", { name: "选择 API 厂商" })).toBeTruthy();
    expect(within(dialog).getByLabelText("配置名称")).toBeTruthy();
    expect(within(dialog).getByLabelText("API 密钥")).toBeTruthy();
    expect(within(dialog).queryByLabelText("提供商")).toBeNull();
    expect(within(dialog).queryByLabelText("Base URL")).toBeNull();
    expect(within(dialog).getByRole("button", { name: "保存 OnePiece" })).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "已注册 Agent" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "注册 API Agent" })).toBeNull();
  });
});
