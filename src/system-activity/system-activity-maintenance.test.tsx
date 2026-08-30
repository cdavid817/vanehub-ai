// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import type { SystemActivitySession } from "../services/system-activity-service";
import { renderWithAppProviders } from "../test/render";
import { SystemActivityControls } from "./system-activity-controls";
import { SystemActivityHealthPanel } from "./system-activity-health-panel";

const session: SystemActivitySession = {
  sessionId: "system-activity:workspace:workspace-one",
  kind: "system-activity",
  scopeKind: "workspace",
  canonicalScopeId: "workspace-one",
  safeDisplayIdentity: "Workspace One",
  activeGenerationId: "generation-one",
  lastSequence: 4,
  unreadCount: 0,
  attentionKind: "none",
  firstActivityAtMs: 1,
  lastActivityAtMs: 2,
  visible: true,
};

function deferred<T>() {
  let complete: (value: T) => void = () => undefined;
  const promise = new Promise<T>((resolve) => {
    complete = resolve;
  });
  return { promise, complete };
}

afterEach(() => vi.restoreAllMocks());

describe("system activity maintenance", () => {
  it("renders per-domain gaps, failures, cursors, and rebuild history", async () => {
    await activateAppLanguage("zh-CN");
    renderWithAppProviders(
      <SystemActivityHealthPanel
        health={{
          leaseOwner: "projector-one",
          lastCompletedAtMs: 1_787_000_000_000,
          domains: [
            {
              sourceDomain: "generation",
              opaqueCursor: null,
              lastSequence: 42,
              lastSourceHash: null,
              retentionFloor: null,
              pendingCount: 3,
              oldestPendingAtMs: 1_786_999_000_000,
              gap: "source_sequence_gap",
              failureCode: "unsupported_envelope",
              revision: 7,
            },
          ],
          rebuilds: [
            {
              rebuildId: "rebuild-one",
              scopeKind: "workspace",
              canonicalScopeId: "workspace-one",
              status: "validating",
              processedItems: 80,
              itemBudget: 100,
            },
          ],
        }}
        language="zh-CN"
      />,
    );

    expect(screen.getByText("generation")).toBeTruthy();
    expect(screen.getByText("待处理 3")).toBeTruthy();
    expect(screen.getByText("来源游标序号 42")).toBeTruthy();
    expect(screen.getByText(/source_sequence_gap/)).toBeTruthy();
    expect(screen.getByText(/validating · 80\/100/)).toBeTruthy();
  });

  it("shows rebuild progress and cancels through the service boundary", async () => {
    await activateAppLanguage("zh-CN");
    const advance = deferred<{ step: "validating"; processedItems: number }>();
    vi.spyOn(agentService, "getSystemActivityPreferences").mockResolvedValue(null);
    vi.spyOn(agentService, "beginSystemActivityRebuild").mockResolvedValue({
      rebuildId: "rebuild-one",
      scopeKind: "workspace",
      canonicalScopeId: "workspace-one",
      shadowGenerationId: "generation-shadow",
      sourceSnapshotHash: "snapshot",
      status: "running",
      processedItems: 0,
      itemBudget: 100,
      revision: 1,
    });
    vi.spyOn(agentService, "advanceSystemActivityRebuild").mockReturnValue(advance.promise);
    const cancel = vi.spyOn(agentService, "cancelSystemActivityRebuild").mockResolvedValue();
    renderWithAppProviders(<SystemActivityControls onChanged={() => undefined} session={session} />);

    await userEvent.click(screen.getByTestId("system-activity-rebuild"));
    expect(await screen.findByTestId("system-activity-rebuild-progress")).toBeTruthy();
    expect(screen.getByText("正在构建影子代次")).toBeTruthy();
    await userEvent.click(screen.getByTestId("system-activity-rebuild-cancel"));

    await waitFor(() => expect(cancel).toHaveBeenCalledWith("rebuild-one"));
    expect(await screen.findByText("重建已取消,原投影保持可用。")).toBeTruthy();
    advance.complete({ step: "validating", processedItems: 10 });
  });
});
