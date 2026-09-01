// @vitest-environment jsdom

/**
 * Interaction coverage for the CLI management page, against service doubles.
 *
 * These replace the string-only page tests that rendered markup and asserted on the copy inside
 * it. That style passed while the page was dispatching an install for a version the user had not
 * chosen, because the words on screen were right and the request was wrong. Every assertion here
 * is either "what reached the service" or "what a user can do next".
 *
 * The doubles stand in for the whole boundary. No Tauri, no process, no package manager, and no
 * network: the page under test is not allowed to reach any of them, and a double that only answers
 * the nine `CliToolService` methods is how that stays true.
 */

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../../../i18n";
import type { CliBulkItemResult } from "../../../types/cli-environment";
import type {
  CliActionPlan,
  CliBulkActionPlan,
  CliEnvironmentSnapshot,
  CliSourceSummary,
} from "../../../types/cli-environment-snapshot";
import type { OperationTask } from "../../../types/operation";
import type { SettingsPageStatus } from "../../settings-page-types";

const listCliEnvironments = vi.fn<() => Promise<CliEnvironmentSnapshot[]>>();
const refreshCliEnvironments = vi.fn<(ids: string[], force: boolean) => Promise<OperationTask>>();
const prepareCliAction = vi.fn<(input: unknown) => Promise<OperationTask>>();
const getCliActionPlan = vi.fn<(planId: string) => Promise<CliActionPlan>>();
const executeCliAction = vi.fn<(input: unknown) => Promise<OperationTask>>();
const prepareCliBulkUpgrade = vi.fn<(ids: string[]) => Promise<OperationTask>>();
const getCliBulkActionPlan = vi.fn<(planId: string) => Promise<CliBulkActionPlan>>();
const executeCliBulkAction = vi.fn<(input: unknown) => Promise<OperationTask>>();
const runCliDoctor = vi.fn<(agentId: string) => Promise<OperationTask>>();
const getOperationStatus = vi.fn<(id: string) => Promise<OperationTask>>();
const cancelOperation = vi.fn<(id: string) => Promise<void>>();

vi.mock("../../../services/runtime-agent-client", () => ({
  agentService: {
    listCliEnvironments: () => listCliEnvironments(),
    refreshCliEnvironments: (ids: string[], force: boolean) => refreshCliEnvironments(ids, force),
    prepareCliAction: (input: unknown) => prepareCliAction(input),
    getCliActionPlan: (planId: string) => getCliActionPlan(planId),
    executeCliAction: (input: unknown) => executeCliAction(input),
    prepareCliBulkUpgrade: (ids: string[]) => prepareCliBulkUpgrade(ids),
    getCliBulkActionPlan: (planId: string) => getCliBulkActionPlan(planId),
    executeCliBulkAction: (input: unknown) => executeCliBulkAction(input),
    runCliDoctor: (agentId: string) => runCliDoctor(agentId),
  },
}));

vi.mock("../../../services/runtime-operation-client", () => ({
  operationService: {
    getOperationStatus: (id: string) => getOperationStatus(id),
    cancelOperation: (id: string) => cancelOperation(id),
  },
}));

vi.mock("../../../services/runtime-settings-client", () => ({
  settingsService: { reportClientLogEvent: () => Promise.resolve() },
}));

function operation(overrides: Partial<OperationTask> & { id: string }): OperationTask {
  return {
    kind: "cli",
    status: "succeeded",
    relatedEntityId: null,
    message: null,
    logs: [],
    result: null,
    error: null,
    createdAt: "2026-01-01T00:00:00.000Z",
    updatedAt: "2026-01-01T00:00:00.000Z",
    ...overrides,
  };
}

