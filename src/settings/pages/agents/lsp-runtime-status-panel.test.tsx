// @vitest-environment jsdom

import { act, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { LspServerStatus } from "../../../types/lsp";
import {
  LspRuntimeStatusPanel,
  lspStatusPollingIntervalMs,
} from "./lsp-runtime-status-panel";

const statuses: LspServerStatus[] = [
  {
    language: "rust",
    server: "rust_analyzer",
    relativeProjectRoot: "crates/api",
    state: "ready",
    restartCount: 2,
    lastResponseAt: "2026-01-02T03:04:05Z",
    diagnosticCount: 3,
    reasonCode: null,
    negotiatedCapabilities: {
      positionEncoding: "utf16",
      documentSync: "incremental",
      definition: true,
      references: true,
      hover: false,
      diagnostics: true,
    },
  },
  {
    language: "typescript_javascript",
    server: "typescript_language_server",
    relativeProjectRoot: ".",
    state: "backoff",
    restartCount: 4,
    lastResponseAt: null,
    diagnosticCount: 0,
    reasonCode: "restart_exhausted",
    negotiatedCapabilities: null,
  },
];

describe("LspRuntimeStatusPanel", () => {
  beforeEach(async () => activateAppLanguage("zh-CN"));
  afterEach(() => vi.useRealTimers());

  it("renders safe runtime identity, lifecycle metrics, and negotiated capabilities", async () => {
    const getLspServerStatus = vi.fn(async () => statuses);
    const service = createAgentServiceDouble({ getLspServerStatus });
    renderWithAppProviders(<LspRuntimeStatusPanel service={service} />);

    const rust = await screen.findByRole("article", { name: "Rust rust_analyzer" });
    expect(within(rust).getByText("就绪")).toBeTruthy();
    expect(within(rust).getByText("crates/api")).toBeTruthy();
    expect(within(rust).getByText("2")).toBeTruthy();
    expect(within(rust).getByText("3")).toBeTruthy();
    expect(within(rust).getByText("UTF-16")).toBeTruthy();
    expect(within(rust).getByText("incremental")).toBeTruthy();
    expect(within(rust).getAllByText("支持")).toHaveLength(3);
    expect(within(rust).getByText("不支持")).toBeTruthy();

    const typescript = screen.getByRole("article", {
      name: "TypeScript / JavaScript typescript_language_server",
    });
    expect(within(typescript).getByText("等待重启")).toBeTruthy();
    expect(within(typescript).getByText("尚无响应")).toBeTruthy();
    expect(within(typescript).getByText("已达到自动重启次数上限。")).toBeTruthy();
    expect(screen.getByText(/不提供可移植的内存占用和已索引文件数量指标/)).toBeTruthy();
    expect(getLspServerStatus).toHaveBeenCalledOnce();
  });

  it("renders the empty runtime state", async () => {
    const service = createAgentServiceDouble({ getLspServerStatus: async () => [] });
    renderWithAppProviders(<LspRuntimeStatusPanel service={service} />);

    expect(await screen.findByText("当前没有活动的语言服务器实例。")).toBeTruthy();
  });

  it("polls while mounted and stops polling after unmount", async () => {
    vi.useFakeTimers();
    const getLspServerStatus = vi.fn(async () => []);
    const service = createAgentServiceDouble({ getLspServerStatus });
    const { unmount } = renderWithAppProviders(<LspRuntimeStatusPanel service={service} />);

    await act(async () => { await Promise.resolve(); });
    expect(getLspServerStatus).toHaveBeenCalledTimes(1);
    await act(async () => {
      vi.advanceTimersByTime(lspStatusPollingIntervalMs);
      await Promise.resolve();
    });
    expect(getLspServerStatus).toHaveBeenCalledTimes(2);

    unmount();
    await act(async () => {
      vi.advanceTimersByTime(lspStatusPollingIntervalMs * 2);
      await Promise.resolve();
    });
    expect(getLspServerStatus).toHaveBeenCalledTimes(2);
  });

  it("hides rejected service details and retries status loading", async () => {
    const getLspServerStatus = vi.fn()
      .mockRejectedValueOnce(new Error("C:/private/workspace"))
      .mockResolvedValueOnce([]);
    const service = createAgentServiceDouble({ getLspServerStatus });
    const { user } = renderWithAppProviders(<LspRuntimeStatusPanel service={service} />);

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("无法加载语言服务器设置。");
    expect(alert.textContent).not.toContain("private");
    await user.click(screen.getByRole("button", { name: "重试" }));

    await waitFor(() => expect(getLspServerStatus).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("当前没有活动的语言服务器实例。")).toBeTruthy();
  });
});
