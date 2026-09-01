// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import type { SshConnectionService } from "../../services/ssh-connection-service";
import type { SaveSshConnectionInput, SshConnection, SshConnectionTestResult } from "../../types/ssh-connection";
import { renderWithAppProviders } from "../../test/render";
import { SshConnectionsPage } from "./ssh-connections-page";

const { serviceRef } = vi.hoisted(() => ({
  serviceRef: { current: null as SshConnectionService | null },
}));

vi.mock("../../services/runtime-ssh-connection-client", () => ({
  get sshConnectionService() {
    if (!serviceRef.current) throw new Error("no service installed");
    return serviceRef.current;
  },
}));

function connection(overrides: Partial<SshConnection> = {}): SshConnection {
  return {
    id: "conn-1",
    name: "prod-app",
    host: "10.0.0.5",
    port: 22,
    user: "deploy",
    defaultPath: "/srv/app",
    authMode: "key",
    keyPath: "~/.ssh/id_ed25519",
    hasPassword: false,
    revision: 1,
    hostTrust: null,
    testStatus: "not-tested",
    lastConnectedAt: null,
    lastError: null,
    createdAt: "2026-08-01T00:00:00Z",
    updatedAt: "2026-08-01T00:00:00Z",
    ...overrides,
  };
}

function install(overrides: Partial<SshConnectionService> = {}): SshConnectionService {
  const service: SshConnectionService = {
    listConnections: vi.fn(async () => [connection()]),
    createConnection: vi.fn(async (input: SaveSshConnectionInput) => connection({ ...input, id: "conn-new" })),
    updateConnection: vi.fn(async (_id, input: SaveSshConnectionInput) => connection({ ...input, id: "conn-1" })),
    deleteConnection: vi.fn(async () => undefined),
    testConnection: vi.fn(async () => ({ message: "ok", status: "succeeded" as const, testedAt: "2026-08-01T00:00:00Z" })),
    ...overrides,
  };
  serviceRef.current = service;
  return service;
}

function renderPage() {
  return renderWithAppProviders(<SshConnectionsPage searchTerm="" />);
}

async function whenLoaded() {
  return screen.findByRole("article");
}

describe("SshConnectionsPage (task 12.18 shared-primitive migration)", () => {
  beforeEach(async () => {
    await activateAppLanguage("zh-CN");
  });

  it("shows the page icon and exactly one primary header action, with Refresh tucked behind More", async () => {
    install();
    const { user } = renderPage();
    await whenLoaded();

    expect(document.querySelector(".border-b.border-border-subtle")?.querySelector("svg")).toBeTruthy();
    expect(screen.getByRole("button", { name: "新增" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "刷新" })).toBeNull();

    await user.click(screen.getByRole("button", { name: "更多操作" }));
    expect(screen.getByRole("menuitem", { name: "刷新" })).toBeTruthy();
  });

  it("collapses per-card actions behind one ActionMenu instead of three separate buttons", async () => {
    install();
    const { user } = renderPage();
    const card = await whenLoaded();

    expect(within(card).queryByRole("button", { name: "测试" })).toBeNull();
    expect(within(card).queryByRole("button", { name: "编辑" })).toBeNull();
    expect(within(card).queryByRole("button", { name: "删除" })).toBeNull();

    await user.click(within(card).getByRole("button", { name: "prod-app的操作" }));
    expect(within(card).getByRole("menuitem", { name: "测试" })).toBeTruthy();
    expect(within(card).getByRole("menuitem", { name: "编辑" })).toBeTruthy();
    expect(within(card).getByRole("menuitem", { name: "删除" })).toBeTruthy();
  });

  it("shows pending, then clears, mutation status for a test triggered from the menu", async () => {
    let resolveTest: (value: SshConnectionTestResult) => void = () => {};
    const service = install({
      testConnection: vi.fn(() => new Promise<SshConnectionTestResult>((resolve) => { resolveTest = resolve; })),
    });
    const { user } = renderPage();
    const card = await whenLoaded();

    await user.click(within(card).getByRole("button", { name: "prod-app的操作" }));
    await user.click(within(card).getByRole("menuitem", { name: "测试" }));

    expect(within(card).getByRole("status").textContent).toContain("保存中");
    resolveTest({ message: "ok", status: "succeeded", testedAt: "2026-08-01T00:00:00Z" });
    await waitFor(() => expect(within(card).queryByRole("status")).toBeNull());
    expect(service.testConnection).toHaveBeenCalledWith("conn-1");
  });

  it("asks for confirmation before deleting, and does not call delete when cancelled", async () => {
    const service = install();
    const { user } = renderPage();
    const card = await whenLoaded();

    await user.click(within(card).getByRole("button", { name: "prod-app的操作" }));
    await user.click(within(card).getByRole("menuitem", { name: "删除" }));

    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(service.deleteConnection).not.toHaveBeenCalled();
  });

  it("renders the shared EmptyState with a create action when there are no connections", async () => {
    install({ listConnections: vi.fn(async () => []) });
    renderPage();

    await waitFor(() => expect(screen.getByText("暂无 SSH 连接")).toBeTruthy());
    expect(screen.getByRole("button", { name: "新增连接" })).toBeTruthy();
    expect(screen.queryByRole("article")).toBeNull();
  });

  it("renders the shared AsyncBoundary error state with a working retry", async () => {
    const listConnections = vi.fn(async (): Promise<SshConnection[]> => { throw new Error("network down"); });
    install({ listConnections });
    const { user } = renderPage();

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("network down"));
    listConnections.mockResolvedValueOnce([connection()]);
    await user.click(screen.getByRole("button", { name: "重试" }));
    await whenLoaded();
  });
});
