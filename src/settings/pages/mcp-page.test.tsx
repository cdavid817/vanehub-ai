// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import type { McpService } from "../../services/mcp-service";
import type { OperationService } from "../../services/operation-service";
import type { McpServerConfig } from "../../types/mcp";
import type { OperationTask } from "../../types/operation";
import { renderWithAppProviders } from "../../test/render";
import { McpPage } from "./mcp-page";

const { operationServiceRef, serviceRef } = vi.hoisted(() => ({
  operationServiceRef: { current: null as OperationService | null },
  serviceRef: { current: null as McpService | null },
}));

vi.mock("../../services/runtime-mcp-client", () => ({
  get mcpService() {
    if (!serviceRef.current) throw new Error("no MCP service installed");
    return serviceRef.current;
  },
}));

vi.mock("../../services/runtime-operation-client", () => ({
  get operationService() {
    if (!operationServiceRef.current) throw new Error("no operation service installed");
    return operationServiceRef.current;
  },
}));

function server(overrides: Partial<McpServerConfig> = {}): McpServerConfig {
  return {
    active: true,
    args: ["mcp-server"],
    command: "npx",
    name: "docs-mcp",
    scope: "user",
    transportType: "stdio",
    ...overrides,
  };
}

function operation(overrides: Partial<OperationTask> = {}): OperationTask {
  return {
    createdAt: "2026-08-01T00:00:00Z",
    id: "op-1",
    kind: "mcp",
    logs: [],
    relatedEntityId: null,
    status: "succeeded",
    updatedAt: "2026-08-01T00:00:00Z",
    ...overrides,
  };
}

function install(overrides: Partial<McpService> = {}): McpService {
  const service: McpService = {
    addServer: vi.fn(async () => undefined),
    callTool: vi.fn(async () => ({ content: "", isError: false })),
    exportServers: vi.fn(async () => ({ mcpServers: {} })),
    getServerStatus: vi.fn(async (name: string) => ({ connectionStatus: "connected" as const, name, tools: [] })),
    importServers: vi.fn(async () => ({ failures: [], imported: [], skipped: [] })),
    listServers: vi.fn(async () => [server()]),
    removeServer: vi.fn(async () => undefined),
    testConnection: vi.fn(async (name: string) => operation({ relatedEntityId: name })),
    toggleServer: vi.fn(async () => undefined),
    updateServer: vi.fn(async () => undefined),
    ...overrides,
  };
  serviceRef.current = service;
  return service;
}

function installOperations(overrides: Partial<OperationService> = {}): OperationService {
  const service: OperationService = {
    cancelOperation: vi.fn(async () => operation()),
    getOperationStatus: vi.fn(async () => operation()),
    listOperations: vi.fn(async () => []),
    ...overrides,
  };
  operationServiceRef.current = service;
  return service;
}

function renderPage() {
  return renderWithAppProviders(<McpPage searchTerm="" />);
}

async function whenLoaded() {
  return screen.findByRole("article");
}

