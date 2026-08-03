// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import type { McpServerConfig } from "../../../types/mcp";
import { McpImportExportModal } from "./mcp-import-export";
import { McpServerCard } from "./mcp-server-card";
import { McpServerForm } from "./mcp-server-form";
import { McpTestResultPanel } from "./mcp-test-result";

const legacyServer: McpServerConfig = {
  name: "legacy-events",
  transportType: "sse",
  url: "https://example.test/events",
  active: true,
  scope: "user",
};

const streamableServer: McpServerConfig = {
  ...legacyServer,
  name: "modern-http",
  transportType: "streamable_http",
  url: "https://example.test/mcp",
};

describe("MCP user-visible behavior", () => {
  beforeEach(async () => {
    await activateAppLanguage("zh-CN");
  });

  it("labels legacy SSE and Streamable HTTP distinctly in forms and cards", () => {
    const callbacks = {
      onDelete: vi.fn(),
      onEdit: vi.fn(),
      onTest: vi.fn(),
      onToggle: vi.fn(),
    };
    const form = render(<McpServerForm onCancel={vi.fn()} onSave={vi.fn()} />);

    expect(screen.getByRole("option", { name: "stdio（本地进程）" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "旧版 SSE" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "Streamable HTTP" })).toBeTruthy();

    form.unmount();
    render(<McpServerCard {...callbacks} server={legacyServer} testing={false} />);
    expect(screen.getByText("旧版 SSE")).toBeTruthy();
    expect(screen.queryByText("Streamable HTTP")).toBeNull();
  });

  it("renders classified failures and hides unclassified details", () => {
    const view = render(
      <McpTestResultPanel
        status={{
          name: "modern-http",
          connectionStatus: "error",
          tools: [],
          errorCode: "limit_exceeded",
          error: "工具目录超过安全限制。",
        }}
      />,
    );

    expect(screen.getByText(/超出资源限制 \[limit_exceeded\]/)).toBeTruthy();
    expect(screen.getByText(/工具目录超过安全限制/)).toBeTruthy();

    view.rerender(
      <McpTestResultPanel
        status={{
          name: "modern-http",
          connectionStatus: "error",
          tools: [],
          error: "Authorization: secret-token",
        }}
      />,
    );
    expect(screen.getByText("MCP 操作失败，当前没有可安全显示的错误详情。")).toBeTruthy();
    expect(screen.queryByText(/secret-token/)).toBeNull();
  });

  it("presents imported, skipped, validation, and storage outcomes separately", async () => {
    const user = userEvent.setup();
    const onImport = vi.fn().mockResolvedValue({
      imported: ["modern-http"],
      skipped: ["existing-server"],
      failures: [
        {
          name: "Bad_Name",
          stage: "validation",
          errorCode: "validation",
          message: "名称无效。",
        },
        {
          name: "storage-failure",
          stage: "storage",
          errorCode: null,
          message: "服务器配置无法保存。",
        },
      ],
    });
    render(
      <McpImportExportModal
        servers={[legacyServer, streamableServer]}
        onCancel={vi.fn()}
        onExport={vi.fn()}
        onImport={onImport}
      />,
    );

    await user.click(screen.getByRole("button", { name: "确认导入" }));

    expect(await screen.findByText("已导入 1 个，跳过 1 个，失败 2 个")).toBeTruthy();
    const failures = screen.getAllByRole("listitem").map((item) => item.textContent);
    expect(failures).toEqual([
      expect.stringMatching(/Bad_Name.*校验.*\[validation\]/),
      expect.stringMatching(/storage-failure.*存储.*服务器配置无法保存/),
    ]);
  });

  it("shows transport semantics alongside export selections", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn().mockResolvedValue({
      mcpServers: {
        "legacy-events": { type: "sse", url: legacyServer.url },
        "modern-http": { type: "http", url: streamableServer.url },
      },
    });
    render(
      <McpImportExportModal
        servers={[legacyServer, streamableServer]}
        onCancel={vi.fn()}
        onExport={onExport}
        onImport={vi.fn()}
      />,
    );

    await user.click(screen.getByRole("button", { name: "导出" }));
    expect(screen.getByText("旧版 SSE")).toBeTruthy();
    expect(screen.getByText("Streamable HTTP")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "生成 JSON" }));

    expect(onExport).toHaveBeenCalledWith(["legacy-events", "modern-http"]);
    const output = screen.getByRole("textbox") as HTMLTextAreaElement;
    expect(output.value).toContain('"type": "sse"');
    expect(output.value).toContain('"type": "http"');
  });
});
