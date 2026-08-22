import { i18n } from "../i18n";
import type { CliToolService } from "./cli-service";
import { createWebMockOperation } from "./web-operation-client";
import { nowIso } from "./web-mock-clock";
import type { CliToolStatus } from "../types/agent";
import type { OperationTask } from "../types/operation";

export function webLocalCliDetectionMessage() {
  return i18n.t("web.error.localCliDetection");
}

export function webCliPackageOperationsMessage() {
  return i18n.t("web.error.cliPackageOperations");
}

const webCliTools: CliToolStatus[] = [
  {
    agentId: "claude-code",
    displayName: "Anthropic Claude Code CLI",
    provider: "Anthropic",
    executableName: "claude",
    packageName: "@anthropic-ai/claude-code",
    installed: null,
    currentVersion: null,
    latestVersion: null,
    availableVersions: [],
    detectedPath: null,
    installCommand: "bash -lc 'tmp=$(mktemp) && wget -qO \"$tmp\" https://claude.ai/install.sh && bash \"$tmp\"; status=$?; rm -f \"$tmp\"; exit $status' || npm install -g @anthropic-ai/claude-code@latest",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
    environmentType: "unknown",
    installations: [],
    activeInstallationPath: null,
    conflictState: "none",
    lifecycleEligibility: "unavailable",
  },
  {
    agentId: "codex-cli",
    displayName: "OpenAI Codex CLI",
    provider: "OpenAI",
    executableName: "codex",
    packageName: "@openai/codex",
    installed: null,
    currentVersion: null,
    latestVersion: null,
    availableVersions: [],
    detectedPath: null,
    installCommand: "npm install -g @openai/codex@latest",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
    environmentType: "unknown",
    installations: [],
    activeInstallationPath: null,
    conflictState: "none",
    lifecycleEligibility: "unavailable",
  },
  {
    agentId: "gemini-cli",
    displayName: "Google Gemini CLI",
    provider: "Google",
    executableName: "gemini",
    packageName: "@google/gemini-cli",
    installed: null,
    currentVersion: null,
    latestVersion: null,
    availableVersions: [],
    detectedPath: null,
    installCommand: "npm install -g @google/gemini-cli@latest",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
    environmentType: "unknown",
    installations: [],
    activeInstallationPath: null,
    conflictState: "none",
    lifecycleEligibility: "unavailable",
  },
  {
    agentId: "opencode",
    displayName: "OpenCode CLI",
    provider: "OpenCode",
    executableName: "opencode",
    packageName: "opencode-ai",
    installed: null,
    currentVersion: null,
    latestVersion: null,
    availableVersions: [],
    detectedPath: null,
    installCommand: "bash -lc 'tmp=$(mktemp) && wget -qO \"$tmp\" https://opencode.ai/install && bash \"$tmp\"; status=$?; rm -f \"$tmp\"; exit $status' || npm install -g opencode-ai@latest",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
    environmentType: "unknown",
    installations: [],
    activeInstallationPath: null,
    conflictState: "none",
    lifecycleEligibility: "unavailable",
  },
  {
    agentId: "antigravity-cli",
    displayName: "Google Antigravity CLI",
    provider: "Google",
    executableName: "agy",
    // Distributed only by installer script, so there is no package to name.
    packageName: null,
    installed: null,
    currentVersion: null,
    latestVersion: null,
    availableVersions: [],
    detectedPath: null,
    installCommand: "bash -lc 'tmp=$(mktemp) && wget -qO \"$tmp\" https://antigravity.google/cli/install.sh && bash \"$tmp\"; status=$?; rm -f \"$tmp\"; exit $status'",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
    environmentType: "unknown",
    installations: [],
    activeInstallationPath: null,
    conflictState: "none",
    lifecycleEligibility: "unavailable",
  },
];

export const webCliToolClient: CliToolService = {
  async listCliTools() {
    return webCliTools.map((tool) => ({
      ...tool,
      availableVersions: [...tool.availableVersions],
      installations: tool.installations.map((installation) => ({ ...installation })),
      lastError: webLocalCliDetectionMessage(),
    }));
  },

  async refreshCliDetections(agentId?: string): Promise<OperationTask> {
    const timestamp = nowIso();
    const message = webLocalCliDetectionMessage();
    const operationId = `web-cli-refresh-${timestamp}`;
    return createWebMockOperation({
      id: operationId,
      kind: "cli",
      relatedEntityId: agentId ?? null,
      message,
      terminalStatus: "failed",
      error: message,
      result: { agentIds: agentId ? [agentId] : webCliTools.map((tool) => tool.agentId) },
    });
  },

  async installCliVersion(input): Promise<OperationTask> {
    const timestamp = nowIso();
    const message = webCliPackageOperationsMessage();
    const operationId = `web-cli-install-${input.agentId}-${timestamp}`;
    return createWebMockOperation({
      id: operationId,
      kind: "cli",
      relatedEntityId: input.agentId,
      message,
      terminalStatus: "failed",
      error: message,
      result: { agentId: input.agentId, targetVersion: input.targetVersion },
    });
  },

  async upgradeAllCliVersions(): Promise<OperationTask> {
    const timestamp = nowIso();
    const message = webCliPackageOperationsMessage();
    return createWebMockOperation({
      id: `web-cli-upgrade-all-${timestamp}`,
      kind: "cli",
      relatedEntityId: null,
      message,
      terminalStatus: "failed",
      error: message,
      result: { agentIds: webCliTools.map((tool) => tool.agentId) },
    });
  },
};