describe("McpPage (task 12.18 shared-primitive migration)", () => {
  beforeEach(async () => {
    await activateAppLanguage("zh-CN");
    installOperations();
  });

  it("shows the page icon and exactly one primary header action, with Refresh and Import/Export tucked behind More", async () => {
    install();
    const { user } = renderPage();
    await whenLoaded();

    expect(document.querySelector(".border-b.border-border-subtle")?.querySelector("svg")).toBeTruthy();
    expect(screen.getByRole("button", { name: "添加 MCP" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "刷新" })).toBeNull();
    expect(screen.queryByRole("button", { name: "导入/导出" })).toBeNull();

    await user.click(screen.getByRole("button", { name: "更多操作" }));
    expect(screen.getByRole("menuitem", { name: "刷新" })).toBeTruthy();
    expect(screen.getByRole("menuitem", { name: "导入/导出" })).toBeTruthy();
  });

  it("collapses per-card actions behind one ActionMenu instead of four separate buttons", async () => {
    install();
    const { user } = renderPage();
    const card = await whenLoaded();

    expect(within(card).queryByRole("button", { name: "测试" })).toBeNull();
    expect(within(card).queryByRole("button", { name: "编辑" })).toBeNull();
    expect(within(card).queryByRole("button", { name: "删除" })).toBeNull();
    expect(within(card).queryByRole("button", { name: /^(启用|禁用) docs-mcp$/ })).toBeNull();

    await user.click(within(card).getByRole("button", { name: "docs-mcp的操作" }));
    expect(within(card).getByRole("menuitem", { name: "测试" })).toBeTruthy();
    expect(within(card).getByRole("menuitem", { name: "编辑" })).toBeTruthy();
    expect(within(card).getByRole("menuitem", { name: "删除" })).toBeTruthy();
    expect(within(card).getByRole("menuitem", { name: "禁用 docs-mcp" })).toBeTruthy();
  });

  it("asks for confirmation before deleting, and does not call delete when cancelled", async () => {
    const service = install();
    const { user } = renderPage();
    const card = await whenLoaded();

    await user.click(within(card).getByRole("button", { name: "docs-mcp的操作" }));
    await user.click(within(card).getByRole("menuitem", { name: "删除" }));

    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(service.removeServer).not.toHaveBeenCalled();
  });

  it("shows pending, then clears, mutation status for the toggle action", async () => {
    let resolveToggle: () => void = () => {};
    const service = install({
      toggleServer: vi.fn(() => new Promise<void>((resolve) => { resolveToggle = resolve; })),
    });
    const { user } = renderPage();
    const card = await whenLoaded();

    await user.click(within(card).getByRole("button", { name: "docs-mcp的操作" }));
    await user.click(within(card).getByRole("menuitem", { name: "禁用 docs-mcp" }));

    expect(within(card).getByRole("status").textContent).toContain("保存中");
    resolveToggle();
    await waitFor(() => expect(within(card).queryByRole("status")).toBeNull());
    expect(service.toggleServer).toHaveBeenCalledWith("docs-mcp", false);
  });

  it("runs the two-phase test-connection lifecycle: pending during the call, cleared with a pass notice once the operation settles", async () => {
    let resolveConnect: (op: OperationTask) => void = () => {};
    const testConnection = vi.fn(() => new Promise<OperationTask>((resolve) => { resolveConnect = resolve; }));
    install({ testConnection });
    installOperations({
      getOperationStatus: vi.fn(async () =>
        operation({ relatedEntityId: "docs-mcp", result: { success: true, tools: [] }, status: "succeeded" }),
      ),
    });

    const { user } = renderPage();
    const card = await whenLoaded();

    await user.click(within(card).getByRole("button", { name: "docs-mcp的操作" }));
    await user.click(within(card).getByRole("menuitem", { name: "测试" }));

    expect(within(card).getByRole("status").textContent).toContain("保存中");
    resolveConnect(operation({ relatedEntityId: "docs-mcp", status: "running" }));

    await waitFor(() => expect(within(card).queryByRole("status")).toBeNull());
    expect(await screen.findByText(/测试通过，发现 0 个工具/)).toBeTruthy();
    expect(testConnection).toHaveBeenCalledWith("docs-mcp");
  });

  it("groups servers into user/project scope sections, hiding a section with no matching servers", async () => {
    install({
      getServerStatus: vi.fn(async (name: string) => ({ connectionStatus: "connected" as const, name, tools: [] })),
      listServers: vi.fn(async () => [server({ name: "docs-mcp", scope: "user" }), server({ name: "ci-mcp", scope: "project" })]),
    });
    renderPage();
    await screen.findByText("docs-mcp");

    expect(screen.getByText("用户配置")).toBeTruthy();
    expect(screen.getByText("项目配置")).toBeTruthy();
  });

  it("hides the project-scope section entirely when every visible server is user-scoped", async () => {
    install({ listServers: vi.fn(async () => [server({ name: "docs-mcp", scope: "user" })]) });
    renderPage();
    await whenLoaded();

    expect(screen.getByText("用户配置")).toBeTruthy();
    expect(screen.queryByText("项目配置")).toBeNull();
  });

  it("renders the shared EmptyState with a create action when there are no servers", async () => {
    install({ listServers: vi.fn(async () => []) });
    renderPage();

    await waitFor(() => expect(screen.getByText("没有可见的 MCP 服务器")).toBeTruthy());
    expect(screen.getByRole("button", { name: "添加第一个 MCP 服务器" })).toBeTruthy();
    expect(screen.queryByRole("article")).toBeNull();
  });

  it("renders the shared AsyncBoundary error state with a working retry, using the safe error-code taxonomy", async () => {
    const failure = Object.assign(new Error("stdio process exited"), { errorCode: "spawn" });
    const listServers = vi.fn(async (): Promise<McpServerConfig[]> => { throw failure; });
    install({ listServers });
    const { user } = renderPage();

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("进程启动失败"));
    listServers.mockResolvedValueOnce([server()]);
    await user.click(screen.getByRole("button", { name: "重试" }));
    await whenLoaded();
  });

  it("never surfaces a raw, uncoded list-fetch error message -- only the safe fallback", async () => {
    const listServers = vi.fn(async (): Promise<McpServerConfig[]> => {
      throw new Error("Authorization: Bearer sk-should-not-leak");
    });
    install({ listServers });
    renderPage();

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("没有可安全显示的错误详情"));
    expect(screen.getByRole("alert").textContent).not.toContain("sk-should-not-leak");
  });

  it("never renders a server's env/headers credential values anywhere in the card list", async () => {
    install({
      listServers: vi.fn(async () => [
        server({ env: { API_KEY: "sk-should-never-render-e46f" }, name: "secret-mcp" }),
      ]),
    });
    renderPage();
    await whenLoaded();

    expect(screen.queryByText(/sk-should-never-render-e46f/)).toBeNull();
  });
});
