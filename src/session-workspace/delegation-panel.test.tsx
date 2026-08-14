// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../test/render";
import type { ArtifactDetail, DelegationAttemptSummary } from "../types/builtin-tools";
import { DelegationPanel } from "./delegation-panel";

const attempt: DelegationAttemptSummary = {
  id: "attempt-1",
  delegationId: "delegation-1",
  provider: "claude_code",
  mode: "edit",
  status: "succeeded",
  baseCommit: "base-1",
  changeSetArtifactId: "artifact-change-set",
  createdAt: "2026-08-14T00:00:00Z",
  completedAt: "2026-08-14T00:01:00Z",
};

const artifact: ArtifactDetail = {
  id: "artifact-change-set",
  displayName: "changeset.json",
  mediaType: "application/json",
  sizeBytes: 100,
  contentHash: "sha256:content",
  integrity: "verified",
  createdAt: "2026-08-14T00:01:00Z",
  expiresAt: null,
  simulated: false,
  producerOperationId: "operation-1",
  provenance: ["host-observed"],
  publishedAt: null,
  publicationUrl: null,
  limitations: [],
};

describe("DelegationPanel", () => {
  beforeEach(async () => activateAppLanguage("zh-CN"));

  it("reviews host evidence and requires exact apply acknowledgement", async () => {
    const applyDelegationChanges = vi.fn(async () => ({
      id: "apply-1",
      agentId: "onepiece",
      sessionId: "session-1",
      capability: "delegation" as const,
      operation: "apply",
      status: "running" as const,
      progress: null,
      artifactIds: [artifact.id],
      errorCode: null,
      simulated: false,
      createdAt: "2026-08-14T00:02:00Z",
      updatedAt: "2026-08-14T00:02:00Z",
    }));
    const listDelegationAttempts = vi.fn(async () => [attempt]);
    const service = createAgentServiceDouble({
      applyDelegationChanges,
      getChangeSetReview: async () => ({
        artifact,
        repositoryIdentity: "repo-1",
        baseCommit: "base-1",
        diffHash: "sha256:diff",
        files: ["src/main.ts"],
        diffText: "+safe change",
        riskClassification: "review",
        applyable: true,
      }),
      getDelegationRecovery: async () => ({ operationId: "apply-1", state: "rolled_back", capsuleReference: "capsule-1" }),
      getDelegationReport: async () => ({
        attempt,
        outcome: "succeeded",
        summary: "Host evidence captured",
        hostEvidence: ["src/main.ts changed"],
        providerClaims: ["tests passed"],
        warnings: ["provider claim not host observed"],
      }),
      listDelegationAttempts,
    });
    const { user } = renderWithAppProviders(<DelegationPanel defaultTargetRoot="D:/repo" service={service} sessionId="session-1" />);

    await waitFor(() => expect(listDelegationAttempts).toHaveBeenCalledWith("session-1"));
    await user.click(await screen.findByRole("button", { name: /claude_code/ }));
    expect(await screen.findByText("src/main.ts changed")).toBeTruthy();
    expect(await screen.findByText("+safe change")).toBeTruthy();
    const apply = screen.getByRole("button", { name: "精确应用 ChangeSet" });
    expect((apply as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByText(/我已核对上方精确仓库/));
    await user.click(apply);

    await waitFor(() => expect(applyDelegationChanges).toHaveBeenCalledWith({
      agentId: "onepiece",
      sessionId: "session-1",
      artifactId: artifact.id,
      expectedContentHash: artifact.contentHash,
      expectedDiffHash: "sha256:diff",
      repositoryIdentity: "repo-1",
      baseCommit: "base-1",
      acknowledgement: true,
    }));
    expect(await screen.findByText(/已恢复验证过的应用前状态/)).toBeTruthy();
    expect(screen.getByText(/capsule-1/)).toBeTruthy();
  });

  it("starts a bounded analyze delegation through the shared service", async () => {
    const startDelegation = vi.fn(async () => ({ ...attempt, id: "attempt-2", mode: "analyze" as const }));
    const service = createAgentServiceDouble({
      listDelegationAttempts: async () => [],
      startDelegation,
    });
    const { user } = renderWithAppProviders(<DelegationPanel defaultTargetRoot="D:/repo" service={service} sessionId="session-1" />);

    await user.type(screen.getByLabelText("有界任务提示"), "Review the repository");
    await user.click(screen.getByRole("button", { name: "开始委托" }));
    await waitFor(() => expect(startDelegation).toHaveBeenCalledWith({
      agentId: "onepiece",
      sessionId: "session-1",
      provider: "claude_code",
      mode: "analyze",
      prompt: "Review the repository",
      artifactIds: [],
    }));
  });
});
