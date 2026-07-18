import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../../i18n";
import type { CliToolStatus } from "../../types/agent";
import type { OperationTask } from "../../types/operation";
import { ProvidersPage, refreshButtonState, resolveCliPackageActionTargetVersion } from "./providers-page";
import { isBulkCliUpgradeEligible } from "./cli-management-utils";

const cliTool: CliToolStatus = {
  agentId: "claude-code",
  displayName: "Anthropic Claude Code CLI",
  provider: "Anthropic",
  executableName: "claude",
  packageName: "@anthropic-ai/claude-code",
  installed: true,
  currentVersion: "1.2.0",
  latestVersion: "1.3.0",
  availableVersions: ["1.3.0", "1.2.0"],
  detectedPath: "C:\\Users\\dev\\claude.cmd",
  installCommand: "npm install -g @anthropic-ai/claude-code@latest",
  lastCheckedAt: "123",
  lastError: null,
  lastOperationId: "op-1",
  versionCheckStatus: "succeeded",
  environmentType: "windows",
  installations: [{
    path: "C:\\Users\\dev\\claude.cmd",
    version: "1.2.0",
    runnable: true,
    error: null,
    source: "npm",
    environmentType: "windows",
    isActive: true,
  }],
  activeInstallationPath: "C:\\Users\\dev\\claude.cmd",
  conflictState: "none",
  lifecycleEligibility: "npm",
};

const operation: OperationTask = {
  id: "op-1",
  kind: "agent",
  status: "succeeded",
  relatedEntityId: "claude-code",
  message: "Installed",
  logs: [{ operationId: "op-1", line: "npm install complete", timestamp: "123" }],
  result: null,
  error: null,
  createdAt: "123",
  updatedAt: "124",
};

describe("ProvidersPage CLI management rendering", () => {
  it("renders cached CLI status and card-local operation state", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["cli-tools"], [cliTool]);
    queryClient.setQueryData(["operation", "op-1"], operation);

    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <ProvidersPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain("CLI 管理");
    expect(html).toContain("本地环境检查");
    expect(html).toContain("诊断安装冲突");
    expect(html).toContain("全部升级 1");
    expect(html).toContain("Anthropic Claude Code CLI");
    expect(html).toContain("1.3.0");
    expect(html).toContain("C:\\Users\\dev\\claude.cmd");
    expect(html).toContain("最近操作");
    expect(html).toContain("已成功");
    expect(html).toContain("当前生效路径");
    expect(html).toContain("Windows");
    expect(html).not.toContain("复制安装命令");
  });

  it("derives refresh button loading state from mutation or operation status", () => {
    expect(refreshButtonState(true, undefined)).toMatchObject({
      disabled: true,
      labelKey: "cli.refreshing",
    });
    expect(refreshButtonState(false, { ...operation, status: "queued" })).toMatchObject({
      disabled: true,
      labelKey: "cli.refreshing",
    });
    expect(refreshButtonState(false, { ...operation, status: "running" }).iconClassName).toContain("animate-spin");
    expect(refreshButtonState(false, { ...operation, status: "failed" })).toMatchObject({
      disabled: false,
      labelKey: "cli.refresh",
    });
  });

  it("renders cause-specific source-native guidance instead of generic manual warning", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["cli-tools"], [{
      ...cliTool,
      lifecycleEligibility: "manual",
      installations: [{ ...cliTool.installations[0], source: "homebrew" }],
    }]);

    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <ProvidersPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain("当前生效路径来自 Homebrew");
    expect(html).not.toContain("当前生效的 CLI 不是可安全管理的 npm 安装");
  });

  it("renders one-click upgrade when no latest version was resolved", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["cli-tools"], [{
      ...cliTool,
      latestVersion: null,
      availableVersions: [],
    }]);

    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <ProvidersPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain(">升级<");
  });

  it("renders one-click upgrade when installed CLI is already current", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["cli-tools"], [{
      ...cliTool,
      currentVersion: "1.3.0",
      latestVersion: "1.3.0",
      availableVersions: ["1.3.0"],
    }]);

    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <ProvidersPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain(">升级<");
    expect(html).not.toContain("当前版本</span>");
  });

  it("keeps disabled upgrade visible when installed CLI is not backend-managed", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["cli-tools"], [{
      ...cliTool,
      lifecycleEligibility: "manual",
      latestVersion: "1.2.0",
      installations: [{ ...cliTool.installations[0], source: "system" }],
    }]);

    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <ProvidersPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain(">升级<");
    expect(html).toContain("disabled");
  });

  it("keeps disabled upgrade visible when installed CLI lifecycle is unavailable", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["cli-tools"], [{
      ...cliTool,
      lifecycleEligibility: "unavailable",
      latestVersion: null,
    }]);

    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <ProvidersPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain(">升级<");
    expect(html).toContain("disabled");
  });

  it("renders enabled upgrade for WinGet-managed installed CLI", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["cli-tools"], [{
      ...cliTool,
      lifecycleEligibility: "winget",
      installations: [{ ...cliTool.installations[0], source: "winget" }],
    }]);

    const html = renderToString(
      <QueryClientProvider client={queryClient}>
        <ProvidersPage searchTerm="" />
      </QueryClientProvider>,
    );

    expect(html).toContain(">升级<");
    expect(html).not.toContain("当前生效路径来自 WinGet");
  });

  it("enables bulk upgrade eligibility only for safe backend-managed upgrades", () => {
    expect(isBulkCliUpgradeEligible(cliTool)).toBe(true);
    expect(isBulkCliUpgradeEligible({
      ...cliTool,
      lifecycleEligibility: "wget",
      installations: [{ ...cliTool.installations[0], source: "vendor" }],
    })).toBe(true);
    expect(isBulkCliUpgradeEligible({
      ...cliTool,
      lifecycleEligibility: "winget",
      installations: [{ ...cliTool.installations[0], source: "winget" }],
    })).toBe(true);
    expect(isBulkCliUpgradeEligible({ ...cliTool, latestVersion: "1.2.0" })).toBe(false);
    expect(isBulkCliUpgradeEligible({ ...cliTool, lifecycleEligibility: "manual" })).toBe(false);
    expect(isBulkCliUpgradeEligible({
      ...cliTool,
      installations: [
        ...cliTool.installations,
        { ...cliTool.installations[0], path: "C:\\Tools\\claude.cmd", isActive: false },
      ],
    })).toBe(false);
  });

  it("targets latest for one-click package actions instead of copying install commands", () => {
    expect(resolveCliPackageActionTargetVersion(cliTool)).toBe("1.3.0");
    expect(resolveCliPackageActionTargetVersion({ ...cliTool, latestVersion: null })).toBe("latest");
  });
});