function npmSource(versions: string[]): CliSourceSummary {
  return {
    sourceId: "npm",
    kind: "npm",
    supportedOnThisPlatform: true,
    availableVersionCount: versions.length,
    availableVersions: versions,
    management: "managed",
    guidanceCode: null,
    capabilities: {
      install: "exact",
      upgrade: "exact",
      downgrade: "exact",
      reinstall: "exact",
      uninstall: true,
      repair: "unsupported",
    },
  };
}

function homebrewSource(): CliSourceSummary {
  return {
    sourceId: "homebrew",
    kind: "homebrew",
    supportedOnThisPlatform: true,
    availableVersionCount: null,
    availableVersions: [],
    management: "detect-only",
    guidanceCode: "cli.guidance.homebrew",
    capabilities: {
      install: "unsupported",
      upgrade: "unsupported",
      downgrade: "unsupported",
      reinstall: "unsupported",
      uninstall: false,
      repair: "unsupported",
    },
  };
}

function snapshot(overrides: Partial<CliEnvironmentSnapshot> = {}): CliEnvironmentSnapshot {
  return {
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
      id: "claude-npm",
      executablePath: "/mock/npm/bin/claude",
      canonicalPath: "/mock/npm/lib/claude",
      aliasPaths: [],
      targetMissing: false,
      reportedVersion: "1.2.0",
      sourceId: "npm",
      sourceKind: "npm",
      sourceConfidence: "verified",
      pathPriority: 0,
      environmentOrigin: "path",
      executableStatus: "healthy",
    }],
    pathSelectedInstallationId: "claude-npm",
    recommendedInstallationId: "claude-npm",
    discovery: "found-one",
    executable: "healthy",
    authentication: "authenticated",
    readiness: "ready",
    compatibility: "supported",
    update: "available",
    conflicts: [],
    sources: [npmSource(["1.3.0", "1.2.0", "1.1.0"])],
    allowedActions: [
      { action: "upgrade", sourceId: "npm", targetMode: "exact", defaultTarget: "1.3.0", requiresTargetSelection: false, reasonCode: null },
      { action: "downgrade", sourceId: "npm", targetMode: "exact", defaultTarget: null, requiresTargetSelection: true, reasonCode: null },
    ],
    lastMutation: null,
    lastOperationId: null,
    checkedAt: "2026-01-01T00:00:00.000Z",
    ...overrides,
  };
}

/** A second tool, so "per tool" can be told apart from "everything at once". */
function codexSnapshot(overrides: Partial<CliEnvironmentSnapshot> = {}): CliEnvironmentSnapshot {
  return snapshot({
    agentId: "codex-cli",
    displayName: "OpenAI Codex CLI",
    provider: "OpenAI",
    executableNames: ["codex"],
    installations: [{
      id: "codex-npm",
      executablePath: "/mock/npm/bin/codex",
      canonicalPath: null,
      aliasPaths: [],
      targetMissing: false,
      reportedVersion: "2.0.0",
      sourceId: "npm",
      sourceKind: "npm",
      sourceConfidence: "verified",
      pathPriority: 1,
      environmentOrigin: "path",
      executableStatus: "healthy",
    }],
    pathSelectedInstallationId: "codex-npm",
    recommendedInstallationId: "codex-npm",
    sources: [npmSource(["2.1.0", "2.0.0"])],
    allowedActions: [
      { action: "upgrade", sourceId: "npm", targetMode: "exact", defaultTarget: "2.1.0", requiresTargetSelection: false, reasonCode: null },
    ],
    ...overrides,
  });
}

function plan(overrides: Partial<CliActionPlan> = {}): CliActionPlan {
  return {
    id: "plan-1",
    revision: 7,
    agentId: "claude-code",
    action: "downgrade",
    sourceId: "npm",
    installationId: "claude-npm",
    currentVersion: "1.2.0",
    targetVersion: "1.1.0",
    channel: null,
    commandPreview: { program: "npm", args: ["install", "-g", "@anthropic-ai/claude-code@1.1.0"] },
    preconditions: ["source-executable-available", "network-reachable"],
    warnings: ["downgrade-may-lose-state"],
    requiresElevation: false,
    requiresNetwork: true,
    state: "draft",
    createdAt: "2026-01-01T00:00:00.000Z",
    expiresAt: "2026-01-01T00:10:00.000Z",
    ...overrides,
  };
}

