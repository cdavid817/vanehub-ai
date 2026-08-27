// @vitest-environment jsdom

// Regression coverage for the two defects this change exists to remove.
//
// Both were written against the *intended* behavior while the page still called the flat
// `installCliVersion` path, and both failed for the whole time that path was in place. They now
// assert the same two things against the action-plan surface that replaced it, because that is
// where "the selected version reaches the backend" and "equality creates no mutation" are now
// decided. The assertions are unchanged in substance: an exact version passthrough, and no
// mutation at all when the target is already installed.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import type { CliEnvironmentSnapshot } from "../../types/cli-environment-snapshot";
import type { OperationTask } from "../../types/operation";

const prepareCliAction = vi.fn<(input: unknown) => Promise<OperationTask>>();
const executeCliAction = vi.fn<(input: unknown) => Promise<OperationTask>>();
const listCliEnvironments = vi.fn<() => Promise<CliEnvironmentSnapshot[]>>();

vi.mock("../../services/runtime-agent-client", () => ({
  agentService: {
    listCliEnvironments: () => listCliEnvironments(),
    prepareCliAction: (input: unknown) => prepareCliAction(input),
    executeCliAction: (input: unknown) => executeCliAction(input),
    refreshCliEnvironments: () => Promise.resolve(startedOperation("op-refresh")),
    prepareCliBulkUpgrade: () => Promise.resolve(startedOperation("op-bulk")),
  },
}));

vi.mock("../../services/runtime-operation-client", () => ({
  operationService: {
    getOperationStatus: (id: string) => Promise.resolve({ ...startedOperation(id), status: "succeeded" as const }),
  },
}));

vi.mock("../../services/runtime-settings-client", () => ({
  settingsService: { reportClientLogEvent: () => Promise.resolve() },
}));

function startedOperation(id: string): OperationTask {
  return {
    id,
    kind: "cli",
    status: "running",
    relatedEntityId: "claude-code",
    message: null,
    logs: [],
    result: null,
    error: null,
    createdAt: "1",
    updatedAt: "1",
  };
}

const npmSource = {
  sourceId: "npm",
  kind: "npm",
  supportedOnThisPlatform: true,
  availableVersionCount: 3,
  availableVersions: ["1.3.0", "1.2.0", "1.1.0"],
  management: "managed" as const,
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

function snapshotAt(installedVersion: string): CliEnvironmentSnapshot {
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
      id: "claude",
      executablePath: "/mock/bin/claude",
      canonicalPath: null,
      aliasPaths: [],
      targetMissing: false,
      reportedVersion: installedVersion,
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
      { action: "downgrade", sourceId: "npm", targetMode: "exact", defaultTarget: null, requiresTargetSelection: true, reasonCode: null },
    ],
    lastMutation: null,
    lastOperationId: null,
    checkedAt: "2026-01-01T00:00:00+00:00",
  };
}

async function renderPage(snapshot: CliEnvironmentSnapshot) {
  const { CliManagementPage } = await import("./cli-management/cli-management-page");
  listCliEnvironments.mockResolvedValue([snapshot]);
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  render(
    <QueryClientProvider client={queryClient}>
      <CliManagementPage searchTerm="" />
    </QueryClientProvider>,
  );
  await screen.findByText(snapshot.displayName);
}

describe("CLI lifecycle regressions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    prepareCliAction.mockResolvedValue(startedOperation("op-plan"));
    executeCliAction.mockResolvedValue(startedOperation("op-execute"));
  });

  it("sends the version the user selected, not the latest version", async () => {
    await renderPage(snapshotAt("1.2.0"));

    const select = await screen.findByLabelText("Anthropic Claude Code CLI 目标版本");
    fireEvent.change(select, { target: { value: "1.1.0" } });
    expect((select as HTMLSelectElement).value).toBe("1.1.0");

    // Scoped to the card: the page toolbar also carries a bulk upgrade button.
    const card = document.querySelector<HTMLElement>('[data-cli-agent="claude-code"]');
    if (!card) throw new Error("CLI card not rendered");
    fireEvent.click(within(card).getByRole("button", { name: "更改版本" }));

    await waitFor(() => expect(prepareCliAction).toHaveBeenCalledTimes(1));
    // The old defect: `resolveCliPackageActionTargetVersion` returned `latestVersion ?? "latest"`,
    // so the selected 1.1.0 never reached the request and 1.3.0 was installed instead.
    expect(prepareCliAction).toHaveBeenCalledWith(
      expect.objectContaining({ agentId: "claude-code", targetVersion: "1.1.0" }),
    );
  });

  it("creates no mutation when the selected target equals the active version", async () => {
    await renderPage(snapshotAt("1.3.0"));

    const select = await screen.findByLabelText("Anthropic Claude Code CLI 目标版本");
    fireEvent.change(select, { target: { value: "1.3.0" } });

    // The old defect: equality was derived as "upgrade", so an enabled upgrade button rendered and
    // a redundant npm install was dispatched for a version that was already active.
    const changeButton = screen.queryByRole("button", { name: "更改版本" });
    expect(changeButton === null || (changeButton as HTMLButtonElement).disabled).toBe(true);

    if (changeButton && !(changeButton as HTMLButtonElement).disabled) fireEvent.click(changeButton);
    await waitFor(() => expect(prepareCliAction).not.toHaveBeenCalled());
    // And nothing is executed either: no plan was created, so there is none to run.
    expect(executeCliAction).not.toHaveBeenCalled();
  });
});
