import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { webWorkspaceClient } from "./web-workspace-client";

const currentDir = dirname(fileURLToPath(import.meta.url));

describe("web workspace client", () => {
  it("returns a browser-compatible workspace snapshot", async () => {
    const snapshot = await webWorkspaceClient.getWorkspaceSnapshot();

    expect(snapshot.conversations.length).toBeGreaterThan(0);
    expect(snapshot.agentNodes.length).toBeGreaterThan(0);
  });

  it("does not import Tauri APIs", () => {
    const source = readFileSync(join(currentDir, "web-workspace-client.ts"), "utf8");

    expect(source).not.toContain("@tauri-apps/api");
    expect(source).not.toContain("invoke(");
  });
});