async function renderPage(
  snapshots: CliEnvironmentSnapshot[],
  onStatusChange?: (status: SettingsPageStatus | null) => void,
) {
  const { CliManagementPage } = await import("./cli-management-page");
  listCliEnvironments.mockResolvedValue(snapshots);
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const view = render(
    <QueryClientProvider client={queryClient}>
      <CliManagementPage onStatusChange={onStatusChange} searchTerm="" />
    </QueryClientProvider>,
  );
  await screen.findByText(snapshots[0].displayName);
  return view;
}

function cardFor(agentId: string): HTMLElement {
  const card = document.querySelector<HTMLElement>(`[data-cli-agent="${agentId}"]`);
  if (!card) throw new Error(`no card rendered for ${agentId}`);
  return card;
}

function changeButton(agentId: string): HTMLButtonElement {
  return within(cardFor(agentId)).getByRole("button", { name: "更改版本" }) as HTMLButtonElement;
}

function selectVersion(agentId: string, displayName: string, version: string) {
  const select = within(cardFor(agentId)).getByLabelText(`${displayName} 目标版本`);
  fireEvent.change(select, { target: { value: version } });
  return select as HTMLSelectElement;
}

beforeEach(() => {
  vi.clearAllMocks();
  refreshCliEnvironments.mockResolvedValue(operation({ id: "op-refresh", status: "running" }));
  prepareCliAction.mockResolvedValue(operation({ id: "op-prepare", status: "running" }));
  executeCliAction.mockResolvedValue(operation({ id: "op-execute", status: "running" }));
  prepareCliBulkUpgrade.mockResolvedValue(operation({ id: "op-bulk-prepare", status: "running" }));
  executeCliBulkAction.mockResolvedValue(operation({ id: "op-bulk-execute", status: "running" }));
  runCliDoctor.mockResolvedValue(operation({ id: "op-doctor", status: "running" }));
  cancelOperation.mockResolvedValue(undefined);
  getCliActionPlan.mockResolvedValue(plan());
  getOperationStatus.mockImplementation((id) =>
    Promise.resolve(operation({ id, status: "succeeded", result: { planId: "plan-1" } })));
});

describe("target selection reaches the backend unchanged", () => {
  it("sends an older selected version and names no action", async () => {
    await renderPage([snapshot()]);

    selectVersion("claude-code", "Anthropic Claude Code CLI", "1.1.0");
    fireEvent.click(within(cardFor("claude-code")).getByRole("button", { name: "更改版本" }));

    await waitFor(() => expect(prepareCliAction).toHaveBeenCalledTimes(1));
    // `action: null` on purpose: naming "downgrade" here would need a version comparison on this
    // side, and two comparisons disagree the first time a prerelease shows up.
    expect(prepareCliAction).toHaveBeenCalledWith({
      agentId: "claude-code",
      action: null,
      sourceId: "npm",
      targetVersion: "1.1.0",
      channel: null,
    });
  });

  it("sends a newer selected version", async () => {
    await renderPage([snapshot()]);

    selectVersion("claude-code", "Anthropic Claude Code CLI", "1.3.0");
    fireEvent.click(within(cardFor("claude-code")).getByRole("button", { name: "更改版本" }));

    await waitFor(() => expect(prepareCliAction).toHaveBeenCalledTimes(1));
    expect(prepareCliAction).toHaveBeenCalledWith(
      expect.objectContaining({ targetVersion: "1.3.0" }),
    );
  });

  it("offers no change and creates no operation when the target is the installed version", async () => {
    await renderPage([snapshot()]);

    selectVersion("claude-code", "Anthropic Claude Code CLI", "1.2.0");

    const card = cardFor("claude-code");
    expect(within(card).queryByRole("button", { name: "更改版本" })).toBeNull();
    expect(within(card).getByText("已是当前版本")).toBeTruthy();
    expect(prepareCliAction).not.toHaveBeenCalled();
    expect(executeCliAction).not.toHaveBeenCalled();
  });
});

