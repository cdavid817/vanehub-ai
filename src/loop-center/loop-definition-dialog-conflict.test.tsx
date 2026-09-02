// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { loopQueryKeys } from "../hooks/loop-query";
import { i18n } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { loopDefinitionFixture } from "../test/loop-fixtures";
import type { LoopDefinition } from "../types/loop";
import { isLoopVersionConflict, LoopDefinitionDialog } from "./loop-definition-dialog";

function mockDiscovery() {
  vi.spyOn(agentService, "listAgents").mockResolvedValue([]);
  vi.spyOn(agentService, "listLoopProjectChoices").mockResolvedValue([]);
  vi.spyOn(agentService, "listLoopBranches").mockResolvedValue([]);
}

/** Opens the wizard already editing `definition` and advances straight to the Review step -- the
 *  fixture is valid on every step, so nothing needs filling in first (mirrors
 *  loop-definition-dialog-agents.test.tsx's own `openAgentsStep` precedent one step further). */
async function openReviewStep(definition: LoopDefinition, client: QueryClient) {
  mockDiscovery();
  render(<QueryClientProvider client={client}><LoopDefinitionDialog definition={definition} onClose={() => undefined} onSaved={() => undefined} /></QueryClientProvider>);
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "下一步" }));
  await screen.findByLabelText("执行智能体");
  await user.click(screen.getByRole("button", { name: "下一步" }));
  await screen.findByLabelText("验证程序");
  await user.click(screen.getByRole("button", { name: "下一步" }));
  await screen.findByRole("button", { name: "保存" });
  return user;
}

describe("isLoopVersionConflict", () => {
  it("matches the Web mock's own localized message", () => {
    expect(isLoopVersionConflict(new Error(i18n.t("loops.web.error.versionConflict")))).toBe(true);
  });

  it("matches Tauri's fixed English validation prose, independent of UI locale", () => {
    expect(isLoopVersionConflict(new Error("validation error: Loop definition was updated by another operation."))).toBe(true);
  });

  it("does not match an unrelated error", () => {
    expect(isLoopVersionConflict(new Error("agent is unavailable: nope"))).toBe(false);
    expect(isLoopVersionConflict(new Error("validation error: Loop definition not found."))).toBe(false);
  });
});

describe("LoopDefinitionDialog version conflict", () => {
  afterEach(() => vi.restoreAllMocks());

  it("refreshes canonical state, explains the conflict, and retries with the refreshed version", async () => {
    const definition = loopDefinitionFixture();
    const refreshed = { ...definition, version: 2, name: "Fixture Loop (renamed elsewhere)" };
    const update = vi.spyOn(agentService, "updateLoopDefinition")
      .mockRejectedValueOnce(new Error(i18n.t("loops.web.error.versionConflict")))
      .mockResolvedValueOnce(refreshed);
    const list = vi.spyOn(agentService, "listLoopDefinitions").mockResolvedValue([refreshed]);
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const user = await openReviewStep(definition, client);

    await user.click(screen.getByRole("button", { name: "保存" }));
    expect(await screen.findByText(/在其他地方被修改/)).toBeTruthy();
    expect(list).toHaveBeenCalledTimes(1);
    // Every other view of this definition (the overview behind this modal, the navigation list)
    // must also see the refreshed row, not just this dialog's own private state.
    expect(client.getQueryData(loopQueryKeys.definitions)).toEqual([refreshed]);

    await user.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(update).toHaveBeenCalledTimes(2));
    expect(update).toHaveBeenLastCalledWith(definition.id, expect.objectContaining({ expectedVersion: 2 }));
  });

  it("explains that the definition was deleted elsewhere when the refetch no longer finds it", async () => {
    const definition = loopDefinitionFixture();
    vi.spyOn(agentService, "updateLoopDefinition").mockRejectedValue(new Error(i18n.t("loops.web.error.versionConflict")));
    vi.spyOn(agentService, "listLoopDefinitions").mockResolvedValue([]);
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const user = await openReviewStep(definition, client);

    await user.click(screen.getByRole("button", { name: "保存" }));
    expect(await screen.findByText(/已在其他地方被删除/)).toBeTruthy();
  });

  it("still shows the unrelated agent-unavailable explanation for a non-conflict failure", async () => {
    const definition = loopDefinitionFixture();
    vi.spyOn(agentService, "updateLoopDefinition").mockRejectedValue(
      new Error("agent is unavailable: Command 'agy' was not found on PATH."),
    );
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const user = await openReviewStep(definition, client);

    await user.click(screen.getByRole("button", { name: "保存" }));
    // Back on the agents step, exactly like the existing non-conflict regression in
    // loop-definition-dialog-agents.test.tsx -- this task must not change that path.
    expect(await screen.findByLabelText("执行智能体")).toBeTruthy();
    expect(screen.getByText(/所选智能体当前不可用/).textContent).toContain("agy");
  });
});
