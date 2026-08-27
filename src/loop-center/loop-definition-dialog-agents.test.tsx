// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { agentService } from "../services/runtime-agent-client";
import type { AgentRegistryEntry, AvailabilityState } from "../types/agent";
import { LoopDefinitionDialog } from "./loop-definition-dialog";

function registryEntry(id: string, displayName: string, availabilityState: AvailabilityState): AgentRegistryEntry {
  return {
    id,
    displayName,
    provider: "test",
    launch: { kind: "cli", command: id },
    supportedInteractionModes: ["cli"],
    availabilityState,
    capabilityTags: [],
    agentOrigin: "builtin",
  };
}

/** Renders the wizard, fills a valid scope step, and advances to the agents step. */
async function openAgentsStep(agents: AgentRegistryEntry[]) {
  vi.spyOn(agentService, "listAgents").mockResolvedValue(agents);
  vi.spyOn(agentService, "listLoopProjectChoices").mockResolvedValue([
    { path: "/repo", displayName: "repo", available: true, simulated: true },
  ]);
  vi.spyOn(agentService, "listLoopBranches").mockResolvedValue([
    { name: "main", kind: "local", available: true, simulated: true },
  ]);
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={client}><LoopDefinitionDialog definition={null} onClose={() => undefined} onSaved={() => undefined} /></QueryClientProvider>);
  await userEvent.type(screen.getByLabelText("名称"), "loop");
  await userEvent.type(screen.getByLabelText("目标"), "goal");
  await userEvent.type(screen.getByLabelText("验收标准（每行一项）"), "criterion");
  await waitFor(() => expect((screen.getByLabelText("项目路径") as HTMLSelectElement).value).toBe("/repo"));
  await userEvent.click(screen.getByRole("button", { name: "下一步" }));
  await screen.findByLabelText("执行智能体");
}

describe("LoopDefinitionDialog agents step", () => {
  afterEach(() => vi.restoreAllMocks());

  it("preselects the first available agents instead of the registry order", async () => {
    await openAgentsStep([
      registryEntry("ghost-cli", "Ghost CLI", "unavailable"),
      registryEntry("codex-cli", "Codex CLI", "available"),
      registryEntry("claude-code", "Claude Code", "available"),
    ]);
    expect((screen.getByLabelText("执行智能体") as HTMLSelectElement).value).toBe("codex-cli");
    expect((screen.getByLabelText("验证智能体") as HTMLSelectElement).value).toBe("claude-code");
  });

  it("labels agents the backend would refuse, matching the project and branch selects", async () => {
    await openAgentsStep([
      registryEntry("ghost-cli", "Ghost CLI", "unavailable"),
      registryEntry("locked-cli", "Locked CLI", "needs-auth"),
      registryEntry("codex-cli", "Codex CLI", "available"),
    ]);
    const options = [...(screen.getByLabelText("执行智能体") as HTMLSelectElement).options].map((option) => option.textContent);
    expect(options).toContain("Ghost CLI — 不可用");
    expect(options).toContain("Locked CLI — 需要登录");
    expect(options).toContain("Codex CLI");
  });

  it("maps a save-time agent refusal to the agents step with a localised message", async () => {
    vi.spyOn(agentService, "createLoopDefinition").mockRejectedValue(
      new Error("agent is unavailable: Command 'agy' was not found on PATH."),
    );
    await openAgentsStep([registryEntry("ghost-cli", "Ghost CLI", "unavailable")]);
    await userEvent.click(screen.getByRole("button", { name: "下一步" }));
    await screen.findByLabelText("验证程序");
    await userEvent.click(screen.getByRole("button", { name: "下一步" }));
    await userEvent.click(await screen.findByRole("button", { name: "保存" }));
    // Back on the agents step, with the backend detail wrapped in the localised explanation.
    expect(await screen.findByLabelText("执行智能体")).toBeTruthy();
    expect(screen.getByText(/所选智能体当前不可用/).textContent).toContain("agy");
  });

  it("clears a stale step error once the input it described changes", async () => {
    vi.spyOn(agentService, "listAgents").mockResolvedValue([]);
    vi.spyOn(agentService, "listLoopProjectChoices").mockResolvedValue([]);
    vi.spyOn(agentService, "listLoopBranches").mockResolvedValue([]);
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(<QueryClientProvider client={client}><LoopDefinitionDialog definition={null} onClose={() => undefined} onSaved={() => undefined} /></QueryClientProvider>);
    await userEvent.click(screen.getByRole("button", { name: "下一步" }));
    expect(screen.getByText("名称、项目路径、基础分支和目标均为必填项。")).toBeTruthy();
    await userEvent.type(screen.getByLabelText("名称"), "loop");
    expect(screen.queryByText("名称、项目路径、基础分支和目标均为必填项。")).toBeNull();
  });

  it("falls back to the registry order when nothing is available", async () => {
    await openAgentsStep([
      registryEntry("ghost-cli", "Ghost CLI", "unavailable"),
      registryEntry("phantom-cli", "Phantom CLI", "unknown"),
    ]);
    expect((screen.getByLabelText("执行智能体") as HTMLSelectElement).value).toBe("ghost-cli");
    expect((screen.getByLabelText("验证智能体") as HTMLSelectElement).value).toBe("phantom-cli");
  });
});