describe("source catalogs", () => {
  it("offers only the owning source's versions, never a merged list", async () => {
    const withTwoSources = snapshot({
      sources: [npmSource(["1.3.0", "1.2.0"]), { ...homebrewSource(), availableVersions: ["9.9.9"] }],
    });
    await renderPage([withTwoSources]);

    const select = within(cardFor("claude-code"))
      .getByLabelText("Anthropic Claude Code CLI 目标版本") as HTMLSelectElement;
    expect([...select.options].map((option) => option.value)).toEqual(["1.3.0", "1.2.0"]);
  });

  it("shows a detect-only installation as manageable elsewhere, not as broken", async () => {
    const brewInstalled = snapshot({
      overallState: "ready",
      update: "unknown",
      installations: [{
        id: "claude-brew",
        executablePath: "/opt/homebrew/bin/claude",
        canonicalPath: null,
        aliasPaths: [],
        targetMissing: false,
        reportedVersion: "1.2.0",
        sourceId: "homebrew",
        sourceKind: "homebrew",
        sourceConfidence: "inferred",
        pathPriority: 0,
        environmentOrigin: "path",
        executableStatus: "healthy",
      }],
      pathSelectedInstallationId: "claude-brew",
      recommendedInstallationId: "claude-brew",
      sources: [homebrewSource()],
      allowedActions: [],
    });
    await renderPage([brewInstalled]);

    const card = cardFor("claude-code");
    expect(within(card).getByText("仅检测")).toBeTruthy();
    expect(within(card).getByText(/brew upgrade/)).toBeTruthy();
    // Healthy and unmanageable at once. Rendering it as broken is the defect this replaces.
    expect(within(card).getByText("可运行")).toBeTruthy();
    expect(within(card).queryByText("无法运行")).toBeNull();
    expect(within(card).queryByRole("button", { name: "更改版本" })).toBeNull();
  });
});

describe("operations are scoped to the tool they touch", () => {
  it("leaves an unrelated tool interactive while one is mutating", async () => {
    await renderPage([snapshot(), codexSnapshot()]);

    getOperationStatus.mockImplementation((id) =>
      Promise.resolve(operation({ id, status: "running", relatedEntityId: "claude-code" })));
    fireEvent.click(within(cardFor("claude-code")).getByRole("button", { name: "更改版本" }));

    await waitFor(() => expect(changeButton("claude-code").disabled).toBe(true));
    // The whole point: no global busy flag. The other tool is still actionable.
    expect(changeButton("codex-cli").disabled).toBe(false);
  });

  it("shows a queued operation only on the tool it belongs to", async () => {
    const queued = operation({ id: "op-queued", status: "queued", relatedEntityId: "claude-code" });
    getOperationStatus.mockResolvedValue(queued);
    await renderPage([snapshot({ lastOperationId: "op-queued" }), codexSnapshot()]);

    await waitFor(() => expect(within(cardFor("claude-code")).getByText("排队中")).toBeTruthy());
    expect(within(cardFor("codex-cli")).queryByText("排队中")).toBeNull();
  });

  it("cancels through the operation service rather than the CLI service", async () => {
    const running = operation({
      id: "op-running",
      status: "running",
      relatedEntityId: "claude-code",
      cancellable: true,
    });
    getOperationStatus.mockResolvedValue(running);
    await renderPage([snapshot({ lastOperationId: "op-running" })]);

    const card = cardFor("claude-code");
    fireEvent.click(await within(card).findByRole("button", { name: "取消" }));

    await waitFor(() => expect(cancelOperation).toHaveBeenCalledWith("op-running"));
  });
});

