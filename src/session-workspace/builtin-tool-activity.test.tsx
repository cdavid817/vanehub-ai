// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../test/render";
import type { BuiltinToolOperation, BuiltinToolOperationEvent } from "../types/builtin-tools";
import { BuiltinToolActivity } from "./builtin-tool-activity";

const running: BuiltinToolOperation = {
  id: "operation-1",
  agentId: "onepiece",
  sessionId: "session-1",
  capability: "web",
  operation: "search",
  status: "running",
  progress: { phase: "fetching", completedUnits: 2, totalUnits: 5, messageCode: null },
  artifactIds: [],
  errorCode: null,
  simulated: false,
  createdAt: "2026-08-14T00:00:00Z",
  updatedAt: "2026-08-14T00:00:01Z",
};

describe("BuiltinToolActivity", () => {
  beforeEach(async () => activateAppLanguage("zh-CN"));

  it("shows bounded progress, consumes events, and cancels through AgentService", async () => {
    let operationListener: ((event: BuiltinToolOperationEvent) => void) | undefined;
    const cancelBuiltinToolOperation = vi.fn(async () => ({ ...running, status: "cancelled" as const }));
    const listBuiltinToolOperations = vi.fn(async () => [running]);
    const service = createAgentServiceDouble({
      cancelBuiltinToolOperation,
      listBuiltinToolOperations,
      subscribeBuiltinToolOperations: async (_sessionId, listener) => {
        operationListener = listener;
        return () => undefined;
      },
    });
    const { user } = renderWithAppProviders(<BuiltinToolActivity service={service} sessionId="session-1" />);

    expect(await screen.findByText("Web 搜索与抓取")).toBeTruthy();
    expect(screen.getByText("2/5")).toBeTruthy();
    expect(listBuiltinToolOperations).toHaveBeenCalledWith({ sessionId: "session-1", limit: 50 });
    await user.click(screen.getByRole("button", { name: "取消 search" }));
    expect(cancelBuiltinToolOperation).toHaveBeenCalledWith("operation-1");

    operationListener?.({ kind: "snapshot", operation: { ...running, status: "succeeded" } });
    await waitFor(() => expect(screen.getByText("已成功")).toBeTruthy());
    expect(screen.queryByRole("button", { name: "取消 search" })).toBeNull();
  });

  it("hides the surface when no operations exist", async () => {
    const service = createAgentServiceDouble({
      listBuiltinToolOperations: async () => [],
      subscribeBuiltinToolOperations: async () => () => undefined,
    });
    renderWithAppProviders(<BuiltinToolActivity service={service} sessionId="session-1" />);

    await waitFor(() => expect(screen.queryByText("内置工具活动")).toBeNull());
  });
});
