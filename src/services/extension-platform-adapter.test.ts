import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { beforeEach, describe, expect, it } from "vitest";
import type { ExtensionPlatformService } from "./extension-platform-service";
import { tauriExtensionPlatformClient } from "./tauri-extension-platform-client";
import {
  resetWebExtensionPlatformStateForTests,
  webExtensionPlatformClient,
} from "./web-extension-platform-client";

const currentDir = dirname(fileURLToPath(import.meta.url));

function methodNames(service: ExtensionPlatformService) {
  return Object.keys(service).sort();
}

describe("extension platform adapter parity", () => {
  beforeEach(() => {
    resetWebExtensionPlatformStateForTests();
  });

  it("keeps Tauri and Web mock adapter method shapes aligned", () => {
    expect(methodNames(tauriExtensionPlatformClient)).toEqual(
      methodNames(webExtensionPlatformClient),
    );
  });

  it("keeps the Web mock adapter free of Tauri imports", () => {
    const source = readFileSync(join(currentDir, "web-extension-platform-client.ts"), "utf8");

    expect(source).not.toContain("@tauri-apps/api");
    expect(source).not.toContain("invoke(");
  });

  it("reports its fixtures as a current read rather than degraded", async () => {
    const { freshness } = await webExtensionPlatformClient.getFeatureGates();

    expect(freshness).toEqual({ kind: "current" });
  });

  it("starts every gate disabled", async () => {
    const { gates } = await webExtensionPlatformClient.getFeatureGates();

    expect(gates).toHaveLength(7);
    for (const gate of gates) {
      expect(gate.desiredEnabled).toBe(false);
      expect(gate.status.kind).not.toBe("enabled");
      expect(gate.revision).toBe(0);
    }
  });

  it("reports runtime-bearing gates as not compiled rather than merely switched off", async () => {
    const { gates } = await webExtensionPlatformClient.getFeatureGates();
    const wasm = gates.find((gate) => gate.feature === "wasm_module_runtime");
    const sidecar = gates.find((gate) => gate.feature === "sidecar_runtime");

    expect(wasm?.status.kind).toBe("not_compiled");
    expect(wasm?.buildAvailable).toBe(false);
    expect(sidecar?.status.kind).toBe("not_compiled");
    expect(sidecar?.buildAvailable).toBe(false);
  });

  it("refuses to enable a gate the build cannot serve", async () => {
    await expect(
      webExtensionPlatformClient.setFeatureGate({
        feature: "sidecar_runtime",
        desiredEnabled: true,
        expectedRevision: 0,
      }),
    ).rejects.toThrow(/feature_unavailable_in_build/);

    const { gates } = await webExtensionPlatformClient.getFeatureGates();
    expect(gates.find((gate) => gate.feature === "sidecar_runtime")?.revision).toBe(0);
  });

  it("enables a compiled gate and advances its revision", async () => {
    const after = await webExtensionPlatformClient.setFeatureGate({
      feature: "catalog",
      desiredEnabled: true,
      expectedRevision: 0,
      reason: "gate 1",
    });

    const catalog = after.gates.find((gate) => gate.feature === "catalog");
    expect(catalog?.status.kind).toBe("enabled");
    expect(catalog?.revision).toBe(1);
    expect(catalog?.reason).toBe("gate 1");
  });

  it("rejects a stale revision without overwriting", async () => {
    await webExtensionPlatformClient.setFeatureGate({
      feature: "catalog",
      desiredEnabled: true,
      expectedRevision: 0,
    });

    await expect(
      webExtensionPlatformClient.setFeatureGate({
        feature: "catalog",
        desiredEnabled: false,
        expectedRevision: 0,
      }),
    ).rejects.toThrow(/stale_revision/);

    const { gates } = await webExtensionPlatformClient.getFeatureGates();
    expect(gates.find((gate) => gate.feature === "catalog")?.desiredEnabled).toBe(true);
  });
});