describe("outcomes the user cannot act on without an explanation", () => {
  it("explains applied-unverified rather than reporting a clean success", async () => {
    getOperationStatus.mockResolvedValue(operation({
      id: "op-done",
      status: "succeeded",
      relatedEntityId: "claude-code",
      result: {
        outcome: "applied-unverified",
        warnings: ["detection-failed"],
        warning: true,
        termination: "exited",
      },
    }));
    await renderPage([snapshot({ lastOperationId: "op-done" })]);

    const card = cardFor("claude-code");
    expect(await within(card).findByText("已执行，未能验证")).toBeTruthy();
    expect(within(card).getByText(/请先刷新检测/)).toBeTruthy();
  });

  it("states that a changed-but-failed run was not rolled back", async () => {
    getOperationStatus.mockResolvedValue(operation({
      id: "op-half",
      status: "failed",
      relatedEntityId: "claude-code",
      result: {
        outcome: "changed-but-failed",
        warnings: [],
        warning: true,
        termination: "exited",
      },
    }));
    await renderPage([snapshot({ lastOperationId: "op-half" })]);

    const card = cardFor("claude-code");
    expect(await within(card).findByText("已改动，但失败")).toBeTruthy();
    // Never a rollback claim: VaneHub did not undo the external install, and saying it did would
    // send the user off to verify a state that was never restored.
    expect(within(card).getByText(/没有回滚任何东西/)).toBeTruthy();
  });

  it("keeps the cached list on screen while a refresh runs", async () => {
    await renderPage([snapshot()]);

    fireEvent.click(screen.getByRole("button", { name: "刷新检测" }));

    await waitFor(() => expect(refreshCliEnvironments).toHaveBeenCalledWith([], false));
    // Blanking the list during a refresh would read as "nothing is installed" for as long as the
    // probes take.
    expect(screen.getByText("Anthropic Claude Code CLI")).toBeTruthy();
  });

  it("marks a stale snapshot without hiding what it knows", async () => {
    await renderPage([snapshot({ freshness: "stale" })]);

    const card = cardFor("claude-code");
    expect(within(card).getByText("数据已过期")).toBeTruthy();
    // Still showing what it last knew. A stale badge that also blanks the data says nothing.
    expect(card.textContent).toContain("1.2.0");
    expect(card.textContent).toContain("/mock/npm/bin/claude");
  });
});

describe("the action plan review", () => {
  async function openPlan(planOverrides: Partial<CliActionPlan> = {}) {
    getCliActionPlan.mockResolvedValue(plan(planOverrides));
    await renderPage([snapshot()]);
    selectVersion("claude-code", "Anthropic Claude Code CLI", "1.1.0");
    fireEvent.click(within(cardFor("claude-code")).getByRole("button", { name: "更改版本" }));
    return screen.findByRole("dialog");
  }

  it("shows the argv, the preconditions, the warnings, and the absence of a fallback", async () => {
    const dialog = await openPlan();

    // Argv, one argument per line. Never a shell string: there is nothing here to quote.
    const argv = dialog.querySelector("pre");
    expect(argv?.textContent).toBe("npm\ninstall\n-g\n@anthropic-ai/claude-code@1.1.0");
    expect(within(dialog).getByText("需要网络")).toBeTruthy();
    expect(within(dialog).getByText("失败不会自动改用其他来源")).toBeTruthy();
    expect(within(dialog).getByText("需要可访问网络")).toBeTruthy();
    expect(within(dialog).getByText(/退回旧版本/)).toBeTruthy();
  });

  it("submits the plan id and the revision that was on screen, and nothing else", async () => {
    const dialog = await openPlan();

    fireEvent.click(within(dialog).getByRole("button", { name: "确认执行" }));

    await waitFor(() => expect(executeCliAction).toHaveBeenCalledTimes(1));
    // Nothing here a command could be rebuilt from, which is what makes "the version reviewed is
    // the version that runs" structural rather than a convention.
    expect(executeCliAction).toHaveBeenCalledWith({ planId: "plan-1", expectedRevision: 7 });
  });

  it("refuses to run an expired plan and offers a new one instead", async () => {
    const dialog = await openPlan({ state: "expired" });

    expect(within(dialog).getByText("此计划已过期，请重新准备。")).toBeTruthy();
    expect(within(dialog).queryByRole("button", { name: "确认执行" })).toBeNull();
    fireEvent.click(within(dialog).getByRole("button", { name: "重新准备计划" }));

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(executeCliAction).not.toHaveBeenCalled();
  });

  it("refuses to run a plan that has already been consumed", async () => {
    const dialog = await openPlan({ state: "completed" });

    expect(within(dialog).getByText("此计划已经执行过，请重新准备。")).toBeTruthy();
    expect(within(dialog).queryByRole("button", { name: "确认执行" })).toBeNull();
  });
});

