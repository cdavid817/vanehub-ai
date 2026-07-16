import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../../i18n";
import type { CliToolStatus } from "../../types/agent";
import type { OperationTask } from "../../types/operation";
import { ProvidersPage } from "./providers-page";

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
    expect(html).toContain("Anthropic Claude Code CLI");
    expect(html).toContain("1.3.0");
    expect(html).toContain("C:\\Users\\dev\\claude.cmd");
    expect(html).toContain("最近操作");
    expect(html).toContain("已成功");
  });
});
