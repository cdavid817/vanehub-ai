import { describe, expect, it } from "vitest";
import type { CliToolStatus } from "../../types/agent";
import { compareStableVersions, deriveCliVersionAction } from "./cli-management-utils";

const baseTool: CliToolStatus = {
  agentId: "codex-cli",
  displayName: "OpenAI Codex CLI",
  provider: "OpenAI",
  executableName: "codex",
  packageName: "@openai/codex",
  installed: true,
  currentVersion: "1.2.0",
  latestVersion: "1.3.0",
  availableVersions: ["1.3.0", "1.2.0", "1.1.0"],
  detectedPath: "C:\\Users\\dev\\codex.cmd",
  installCommand: "npm install -g @openai/codex@latest",
  lastCheckedAt: "123",
  lastError: null,
  lastOperationId: null,
  versionCheckStatus: "succeeded",
  environmentType: "windows",
  installations: [{
    path: "C:\\Users\\dev\\codex.cmd",
    version: "1.2.0",
    runnable: true,
    error: null,
    source: "npm",
    environmentType: "windows",
    isActive: true,
  }],
  activeInstallationPath: "C:\\Users\\dev\\codex.cmd",
  conflictState: "none",
  lifecycleEligibility: "npm",
};

const cliExamples: Array<Pick<CliToolStatus, "agentId" | "displayName" | "provider" | "executableName" | "packageName">> = [
  {
    agentId: "claude-code",
    displayName: "Anthropic Claude Code CLI",
    provider: "Anthropic",
    executableName: "claude",
    packageName: "@anthropic-ai/claude-code",
  },
  {
    agentId: "opencode",
    displayName: "OpenCode CLI",
    provider: "OpenCode",
    executableName: "opencode",
    packageName: "opencode-ai",
  },
  {
    agentId: "codex-cli",
    displayName: "OpenAI Codex CLI",
    provider: "OpenAI",
    executableName: "codex",
    packageName: "@openai/codex",
  },
  {
    agentId: "gemini-cli",
    displayName: "Google Gemini CLI",
    provider: "Google",
    executableName: "gemini",
    packageName: "@google/gemini-cli",
  },
];

describe("CLI management utilities", () => {
  it("compares stable versions", () => {
    expect(compareStableVersions("1.3.0", "1.2.9")).toBe(1);
    expect(compareStableVersions("v1.2.0", "1.2")).toBe(0);
    expect(compareStableVersions("1.1.9", "1.2.0")).toBe(-1);
    expect(compareStableVersions("1.2.0-beta.1", "1.2.0")).toBeNull();
  });

  it("derives install, upgrade, downgrade, and current actions", () => {
    expect(deriveCliVersionAction({ ...baseTool, installed: false, currentVersion: null }, "1.3.0")).toBe("install");
    expect(deriveCliVersionAction(baseTool, "1.3.0")).toBe("upgrade");
    expect(deriveCliVersionAction(baseTool, "1.1.0")).toBe("downgrade");
    expect(deriveCliVersionAction(baseTool, "1.2.0")).toBe("current");
    expect(deriveCliVersionAction(baseTool, null)).toBe("unavailable");
  });

  it("derives selected-version actions consistently for every managed CLI", () => {
    for (const cli of cliExamples) {
      const tool = { ...baseTool, ...cli };
      expect(deriveCliVersionAction({ ...tool, installed: false, currentVersion: null }, "1.3.0")).toBe("install");
      expect(deriveCliVersionAction(tool, "1.3.0")).toBe("upgrade");
      expect(deriveCliVersionAction(tool, "1.1.0")).toBe("downgrade");
    }
  });

  it("uses manual guidance for source-native active installations", () => {
    expect(deriveCliVersionAction({ ...baseTool, lifecycleEligibility: "manual" }, "1.3.0")).toBe("manual");
  });
});