describe("bulk upgrade", () => {
  const bulkPlan: CliBulkActionPlan = {
    id: "bulk-1",
    revision: 2,
    items: [{
      agentId: "claude-code",
      planId: "plan-1",
      sourceId: "npm",
      currentVersion: "1.2.0",
      targetVersion: "1.3.0",
    }],
    skipped: [{ agentId: "codex-cli", reason: "already-current" }],
    createdAt: "2026-01-01T00:00:00.000Z",
    expiresAt: "2026-01-01T00:10:00.000Z",
  };

  async function openBulk() {
    getCliBulkActionPlan.mockResolvedValue(bulkPlan);
    getOperationStatus.mockImplementation((id) =>
      Promise.resolve(operation({ id, status: "succeeded", result: { planId: "bulk-1" } })));
    await renderPage([snapshot(), codexSnapshot()]);
    fireEvent.click(screen.getByRole("button", { name: /全部升级/ }));
    return screen.findByRole("dialog");
  }

  it("names what will run and what will not, with a reason for each skip", async () => {
    const dialog = await openBulk();

    expect(within(dialog).getByText("Anthropic Claude Code CLI")).toBeTruthy();
    expect(within(dialog).getByText("OpenAI Codex CLI")).toBeTruthy();
    // A shorter list with no reason reads as "everything else is up to date", which is a claim the
    // backend did not make.
    expect(within(dialog).getByText("已是目标版本")).toBeTruthy();
  });

  it("reports a real outcome for every item once it has run", async () => {
    const items: CliBulkItemResult[] = [
      {
        status: "completed",
        agentId: "claude-code",
        planId: "plan-1",
        sourceId: "npm",
        targetVersion: "1.3.0",
        outcome: "applied-unverified",
        reason: null,
      },
      {
        status: "skipped",
        agentId: "codex-cli",
        planId: null,
        sourceId: null,
        targetVersion: null,
        outcome: null,
        reason: "already-current",
      },
    ];
    const dialog = await openBulk();

    getOperationStatus.mockImplementation((id) =>
      Promise.resolve(operation({ id, status: "succeeded", result: { items } })));
    fireEvent.click(within(dialog).getByRole("button", { name: /执行 1 项/ }));

    await waitFor(() => expect(executeCliBulkAction).toHaveBeenCalledWith({
      planId: "bulk-1",
      expectedRevision: 2,
    }));
    // The placeholder this replaces reported "ran" for every item, which said a process had started
    // and nothing about whether the machine changed.
    expect(await within(dialog).findByText("已执行，未能验证")).toBeTruthy();
    expect(within(dialog).getByText("已是目标版本")).toBeTruthy();
    expect(within(dialog).queryByText("ran")).toBeNull();
  });
});

