import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../../i18n";
import type { CliEnvironmentSnapshot } from "../../types/cli-environment-snapshot";
import type { OperationTask } from "../../types/operation";
import { ProvidersPage, refreshButtonState } from "./providers-page";
import { bulkUpgradeEligible, canRequestChange, targetVersionOptions } from "./cli-action-selection";

const npmSource = {
  sourceId: "npm",
  kind: "npm",
  supportedOnThisPlatform: true,
  availableVersionCount: 2,
  availableVersions: ["1.3.0", "1.2.0"],
  capabilities: {
    install: "exact",
    upgrade: "exact",
    downgrade: "exact",
    reinstall: "exact",
    uninstall: true,
    repair: "unsupported",
  },
};

const snapshot: CliEnvironmentSnapshot = {
  schemaVersion: 1,
  agentId: "claude-code",
  displayName: "Anthropic Claude Code CLI",
  provider: "Anthropic",
  executableNames: ["claude"],
  scope: "local-desktop",
  overallState: "update-available",
  freshness: "fresh",
  environmentFingerprint: "fingerprint-a",
  installations: [{
    id: "claude",
    executablePath: "/mock/bin/claude",
    canonicalPath: null,
    aliasPaths: [],
    targetMissing: false,
    reportedVersion: "1.2.0",
    sourceId: "npm",
    sourceKind: "npm",
    sourceConfidence: "inferred",
    pathPriority: 0,
    environmentOrigin: "path",
    executableStatus: "healthy",
  }],
  pathSelectedInstallationId: "claude",
  recommendedInstallationId: "claude",
  discovery: "found-one",
  executable: "healthy",
  authentication: "unknown",
  readiness: "unknown",
  compatibility: "unknown",
  update: "available",
  conflicts: [],
  sources: [npmSource],
  allowedActions: [
    { action: "upgrade", sourceId: "npm", targetMode: "exact", defaultTarget: "1.3.0", requiresTargetSelection: false, reasonCode: null },
  ],
  lastMutation: null,
  lastOperationId: "op-1",
  checkedAt: "2026-01-01T00:00:00+00:00",
};

const operation: OperationTask = {
  id: "op-1",
  kind: "cli",
  status: "succeeded",
  relatedEntityId: "claude-code",
  message: "Installed",
  logs: [{ operationId: "op-1", line: "npm install complete", timestamp: "123" }],
  result: null,
  error: null,
  createdAt: "123",
  updatedAt: "124",
};

function renderPage(snapshots: CliEnvironmentSnapshot[], operations: OperationTask[] = []) {
  const queryClient = new QueryClient();
  queryClient.setQueryData(["cli-environments"], snapshots);
  for (const task of operations) queryClient.setQueryData(["operation", task.id], task);
  return renderToString(
    <QueryClientProvider client={queryClient}>
      <ProvidersPage searchTerm="" />
    </QueryClientProvider>,
  );
}

describe("ProvidersPage CLI management rendering", () => {
  it("sorts service-backed cards with the shared settings priority", () => {
    const snapshots = [
      ["gemini-cli", "Google Gemini CLI"],
      ["antigravity-cli", "Antigravity CLI"],
      ["opencode", "OpenCode CLI"],
      ["codex-cli", "OpenAI Codex CLI"],
      ["claude-code", "Anthropic Claude Code CLI"],
    ].map(([agentId, displayName]) => ({ ...snapshot, agentId, displayName }));

    const html = renderPage(snapshots);
    const positions = snapshots.map((item) => html.indexOf(`data-cli-agent="${item.agentId}"`));

    expect(positions).toEqual([...positions].sort((left, right) => right - left));
  });

  it("renders cached snapshot data and card-local operation state", () => {
    const html = renderPage([snapshot], [operation]);

    expect(html).toContain("CLI 管理");
    expect(html).toContain("本地环境检查");
    expect(html).toContain('data-testid="cli-installation-summary"');
    expect(html).toContain("诊断安装冲突");
    expect(html).toContain("全部升级 1");
    expect(html).toContain("Anthropic Claude Code CLI");
    expect(html).toContain("1.2.0");
    expect(html).toContain("最近操作");
    expect(html).toContain("已成功");
    expect(html).toContain("当前生效路径");
    // The backend's own update state, not a comparison this page made.
    expect(html).toContain("有可用更新");
  });

  it("derives refresh button loading state from mutation or operation status", () => {
    expect(refreshButtonState(true, undefined)).toMatchObject({
      disabled: true,
      labelKey: "cli.refreshing",
    });
    expect(refreshButtonState(false, { ...operation, status: "queued" })).toMatchObject({
      disabled: true,
      labelKey: "cli.refreshing",
    });
    expect(refreshButtonState(false, { ...operation, status: "running" }).iconClassName).toContain("animate-spin");
    expect(refreshButtonState(false, { ...operation, status: "failed" })).toMatchObject({
      disabled: false,
      labelKey: "cli.refresh",
    });
  });

  it("offers no version change while a conflict blocks mutation", () => {
    const conflicted: CliEnvironmentSnapshot = {
      ...snapshot,
      overallState: "conflict",
      conflicts: [{
        kind: "path-shadowing",
        severity: "blocking",
        installationIds: ["claude"],
        blocksMutation: true,
        blocksLaunch: false,
        reasonCode: "path-shadowing",
      }],
    };

    const html = renderPage([conflicted]);

    expect(html).not.toContain("更改版本");
    // The conflict is explained by its code, not by a parsed message.
    expect(html).toContain("遮蔽");
    // And it is excluded from the batch rather than silently dropped from the count.
    expect(html).toContain("全部升级 0");
  });

  it("shows the current-version marker instead of a button that would do nothing", () => {
    const current: CliEnvironmentSnapshot = {
      ...snapshot,
      installations: [{ ...snapshot.installations[0], reportedVersion: "1.3.0" }],
      update: "up-to-date",
    };

    const html = renderPage([current]);

    expect(html).toContain("已是当前版本");
    expect(html).not.toContain("更改版本");
  });

  it("takes its target list from the source that owns the recommended installation", () => {
    // One source's catalog, never a merge. Borrowing another source's list is the defect removed.
    expect(targetVersionOptions(snapshot)).toEqual(["1.3.0", "1.2.0"]);
    expect(targetVersionOptions({ ...snapshot, sources: [] })).toEqual([]);
  });

  it("reads changeability off the backend's action list rather than comparing versions", () => {
    expect(canRequestChange(snapshot, "1.3.0")).toBe(true);
    // Equal to what is installed: no change on offer, and the comparison is a string equality on
    // the backend's own reported version, not a semantic version ordering.
    expect(canRequestChange(snapshot, "1.2.0")).toBe(false);
    // The backend offered nothing, so neither does the page.
    expect(canRequestChange({ ...snapshot, allowedActions: [] }, "1.3.0")).toBe(false);
  });

  it("enables bulk upgrade only for tools the backend offers an upgrade for", () => {
    expect(bulkUpgradeEligible(snapshot)).toBe(true);
    expect(bulkUpgradeEligible({ ...snapshot, allowedActions: [] })).toBe(false);
    expect(bulkUpgradeEligible({
      ...snapshot,
      conflicts: [{
        kind: "version-divergence",
        severity: "blocking",
        installationIds: ["claude"],
        blocksMutation: true,
        blocksLaunch: false,
        reasonCode: "version-divergence",
      }],
    })).toBe(false);
  });
});
