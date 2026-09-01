// @vitest-environment jsdom

import { readFileSync } from "node:fs";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import en from "../../i18n/locales/en.json";
import type { ExtensionService } from "../../services/extension-service";
import type { ExtensionEnvironment, ExtensionFrameworkDefinition, ExtensionFrameworkStatus, ExtensionOverview } from "../../types/extension";
import type { OperationTask } from "../../types/operation";
import { renderWithAppProviders } from "../../test/render";
import { ExtensionsPage, filterExtensionDefinitions } from "./extensions-page";

vi.mock("../../services/runtime-operation-client", () => ({
  operationService: {
    // Every scenario in this file drives the operation lifecycle through the resolved value of
    // the `ExtensionService` method itself (already terminal by the time it resolves), so this
    // stub only needs to exist -- not vary -- to keep `operationQuery`'s own immediate fetch from
    // reaching the real Tauri/web-mock adapter.
    getOperationStatus: async (): Promise<OperationTask> => ({
      id: "op-stub",
      kind: "extension",
      status: "succeeded",
      relatedEntityId: null,
      logs: [],
      createdAt: "2026-08-01T00:00:00Z",
      updatedAt: "2026-08-01T00:00:00Z",
    }),
  },
}));

const definitions: ExtensionFrameworkDefinition[] = [
  {
    id: "paddleocr",
    capabilityId: "ocr",
    nameKey: "extensions.framework.paddleocr.name",
    descriptionKey: "extensions.framework.paddleocr.description",
    defaultPort: 9875,
    requirement: {
      runtime: "Python 3.10+",
      packages: ["paddleocr"],
      estimatedDownloadMb: 1,
      estimatedDiskMb: 1,
      models: [],
    },
  },
];

const statuses: ExtensionFrameworkStatus[] = [
  {
    frameworkId: "paddleocr",
    capabilityId: "ocr",
    status: "not-installed",
    installed: false,
    enabled: false,
    running: false,
    port: 9875,
    installPath: null,
    installedVersion: null,
    lastHealthCheck: null,
    lastError: null,
    lastOperationId: null,
  },
];

const translate = (key: string) => en[key as keyof typeof en] ?? key;

describe("ExtensionsPage", () => {
  it("filters by localized capability and package text", () => {
    expect(filterExtensionDefinitions(definitions, statuses, "OCR", translate)).toHaveLength(1);
    expect(filterExtensionDefinitions(definitions, statuses, "paddleocr", translate)).toHaveLength(1);
    expect(filterExtensionDefinitions(definitions, statuses, "speech synthesis", translate)).toHaveLength(0);
  });

  it("uses semantic styles without theme-name branches or direct Tauri calls", () => {
    // Task 12.18: the per-card markup (and its "ucd-panel" semantic-tone class) moved into its
    // own extension-framework-card.tsx as part of the shared-primitive migration -- read together
    // so this still proves the whole feature, not just whichever file happens to hold the article.
    const pageSource = readFileSync("src/settings/pages/extensions-page.tsx", "utf8");
    const cardSource = readFileSync("src/settings/pages/extensions/extension-framework-card.tsx", "utf8");
    const combined = `${pageSource}\n${cardSource}`;
    expect(combined).not.toContain("@tauri-apps/api");
    expect(combined).not.toContain("invoke(");
    expect(combined).not.toMatch(/theme\s*===/);
    expect(combined).toContain("ucd-panel");
  });

  it("reports a dependency-unavailable status for its nav entry when native operations are unavailable, and null once they are (task 12.16)", async () => {
    function environment(nativeOperationsAvailable: boolean): ExtensionEnvironment {
      return {
        runtime: "tauri",
        os: "windows",
        arch: "x86_64",
        supported: true,
        nativeOperationsAvailable,
        pythonPath: null,
        pythonVersion: null,
        reason: null,
      };
    }
    function serviceFor(nativeOperationsAvailable: boolean): ExtensionService {
      const overview: ExtensionOverview = { definitions: [], statuses: [], environment: environment(nativeOperationsAvailable) };
      return {
        async getOverview() { return overview; },
        async refreshHealth() { throw new Error("not used"); },
        async getInstallPreview() { throw new Error("not used"); },
        async install() { throw new Error("not used"); },
        async uninstall() { throw new Error("not used"); },
        async setEnabled() { throw new Error("not used"); },
        async start() { throw new Error("not used"); },
        async stop() { throw new Error("not used"); },
        async selfTest() { throw new Error("not used"); },
      };
    }
    function renderPage(nativeOperationsAvailable: boolean, onStatusChange: (status: unknown) => void) {
      const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
      return render(
        <QueryClientProvider client={queryClient}>
          <ExtensionsPage onStatusChange={onStatusChange} searchTerm="" service={serviceFor(nativeOperationsAvailable)} />
        </QueryClientProvider>,
      );
    }

    const unavailable = vi.fn();
    const { unmount } = renderPage(false, unavailable);
    await waitFor(() => expect(unavailable).toHaveBeenLastCalledWith({
      kind: "dependency-unavailable",
      labelKey: "extensions.status.nativeUnavailable",
    }));
    unmount();

    const healthy = vi.fn();
    renderPage(true, healthy);
    await waitFor(() => expect(healthy).toHaveBeenLastCalledWith(null));
  });
});