describe("the details drawer", () => {
  const writeText = vi.fn<(value: string) => Promise<void>>();

  async function openDrawer(snapshots = [snapshot()]) {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: (value: string) => writeText(value) },
    });
    await renderPage(snapshots);
    const trigger = within(cardFor("claude-code"))
      .getByRole("button", { name: "查看 Anthropic Claude Code CLI 的详情" });
    fireEvent.click(trigger);
    return { trigger, dialog: await screen.findByRole("dialog") };
  }

  it("reports its own open state on the trigger and points at the panel it opened", async () => {
    const { trigger, dialog } = await openDrawer();

    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(trigger.getAttribute("aria-haspopup")).toBe("dialog");
    const controls = trigger.getAttribute("aria-controls");
    expect(controls).toBeTruthy();
    expect(dialog.querySelector(`#${CSS.escape(controls ?? "")}`)).toBeTruthy();
  });

  it("moves between tabs by click and by arrow key, one tab stop for the strip", async () => {
    const { dialog } = await openDrawer();
    const tabs = within(dialog).getAllByRole("tab");

    expect(tabs.map((tab) => tab.textContent)).toEqual(["概览", "安装", "诊断", "操作"]);
    expect(tabs[0].getAttribute("aria-selected")).toBe("true");
    // Roving tabIndex: the strip is one stop, not four.
    expect(tabs.map((tab) => tab.getAttribute("tabindex"))).toEqual(["0", "-1", "-1", "-1"]);

    fireEvent.click(within(dialog).getByRole("tab", { name: "安装" }));
    expect(within(dialog).getByRole("tab", { name: "安装" }).getAttribute("aria-selected"))
      .toBe("true");
    expect(within(dialog).getByRole("tabpanel").textContent).toContain("/mock/npm/bin/claude");

    fireEvent.keyDown(within(dialog).getByRole("tablist"), { key: "ArrowRight" });
    expect(within(dialog).getByRole("tab", { name: "诊断" }).getAttribute("aria-selected"))
      .toBe("true");
    fireEvent.keyDown(within(dialog).getByRole("tablist"), { key: "ArrowLeft" });
    expect(within(dialog).getByRole("tab", { name: "安装" }).getAttribute("aria-selected"))
      .toBe("true");
  });

  it("keeps the full path reachable even though the visible one is truncated", async () => {
    const { dialog } = await openDrawer();
    fireEvent.click(within(dialog).getByRole("tab", { name: "安装" }));

    const panel = within(dialog).getByRole("tabpanel");
    // Truncation is a layout decision; the value a user pastes has to survive it.
    expect(within(panel).getByTitle("/mock/npm/bin/claude")).toBeTruthy();
    fireEvent.click(within(panel).getByRole("button", { name: "复制路径" }));
    expect(writeText).toHaveBeenCalledWith("/mock/npm/bin/claude");
  });

  it("copies identifiers and enums out of an operation, never its log lines", async () => {
    getOperationStatus.mockResolvedValue(operation({
      id: "op-done",
      status: "succeeded",
      relatedEntityId: "claude-code",
      logs: [{ operationId: "op-done", line: "npm WARN deprecated secret-looking-token", timestamp: "1" }],
      result: {
        outcome: "verified",
        action: "upgrade",
        sourceId: "npm",
        termination: "exited",
        elapsedMs: 1200,
        warnings: [],
        warning: false,
      },
    }));
    const { dialog } = await openDrawer([snapshot({ lastOperationId: "op-done" })]);
    fireEvent.click(within(dialog).getByRole("tab", { name: "操作" }));

    const panel = within(dialog).getByRole("tabpanel");
    fireEvent.click(within(panel).getByRole("button", { name: "复制摘要" }));

    const copied = writeText.mock.calls.at(-1)?.[0] ?? "";
    expect(copied).toContain("outcome: verified");
    expect(copied).toContain("termination: exited");
    // Process output is bounded and redacted upstream and belongs on the log, not in a summary a
    // user is invited to paste somewhere.
    expect(copied).not.toContain("secret-looking-token");
  });

  it("reruns diagnostics through the CLI service", async () => {
    const { dialog } = await openDrawer();
    fireEvent.click(within(dialog).getByRole("tab", { name: "诊断" }));
    fireEvent.click(within(dialog).getByRole("button", { name: "重新运行诊断" }));

    await waitFor(() => expect(runCliDoctor).toHaveBeenCalledWith("claude-code"));
  });

  it("closes on Escape and puts focus back on the control that opened it", async () => {
    const { trigger, dialog } = await openDrawer();
    expect(document.activeElement).toBe(within(dialog).getByRole("tab", { name: "概览" }));

    fireEvent.keyDown(document, { key: "Escape" });

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    // Focus back where it came from: leaving it on the body drops a keyboard user at the top of
    // the page every time they look at a tool.
    expect(document.activeElement).toBe(trigger);
  });
});

