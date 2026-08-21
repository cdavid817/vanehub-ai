// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { getCliConfigPresets } from "../../../config/cli-agent-provider-presets";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { CliConfigProfile } from "../../../types/cli-agent-config";
import { AgentGlobalConfigPanel } from "./agent-global-config-panel";

const status = { agentId: "claude-code" as const, appliedProfileId: null, driftState: "detached" as const, resolvedPaths: [], lastAppliedAt: null, simulated: true, startupSync: { agentId: "claude-code" as const, state: "unavailable" as const, imported: 0, updated: 0, skipped: 0, warnings: [], synchronizedAt: null, simulated: true } };

function profileFixture(): CliConfigProfile {
  const preset = getCliConfigPresets("claude-code")[0]!;
  return {
    id: "anthropic",
    agentId: "claude-code",
    name: "Anthropic",
    payloadVersion: 1,
    payload: preset.payload,
    sourcePresetId: preset.id,
    sourcePresetVersion: 1,
    credentialConfigured: false,
    validationState: "valid",
    appliedState: "saved",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };
}

describe("AgentGlobalConfigPanel", () => {
  it("verifies a saved CLI credential without exposing it to the page", async () => {
    const profile = { ...profileFixture(), credentialConfigured: true };
    const validateCliConfigCredential = vi.fn(async () => ({ status: "valid" as const, latencyMs: 14, httpStatus: 200 }));
    const service = createAgentServiceDouble({
      listCliConfigPresets: async () => [],
      listCliConfigProfiles: async () => [profile],
      getCliConfigStatus: async () => status,
      validateCliConfigCredential,
    });
    const { user } = renderWithAppProviders(<AgentGlobalConfigPanel agentId="claude-code" service={service} />);

    await user.click(await screen.findByRole("button", { name: "验证 API 密钥" }));
    await waitFor(() => expect(validateCliConfigCredential).toHaveBeenCalledWith({ agentId: "claude-code", profileId: profile.id }));
    expect(await screen.findByText("API 密钥有效。")).toBeTruthy();
    expect(document.body.textContent).not.toContain("sk-");
  });

  it("creates an editable preset draft, restores focus, and does not apply it", async () => {
    const preset = getCliConfigPresets("claude-code")[0]!;
    const save = vi.fn(async (input) => ({ ...profileFixture(), name: input.name, payload: input.payload }));
    const apply = vi.fn();
    const service = createAgentServiceDouble({
      listCliConfigPresets: async () => [preset],
      listCliConfigProfiles: async () => [],
      getCliConfigStatus: async () => status,
      saveCliConfigProfile: save,
    });
    const { user } = renderWithAppProviders(<AgentGlobalConfigPanel agentId="claude-code" service={service} />);

    const addButton = await screen.findByRole("button", { name: "新增配置" });
    expect(screen.queryByText("Anthropic · Claude Code")).toBeNull();
    await user.click(addButton);
    const presetButton = within(screen.getByRole("dialog")).getByRole("button", { name: /Anthropic/ });
    await user.click(presetButton);
    expect(screen.getByRole("dialog")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "继续" }));
    await user.click(screen.getByRole("button", { name: "保存修改" }));

    await waitFor(() => expect(save).toHaveBeenCalledOnce());
    expect(apply).not.toHaveBeenCalled();
    expect(screen.getByText("配置已保存，全局 CLI 文件未修改。")).toBeTruthy();
    expect(document.activeElement).toBe(addButton);
  });

  it("confirms an apply in an application dialog without selecting a runtime Agent", async () => {
    const profile = profileFixture();
    const apply = vi.fn(async () => ({ operationId: "operation", status: "succeeded" as const, agentId: "claude-code" as const, profileId: profile.id, affectedPaths: [], driftResolution: null, backfilledProfileId: null, warnings: [], restartRequired: true, simulated: true, restored: true, error: null }));
    const listProfiles = vi.fn(async () => [profile]);
    const selectAgent = vi.fn();
    const confirm = vi.spyOn(window, "confirm");
    const service = createAgentServiceDouble({
      listCliConfigPresets: async () => [],
      listCliConfigProfiles: listProfiles,
      getCliConfigStatus: async () => status,
      applyCliConfigProfile: apply,
      selectAgent,
    });
    const { user } = renderWithAppProviders(<AgentGlobalConfigPanel agentId="claude-code" service={service} />);

    await user.click(await screen.findByRole("button", { name: "全局应用" }));
    const dialog = screen.getByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "全局应用" }));
    await waitFor(() => expect(apply).toHaveBeenCalledOnce());
    await waitFor(() => expect(listProfiles.mock.calls.length).toBeGreaterThan(1));
    expect(selectAgent).not.toHaveBeenCalled();
    expect(confirm).not.toHaveBeenCalled();
    confirm.mockRestore();
  });

  it("shows a rollback failure returned by the service boundary", async () => {
    const profile = profileFixture();
    const service = createAgentServiceDouble({
      listCliConfigPresets: async () => [],
      listCliConfigProfiles: async () => [profile],
      getCliConfigStatus: async () => ({ ...status, simulated: false, resolvedPaths: ["~/.claude/settings.json"] }),
      applyCliConfigProfile: async () => { throw new Error("configuration rollback was incomplete"); },
    });
    const { user } = renderWithAppProviders(<AgentGlobalConfigPanel agentId="claude-code" service={service} />);

    await user.click(await screen.findByRole("button", { name: "全局应用" }));
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "全局应用" }));
    expect((await screen.findByRole("alert")).textContent).toContain("configuration rollback was incomplete");
  });

  it("filters providers by category and confirms delete without a browser prompt", async () => {
    const [official, common] = getCliConfigPresets("claude-code");
    const profile = profileFixture();
    const remove = vi.fn(async () => undefined);
    const confirm = vi.spyOn(window, "confirm");
    const service = createAgentServiceDouble({
      listCliConfigPresets: async () => [official!, common!],
      listCliConfigProfiles: async () => [profile],
      getCliConfigStatus: async () => status,
      deleteCliConfigProfile: remove,
    });
    const { user } = renderWithAppProviders(<AgentGlobalConfigPanel agentId="claude-code" service={service} />);

    await screen.findByText("Anthropic");
    await user.click(screen.getByRole("button", { name: "新增配置" }));
    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText("Anthropic · Claude Code")).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "常用" }));
    expect(within(dialog).queryByText("Anthropic · Claude Code")).toBeNull();
    expect(within(dialog).getByText("OpenRouter · Claude Code")).toBeTruthy();
    await user.keyboard("{Escape}");
    await user.click(screen.getByRole("button", { name: /更多配置操作: Anthropic/ }));
    await user.click(screen.getByRole("menuitem", { name: "删除配置" }));
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "删除配置" }));
    await waitFor(() => expect(remove).toHaveBeenCalledOnce());
    expect(confirm).not.toHaveBeenCalled();
    confirm.mockRestore();
  });

  it("imports the current configuration through a named application dialog", async () => {
    const imported = vi.fn(async () => profileFixture());
    const prompt = vi.spyOn(window, "prompt");
    const service = createAgentServiceDouble({
      listCliConfigPresets: async () => [],
      listCliConfigProfiles: async () => [],
      getCliConfigStatus: async () => status,
      importCliConfigProfile: imported,
    });
    const { user } = renderWithAppProviders(<AgentGlobalConfigPanel agentId="claude-code" service={service} />);

    await user.click(await screen.findByRole("button", { name: "导入当前配置" }));
    const dialog = screen.getByRole("dialog");
    await user.clear(within(dialog).getByRole("textbox", { name: "配置名称" }));
    await user.type(within(dialog).getByRole("textbox", { name: "配置名称" }), "Imported live profile");
    await user.click(within(dialog).getByRole("button", { name: "导入配置" }));
    await waitFor(() => expect(imported).toHaveBeenCalledWith({ agentId: "claude-code", name: "Imported live profile" }));
    expect(prompt).not.toHaveBeenCalled();
    prompt.mockRestore();
  });

  it("shows the startup synchronization outcome without a candidate-selection prompt", async () => {
    const service = createAgentServiceDouble({
      listCliConfigPresets: async () => [],
      listCliConfigProfiles: async () => [],
      getCliConfigStatus: async () => ({
        ...status,
        agentId: "opencode" as const,
        startupSync: { agentId: "opencode" as const, state: "updated" as const, imported: 1, updated: 2, skipped: 0, warnings: [], synchronizedAt: "2026-08-02T00:00:00Z", simulated: false },
      }),
    });
    renderWithAppProviders(<AgentGlobalConfigPanel agentId="opencode" service={service} />);

    expect(await screen.findByText("启动同步已导入 1 个并更新 2 个 OpenCode 配置。")).toBeTruthy();
    expect(screen.queryByText("发现本地 CLI 配置")).toBeNull();
  });

  it("renders a safe startup synchronization warning", async () => {
    const service = createAgentServiceDouble({
      listCliConfigPresets: async () => [],
      listCliConfigProfiles: async () => [],
      getCliConfigStatus: async () => ({
        ...status,
        simulated: false,
        startupSync: { agentId: "claude-code" as const, state: "warning" as const, imported: 0, updated: 0, skipped: 1, warnings: ["The local CLI configuration could not be parsed; startup synchronization was skipped."], synchronizedAt: "2026-08-02T00:00:00Z", simulated: false },
      }),
    });
    renderWithAppProviders(<AgentGlobalConfigPanel agentId="claude-code" service={service} />);

    expect(await screen.findByText("启动同步已完成，但存在警告。")).toBeTruthy();
    expect(screen.getByText("The local CLI configuration could not be parsed; startup synchronization was skipped.")).toBeTruthy();
  });

  it("searches profile metadata, emphasizes the applied profile, and keeps edit focused on the form", async () => {
    const profile = { ...profileFixture(), appliedState: "applied" as const };
    const service = createAgentServiceDouble({
      listCliConfigPresets: async () => getCliConfigPresets("claude-code"),
      listCliConfigProfiles: async () => [profile],
      getCliConfigStatus: async () => ({ ...status, appliedProfileId: profile.id, driftState: "applied" }),
    });
    const { user } = renderWithAppProviders(<AgentGlobalConfigPanel agentId="claude-code" service={service} />);

    expect(await screen.findByText("当前已应用")).toBeTruthy();
    const search = screen.getByRole("textbox", { name: "搜索已保存配置" });
    await user.type(search, "api.anthropic.com");
    expect(screen.getByText("Anthropic")).toBeTruthy();
    await user.clear(search);
    await user.type(search, "missing provider");
    expect(screen.getByText("没有符合搜索条件的已保存配置。")).toBeTruthy();
    await user.clear(search);
    await user.click(screen.getByRole("button", { name: /更多配置操作: Anthropic/ }));
    await user.click(screen.getByRole("menuitem", { name: "编辑配置" }));
    expect(screen.getByRole("dialog")).toBeTruthy();
    expect(screen.queryByText("选择厂商预设")).toBeNull();
  });
});
