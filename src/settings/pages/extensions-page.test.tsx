import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import en from "../../i18n/locales/en.json";
import type { ExtensionFrameworkDefinition, ExtensionFrameworkStatus } from "../../types/extension";
import { filterExtensionDefinitions } from "./extensions-page";

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
});
