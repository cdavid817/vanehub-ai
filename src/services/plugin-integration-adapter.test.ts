import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import type { PluginIntegrationService } from "./plugin-integration-service";
import { tauriPluginIntegrationClient } from "./tauri-plugin-integration-client";
import { webPluginIntegrationClient } from "./web-plugin-integration-client";

const currentDir = dirname(fileURLToPath(import.meta.url));

function methodNames(service: PluginIntegrationService) {
  return Object.keys(service).sort();
}

describe("plugin integration adapter parity", () => {
  it("keeps Tauri and Web mock adapter method shapes aligned", () => {
    expect(methodNames(tauriPluginIntegrationClient)).toEqual(methodNames(webPluginIntegrationClient));
  });

  it("keeps the Web mock adapter free of Tauri imports", () => {
    const source = readFileSync(join(currentDir, "web-plugin-integration-client.ts"), "utf8");

    expect(source).not.toContain("@tauri-apps/api");
    expect(source).not.toContain("invoke(");
  });
});
