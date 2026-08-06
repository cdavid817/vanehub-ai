// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import type { PrincipalEntry } from "../../types/permissions";

const agentServiceMocks = vi.hoisted(() => ({
  listAgents: vi.fn(),
}));
const permissionsServiceMocks = vi.hoisted(() => ({
  getAgentPolicyPrincipal: vi.fn(),
  applyPolicyTemplate: vi.fn(),
}));

vi.mock("../../services/runtime-agent-client", () => ({ agentService: agentServiceMocks }));
vi.mock("../../services/runtime-permissions-client", () => ({ permissionsService: permissionsServiceMocks }));

import { AgentPoliciesPage } from "./agent-policies-page";

function principalFor(agentId: string, overrides: Partial<PrincipalEntry> = {}): PrincipalEntry {
  return {
    agentId,
    template: "standard",
    requiresConfirmationToAssign: false,
    hasExplicitAssignment: false,
    ...overrides,
  };
}

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <AgentPoliciesPage searchTerm="" />
    </QueryClientProvider>,
  );
}

function rowFor(title: string): HTMLElement {
  const row = screen.getByText(title).closest("div.grid");
  if (!row) throw new Error(`Row not found: ${title}`);
  return row as HTMLElement;
}

beforeEach(() => {
  vi.clearAllMocks();
  agentServiceMocks.listAgents.mockResolvedValue([]);
  permissionsServiceMocks.getAgentPolicyPrincipal.mockImplementation(async (agentId: string) =>
    principalFor(agentId),
  );
});

describe("AgentPoliciesPage — Claude Code first-use install confirmation", () => {
  it("declining the confirmation applies no template and leaves the hook uninstalled", async () => {
    const user = userEvent.setup();
    renderPage();

    const standardButton = await waitFor(() => {
      const button = within(rowFor("Claude Code")).getByRole("button", { name: "标准" }) as HTMLButtonElement;
      expect(button.disabled).toBe(false);
      return button;
    });
    await user.click(standardButton);

    expect(await screen.findByText("启用 Claude Code 权限管理?")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "取消" }));

    expect(screen.queryByText("启用 Claude Code 权限管理?")).toBeNull();
    expect(permissionsServiceMocks.applyPolicyTemplate).not.toHaveBeenCalled();

    // Declining must leave the principal exactly as it was — clicking again should show the
    // same first-use confirmation, not silently skip it as if installation had happened.
    await user.click(within(rowFor("Claude Code")).getByRole("button", { name: "标准" }));
    expect(await screen.findByText("启用 Claude Code 权限管理?")).toBeTruthy();
  });

  it("confirming the install dialog applies the template exactly once", async () => {
    permissionsServiceMocks.applyPolicyTemplate.mockResolvedValue(
      principalFor("claude-code", { hasExplicitAssignment: true }),
    );
    const user = userEvent.setup();
    renderPage();

    await user.click(
      await waitFor(() => within(rowFor("Claude Code")).getByRole("button", { name: "标准" })),
    );
    await screen.findByText("启用 Claude Code 权限管理?");
    await user.click(screen.getByRole("button", { name: "确认" }));

    await waitFor(() => expect(permissionsServiceMocks.applyPolicyTemplate).toHaveBeenCalledTimes(1));
    expect(permissionsServiceMocks.applyPolicyTemplate).toHaveBeenCalledWith("claude-code", "standard");
    expect(screen.queryByText("启用 Claude Code 权限管理?")).toBeNull();
  });
});

describe("AgentPoliciesPage — other managed CLI agents skip the install confirmation", () => {
  it("codex-cli applies standard immediately, with no install-confirmation dialog", async () => {
    permissionsServiceMocks.applyPolicyTemplate.mockResolvedValue(
      principalFor("codex-cli", { hasExplicitAssignment: true }),
    );
    const user = userEvent.setup();
    renderPage();

    const standardButton = await waitFor(() => {
      const button = within(rowFor("Codex CLI")).getByRole("button", { name: "标准" }) as HTMLButtonElement;
      expect(button.disabled).toBe(false);
      return button;
    });
    await user.click(standardButton);

    expect(screen.queryByText("启用 Claude Code 权限管理?")).toBeNull();
    await waitFor(() => expect(permissionsServiceMocks.applyPolicyTemplate).toHaveBeenCalledTimes(1));
    expect(permissionsServiceMocks.applyPolicyTemplate).toHaveBeenCalledWith("codex-cli", "standard");
  });

  it("codex-cli still shows the generic trusted/yolo confirmation", async () => {
    const user = userEvent.setup();
    renderPage();

    const trustedButton = await waitFor(() =>
      within(rowFor("Codex CLI")).getByRole("button", { name: "信任" }),
    );
    await user.click(trustedButton);

    expect(await screen.findByText("确认提升这个 Agent 的信任等级?")).toBeTruthy();
    expect(permissionsServiceMocks.applyPolicyTemplate).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "确认" }));
    await waitFor(() => expect(permissionsServiceMocks.applyPolicyTemplate).toHaveBeenCalledTimes(1));
    expect(permissionsServiceMocks.applyPolicyTemplate).toHaveBeenCalledWith("codex-cli", "trusted");
  });
});
