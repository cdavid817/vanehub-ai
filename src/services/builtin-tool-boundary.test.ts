import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import type { AgentService } from "./agent-service";
import { tauriRuntimeAgentClient } from "./runtime-agent-client";
import { webAgentClient } from "./web-agent-client";

const currentDir = dirname(fileURLToPath(import.meta.url));
const componentDir = join(currentDir, "..", "session-workspace");
const componentFiles = [
  "artifact-panel.tsx",
  "browser-handoff-control.tsx",
  "builtin-tool-activity.tsx",
  "delegation-panel.tsx",
];

describe("built-in tool service boundary", () => {
  it("keeps React surfaces behind AgentService", () => {
    for (const file of componentFiles) {
      const source = readFileSync(join(componentDir, file), "utf8");
      expect(source, file).not.toContain("@tauri-apps/");
      expect(source, file).not.toMatch(/\binvoke\s*\(/);
      expect(source, file).toContain("AgentService");
    }
  });

  it("keeps desktop and Web adapters assignable to the same contract", () => {
    const adapters: AgentService[] = [tauriRuntimeAgentClient, webAgentClient];

    for (const adapter of adapters) {
      expect(typeof adapter.getBuiltinToolReadiness).toBe("function");
      expect(typeof adapter.getArtifact).toBe("function");
      expect(typeof adapter.startDelegation).toBe("function");
      expect(typeof adapter.applyDelegationChanges).toBe("function");
      expect(typeof adapter.resumeBrowserAutomation).toBe("function");
    }
  });
});
