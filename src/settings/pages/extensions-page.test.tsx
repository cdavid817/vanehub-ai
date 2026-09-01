// @vitest-environment jsdom

import { readFileSync } from "node:fs";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../../i18n";
import en from "../../i18n/locales/en.json";
import type { ExtensionService } from "../../services/extension-service";
import type { ExtensionEnvironment, ExtensionFrameworkDefinition, ExtensionFrameworkStatus, ExtensionOverview } from "../../types/extension";
import { ExtensionsPage, filterExtensionDefinitions } from "./extensions-page";

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
    const source = readFileSync("src/settings/pages/extensions-page.tsx", "utf8");
    expect(source).not.toContain("@tauri-apps/api");
    expect(source).not.toContain("invoke(");
    expect(source).not.toMatch(/theme\s*===/);
    expect(source).toContain("ucd-panel");
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
