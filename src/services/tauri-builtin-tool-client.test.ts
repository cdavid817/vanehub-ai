import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));

import { tauriBuiltinToolClient } from "./tauri-builtin-tool-client";

describe("Tauri built-in tool adapter", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({});
    listenMock.mockReset();
    listenMock.mockResolvedValue(() => undefined);
  });

  it("maps shared contracts to thin Tauri command calls", async () => {
    const applyInput = {
      agentId: "onepiece",
      sessionId: "session-1",
      artifactId: "artifact-1",
      expectedContentHash: "sha256:content",
      expectedDiffHash: "sha256:diff",
      repositoryIdentity: "repo-1",
      baseCommit: "abc",
      acknowledgement: true as const,
    };

    await tauriBuiltinToolClient.getBuiltinToolReadiness("onepiece");
    await tauriBuiltinToolClient.cancelBuiltinToolOperation("operation-1");
    await tauriBuiltinToolClient.listArtifacts({ sessionId: "session-1", limit: 10 });
    await tauriBuiltinToolClient.applyDelegationChanges(applyInput);
    await tauriBuiltinToolClient.resumeBrowserAutomation("operation-1", "owner-1");

    expect(invokeMock.mock.calls).toEqual([
      ["get_builtin_tool_readiness", { agentId: "onepiece" }],
      ["cancel_builtin_tool_operation", { operationId: "operation-1" }],
      ["list_artifacts", { input: { sessionId: "session-1", limit: 10 } }],
      ["apply_delegation_changes", { input: applyInput }],
      ["resume_browser_automation", { operationId: "operation-1", ownershipToken: "owner-1" }],
    ]);
  });

  it("filters operation events by session", async () => {
    const listener = vi.fn();
    await tauriBuiltinToolClient.subscribeBuiltinToolOperations("session-1", listener);
    const callback = listenMock.mock.calls[0]?.[1] as (event: { payload: unknown }) => void;
    const operation = { sessionId: "session-1" };

    callback({ payload: { kind: "snapshot", operation } });
    callback({ payload: { kind: "snapshot", operation: { sessionId: "session-2" } } });
    callback({ payload: { kind: "removed", operationId: "operation-1" } });

    expect(listener).toHaveBeenCalledTimes(2);
  });

  it("maps every query and effect contract to its native command", async () => {
    const artifactRead = { artifactId: "artifact-1", offset: 0, length: 32 };
    const artifactEffect = {
      artifactId: "artifact-1",
      expectedContentHash: "sha256:content",
      acknowledgement: true as const,
    };
    const delegation = {
      agentId: "onepiece",
      sessionId: "session-1",
      provider: "codex_cli" as const,
      mode: "analyze" as const,
      prompt: "Analyze",
      artifactIds: [],
    };

    await tauriBuiltinToolClient.getBuiltinToolOperation("operation-1");
    await tauriBuiltinToolClient.listBuiltinToolOperations({ sessionId: "session-1" });
    await tauriBuiltinToolClient.getArtifact("artifact-1");
    await tauriBuiltinToolClient.readArtifact(artifactRead);
    await tauriBuiltinToolClient.publishArtifact(artifactEffect);
    await tauriBuiltinToolClient.downloadArtifact(artifactEffect);
    await tauriBuiltinToolClient.startDelegation(delegation);
    await tauriBuiltinToolClient.listDelegationAttempts("session-1");
    await tauriBuiltinToolClient.getDelegationReport("attempt-1");
    await tauriBuiltinToolClient.getChangeSetReview("artifact-1");
    await tauriBuiltinToolClient.getDelegationRecovery("operation-1");
    await tauriBuiltinToolClient.getBrowserHandoff("operation-1");
    await tauriBuiltinToolClient.beginBrowserHandoff("operation-1");

    expect(invokeMock.mock.calls.map(([command]) => command)).toEqual([
      "get_builtin_tool_operation",
      "list_builtin_tool_operations",
      "get_artifact",
      "read_artifact",
      "publish_artifact",
      "download_artifact",
      "start_delegation",
      "list_delegation_attempts",
      "get_delegation_report",
      "get_change_set_review",
      "get_delegation_recovery",
      "get_browser_handoff",
      "begin_browser_handoff",
    ]);
  });
});