describe("filters", () => {
  it("filters by search, by summary bucket, by source, and by needing attention", async () => {
    const ready = codexSnapshot({ overallState: "ready", update: "up-to-date" });
    await renderPage([snapshot(), ready]);

    fireEvent.change(screen.getByLabelText("搜索 CLI"), { target: { value: "codex" } });
    expect(screen.queryByText("Anthropic Claude Code CLI")).toBeNull();
    expect(screen.getByText("OpenAI Codex CLI")).toBeTruthy();

    fireEvent.change(screen.getByLabelText("搜索 CLI"), { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: /可更新/ }));
    expect(screen.getByText("Anthropic Claude Code CLI")).toBeTruthy();
    expect(screen.queryByText("OpenAI Codex CLI")).toBeNull();

    // Clicking the active bucket clears it: the same control both applies and removes the filter.
    fireEvent.click(screen.getByRole("button", { name: /可更新/ }));
    expect(screen.getByText("OpenAI Codex CLI")).toBeTruthy();

    fireEvent.click(screen.getByLabelText("只看需要处理的"));
    expect(screen.queryByText("OpenAI Codex CLI")).toBeNull();
    expect(screen.getByText("Anthropic Claude Code CLI")).toBeTruthy();
  });

  it("keeps the filter applied while the list refetches underneath it", async () => {
    await renderPage([snapshot(), codexSnapshot()]);

    fireEvent.change(screen.getByLabelText("搜索 CLI"), { target: { value: "codex" } });
    fireEvent.click(screen.getByRole("button", { name: "刷新检测" }));

    await waitFor(() => expect(refreshCliEnvironments).toHaveBeenCalled());
    expect(screen.queryByText("Anthropic Claude Code CLI")).toBeNull();
    expect((screen.getByLabelText("搜索 CLI") as HTMLInputElement).value).toBe("codex");
  });

  it("says so when nothing matches rather than rendering an empty grid", async () => {
    await renderPage([snapshot()]);

    fireEvent.change(screen.getByLabelText("搜索 CLI"), { target: { value: "nothing-matches" } });

    expect(screen.getByText("没有符合当前筛选条件的 CLI。")).toBeTruthy();
  });
});

describe("nav entry status (task 12.16)", () => {
  it("reports an update-available status while a tool has one, and null once none do", async () => {
    const updateAvailable = vi.fn();
    // Default `snapshot()` is `overallState: "update-available"`, the same count `CliSummaryBar`
    // already renders live -- this reports the same number, not a second computation of it.
    const { unmount } = await renderPage([snapshot()], updateAvailable);
    await waitFor(() => expect(updateAvailable).toHaveBeenLastCalledWith({
      kind: "update-available",
      labelKey: "cli.status.updateAvailable",
      labelParams: { count: 1 },
    }));
    unmount();

    const healthy = vi.fn();
    await renderPage([snapshot({ overallState: "ready", update: "up-to-date" })], healthy);
    await waitFor(() => expect(healthy).toHaveBeenLastCalledWith(null));
  });
});
