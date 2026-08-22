// @vitest-environment jsdom

// Failing regression coverage for the two defects this change exists to remove. Both are currently
// asserted *as correct* by the existing suite (`cli-management-utils.test.ts` expects "upgrade" when
// the target equals the active version), so these tests are deliberately written against the
// intended behavior and fail until the source-aware backend contract lands.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import type { CliToolStatus } from "../../types/agent";
import type { OperationTask } from "../../types/operation";

const installCliVersion = vi.fn<(input: unknown) => Promise<OperationTask>>();
const listCliTools = vi.fn<() => Promise<CliToolStatus[]>>();

vi.mock("../../services/runtime-agent-client", () => ({
  agentService: {
    listCliTools: () => listCliTools(),
    installCliVersion: (input: unknown) => installCliVersion(input),
    refreshCliDetections: () => Promise.resolve(startedOperation("op-refresh")),
    upgradeAllCliVersions: () => Promise.resolve(startedOperation("op-bulk")),
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
    kind: "agent",
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

const cliTool: CliToolStatus = {
  agentId: "claude-code",
  displayName: "Anthropic Claude Code CLI",
  provider: "Anthropic",
  executableName: "claude",
  packageName: "@anthropic-ai/claude-code",
  installed: true,
  currentVersion: "1.2.0",
  latestVersion: "1.3.0",
  availableVersions: ["1.3.0", "1.2.0", "1.1.0"],
  detectedPath: "C:\\Users\\dev\\claude.cmd",
  installCommand: "npm install -g @anthropic-ai/claude-code@latest",
  lastCheckedAt: "123",
  lastError: null,
  lastOperationId: null,
  versionCheckStatus: "succeeded",
  environmentType: "windows",
  installations: [{
    path: "C:\\Users\\dev\\claude.cmd",
    version: "1.2.0",
    runnable: true,
    error: null,
    source: "npm",
    environmentType: "windows",
    isActive: true,
  }],
  activeInstallationPath: "C:\\Users\\dev\\claude.cmd",
  conflictState: "none",
  lifecycleEligibility: "npm",
};

async function renderPage(tool: CliToolStatus) {
  const { ProvidersPage } = await import("./providers-page");
  listCliTools.mockResolvedValue([tool]);
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  render(
    <QueryClientProvider client={queryClient}>
      <ProvidersPage searchTerm="" />
    </QueryClientProvider>,
  );
  await screen.findByText(tool.displayName);
}

describe("CLI lifecycle regressions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    installCliVersion.mockResolvedValue(startedOperation("op-install"));
  });

  it("sends the version the user selected, not the latest version", async () => {
    await renderPage(cliTool);

    const select = await screen.findByLabelText(`${cliTool.displayName} 目标版本`);
    fireEvent.change(select, { target: { value: "1.1.0" } });
    expect((select as HTMLSelectElement).value).toBe("1.1.0");

    // Scoped to the card: the page toolbar also carries a bulk "全部升级 1" button.
    const card = document.querySelector<HTMLElement>('[data-cli-agent="claude-code"]');
    if (!card) throw new Error("CLI card not rendered");
    fireEvent.click(within(card).getByRole("button", { name: /^(升级|降级|安装)$/ }));

    await waitFor(() => expect(installCliVersion).toHaveBeenCalledTimes(1));
    // Defect: `resolveCliPackageActionTargetVersion` returns `tool.latestVersion ?? "latest"`, so the
    // selected 1.1.0 never reaches the request and 1.3.0 is installed instead.
    expect(installCliVersion).toHaveBeenCalledWith(
      expect.objectContaining({ agentId: "claude-code", targetVersion: "1.1.0" }),
    );
  });

  it("creates no mutation when the selected target equals the active version", async () => {
    const current = { ...cliTool, currentVersion: "1.3.0", installations: [{ ...cliTool.installations[0], version: "1.3.0" }] };
    await renderPage(current);

    const select = await screen.findByLabelText(`${current.displayName} 目标版本`);
    fireEvent.change(select, { target: { value: "1.3.0" } });

    // Defect: equality is derived as "upgrade", so an enabled upgrade button is rendered and a
    // redundant npm install is dispatched for a version that is already active.
    const upgradeButton = screen.queryByRole("button", { name: "升级" });
    expect(upgradeButton === null || (upgradeButton as HTMLButtonElement).disabled).toBe(true);

    if (upgradeButton && !(upgradeButton as HTMLButtonElement).disabled) fireEvent.click(upgradeButton);
    await waitFor(() => expect(installCliVersion).not.toHaveBeenCalled());
  });
});