function buildDefinition(overrides: Partial<ExtensionFrameworkDefinition> = {}): ExtensionFrameworkDefinition {
  return {
    id: "paddleocr",
    capabilityId: "ocr",
    nameKey: "extensions.framework.paddleocr.name",
    descriptionKey: "extensions.framework.paddleocr.description",
    defaultPort: 9875,
    requirement: { runtime: "Python 3.10+", packages: ["paddleocr"], estimatedDownloadMb: 1, estimatedDiskMb: 1, models: [] },
    ...overrides,
  };
}

function buildStatus(overrides: Partial<ExtensionFrameworkStatus> = {}): ExtensionFrameworkStatus {
  return {
    frameworkId: "paddleocr",
    capabilityId: "ocr",
    status: "not-installed",
    installed: false,
    enabled: false,
    running: false,
    port: 9875,
    installPath: null,
    installedVersion: null,
    lastHealthCheck: null,
    lastError: null,
    lastOperationId: null,
    ...overrides,
  };
}

function buildEnvironment(overrides: Partial<ExtensionEnvironment> = {}): ExtensionEnvironment {
  return {
    runtime: "tauri",
    os: "windows",
    arch: "x86_64",
    supported: true,
    nativeOperationsAvailable: true,
    pythonPath: null,
    pythonVersion: null,
    reason: null,
    ...overrides,
  };
}

function buildService(overrides: Partial<ExtensionService> = {}): ExtensionService {
  return {
    getOverview: vi.fn(async () => ({ definitions: [buildDefinition()], statuses: [buildStatus()], environment: buildEnvironment() })),
    refreshHealth: vi.fn(async () => { throw new Error("not used"); }),
    getInstallPreview: vi.fn(async () => { throw new Error("not used"); }),
    install: vi.fn(async () => { throw new Error("not used"); }),
    uninstall: vi.fn(async () => { throw new Error("not used"); }),
    setEnabled: vi.fn(async () => { throw new Error("not used"); }),
    start: vi.fn(async () => { throw new Error("not used"); }),
    stop: vi.fn(async () => { throw new Error("not used"); }),
    selfTest: vi.fn(async () => { throw new Error("not used"); }),
    ...overrides,
  };
}

describe("ExtensionsPage (task 12.18 shared-primitive migration)", () => {
  beforeEach(async () => {
    await activateAppLanguage("zh-CN");
  });

  it("shows the page icon and a single primary Refresh action, with no More menu needed", async () => {
    renderWithAppProviders(<ExtensionsPage searchTerm="" service={buildService()} />);
    await screen.findByTestId("extension-card-paddleocr");

    expect(document.querySelector(".border-b.border-border-subtle")?.querySelector("svg")).toBeTruthy();
    expect(screen.getByRole("button", { name: "刷新" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "更多操作" })).toBeNull();
  });

  it("collapses per-card actions behind one ActionMenu instead of separate buttons", async () => {
    const svc = buildService({
      getOverview: vi.fn(async () => ({
        definitions: [buildDefinition()],
        statuses: [buildStatus({ status: "running", installed: true, running: true, enabled: true })],
        environment: buildEnvironment(),
      })),
    });
    const { user } = renderWithAppProviders(<ExtensionsPage searchTerm="" service={svc} />);
    const card = await screen.findByTestId("extension-card-paddleocr");

    expect(within(card).queryByRole("button", { name: "安装要求" })).toBeNull();
    expect(within(card).queryByRole("button", { name: "停止" })).toBeNull();
    expect(within(card).queryByRole("button", { name: "自检" })).toBeNull();
    expect(within(card).queryByRole("button", { name: "卸载" })).toBeNull();

    await user.click(within(card).getByRole("button", { name: "PaddleOCR的操作" }));
    expect(within(card).getByRole("menuitem", { name: "安装要求" })).toBeTruthy();
    expect(within(card).getByRole("menuitem", { name: "停止" })).toBeTruthy();
    expect(within(card).getByRole("menuitem", { name: "自检" })).toBeTruthy();
    expect(within(card).getByRole("menuitem", { name: "停用" })).toBeTruthy();
    expect(within(card).getByRole("menuitem", { name: "卸载" })).toBeTruthy();
  });

  it("shows pending, then clears, mutation status for an action triggered from the menu", async () => {
    let resolveStart: (value: OperationTask) => void = () => {};
    const svc = buildService({
      getOverview: vi.fn(async () => ({
        definitions: [buildDefinition()],
        statuses: [buildStatus({ status: "installed", installed: true, running: false, enabled: true })],
        environment: buildEnvironment(),
      })),
      start: vi.fn(() => new Promise<OperationTask>((resolve) => { resolveStart = resolve; })),
    });
    const { user } = renderWithAppProviders(<ExtensionsPage searchTerm="" service={svc} />);
    const card = await screen.findByTestId("extension-card-paddleocr");

    await user.click(within(card).getByRole("button", { name: "PaddleOCR的操作" }));
    await user.click(within(card).getByRole("menuitem", { name: "启动" }));

    expect(within(card).getByRole("status").textContent).toContain("保存中");
    resolveStart({
      id: "op-1",
      kind: "extension",
      status: "succeeded",
      relatedEntityId: "paddleocr",
      logs: [],
      createdAt: "2026-08-01T00:00:00Z",
      updatedAt: "2026-08-01T00:00:00Z",
    });
    await waitFor(() => expect(within(card).queryByRole("status")).toBeNull());
    expect(svc.start).toHaveBeenCalledWith({ frameworkId: "paddleocr" });
  });

  it("asks for confirmation before uninstalling, and does not call uninstall when cancelled", async () => {
    const svc = buildService({
      getOverview: vi.fn(async () => ({
        definitions: [buildDefinition()],
        statuses: [buildStatus({ status: "installed", installed: true, running: false, enabled: true })],
        environment: buildEnvironment(),
      })),
    });
    const { user } = renderWithAppProviders(<ExtensionsPage searchTerm="" service={svc} />);
    const card = await screen.findByTestId("extension-card-paddleocr");

    await user.click(within(card).getByRole("button", { name: "PaddleOCR的操作" }));
    await user.click(within(card).getByRole("menuitem", { name: "卸载" }));

    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "取消" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(svc.uninstall).not.toHaveBeenCalled();
  });

  it("renders the shared EmptyState when the search term matches nothing", async () => {
    renderWithAppProviders(<ExtensionsPage searchTerm="zzz-no-match" service={buildService()} />);

    await waitFor(() => expect(screen.getByText("没有匹配的扩展能力")).toBeTruthy());
    expect(screen.queryByRole("article")).toBeNull();
  });

  it("renders the shared AsyncBoundary error state with a working retry", async () => {
    const getOverview = vi.fn(async (): Promise<ExtensionOverview> => { throw new Error("network down"); });
    const svc = buildService({ getOverview });
    const { user } = renderWithAppProviders(<ExtensionsPage searchTerm="" service={svc} />);

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("network down"));
    getOverview.mockResolvedValueOnce({ definitions: [buildDefinition()], statuses: [buildStatus()], environment: buildEnvironment() });
    await user.click(screen.getByRole("button", { name: "重试" }));
    await screen.findByTestId("extension-card-paddleocr");
  });
});
