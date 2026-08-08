// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { CodeIndexPhase, CodeIndexWorkspace } from "../../../types/code-index";
import { resetWebRetrievalForTest, webAgentClient } from "../../../services/web-agent-client";
import { CodeIndexManagementSection } from "./code-index-management-section";

function workspace(phase: CodeIndexPhase = "awaiting_embedding_confirmation"): CodeIndexWorkspace {
  return {
    workspaceId: "workspace-stable-a",
    canonicalRoot: "D:/code/app",
    displayName: "App",
    enabled: true,
    selectedRoots: ["src"],
    languages: ["rust"],
    exclusionPatterns: ["dist/**"],
    maxFileBytes: 102_400,
    indexVersion: "1",
    generation: 4,
    status: {
      phase,
      totalFiles: 18,
      processedFiles: 16,
      failedFiles: 1,
      totalChunks: 54,
      processedChunks: 20,
      pendingChunks: 34,
      indexedChunks: 19,
      failedChunks: 1,
      redactionCount: 3,
      estimatedEmbeddingRequests: 2,
      lastFailureCategory: null,
      updatedAt: "2026-08-08T08:00:00Z",
    },
  };
}

afterEach(() => {
  resetWebRetrievalForTest();
  vi.restoreAllMocks();
});

describe("CodeIndexManagementSection", () => {
  it("shows stable identity, unavailable root state, progress, failures, and redactions", async () => {
    const unavailable = workspace("unavailable");
    const service = createAgentServiceDouble({
      listCodeIndexWorkspaces: async () => [unavailable],
      getRetrievalConfiguration: async () => ({ sourceProfileId: null, embeddingModel: null }),
    });
    renderWithAppProviders(<CodeIndexManagementSection service={service} />);

    expect(await screen.findByText("workspace-stable-a")).toBeTruthy();
    expect(screen.getByText("D:/code/app")).toBeTruthy();
    expect(screen.getByText("根目录不可用")).toBeTruthy();
    expect(screen.getByText("16/18")).toBeTruthy();
    expect(screen.getByText("20/54")).toBeTruthy();
    expect(screen.getByText("3")).toBeTruthy();
  });

  it("blocks an enabled configuration with no selected language", async () => {
    const saveCodeIndexConfiguration = vi.fn();
    const service = createAgentServiceDouble({
      listCodeIndexWorkspaces: async () => [workspace()],
      getRetrievalConfiguration: async () => ({ sourceProfileId: null, embeddingModel: null }),
      saveCodeIndexConfiguration,
    });
    const { user } = renderWithAppProviders(<CodeIndexManagementSection service={service} />);

    await user.click(await screen.findByRole("button", { name: "配置代码索引" }));
    const dialog = screen.getByRole("dialog", { name: "配置 App" });
    await user.click(within(dialog).getByRole("checkbox", { name: "Rust" }));
    await user.click(within(dialog).getByRole("button", { name: "保存配置" }));

    expect((await within(dialog).findByRole("alert")).textContent).toContain("enabled indexes need a language");
    expect(saveCodeIndexConfiguration).not.toHaveBeenCalled();
  });

  it("requires an explicit privacy acknowledgement before confirming embedding", async () => {
    const confirmCodeIndexEmbedding = vi.fn(async () => ({ profileId: "profile-a", model: "model-a", generation: 4 }));
    const service = createAgentServiceDouble({
      listCodeIndexWorkspaces: async () => [workspace()],
      getRetrievalConfiguration: async () => ({ sourceProfileId: "profile-a", embeddingModel: "model-a" }),
      confirmCodeIndexEmbedding,
    });
    const { user } = renderWithAppProviders(<CodeIndexManagementSection service={service} />);

    await user.click(await screen.findByRole("button", { name: "查看并确认" }));
    const dialog = screen.getByRole("dialog", { name: "确认代码 Embedding" });
    const confirm = within(dialog).getByRole("button", { name: "确认并开始" });
    expect((confirm as HTMLButtonElement).disabled).toBe(true);
    await user.click(within(dialog).getByRole("checkbox"));
    await user.click(confirm);

    await waitFor(() => expect(confirmCodeIndexEmbedding).toHaveBeenCalledWith(
      "workspace-stable-a", "profile-a", "model-a", 4,
    ));
  });

  it("renders deterministic Web phases and keeps other workspaces when one is deleted", async () => {
    const first = await webAgentClient.registerCodeIndexWorkspace("D:/code/first", "First");
    const second = await webAgentClient.registerCodeIndexWorkspace("D:/code/second", "Second");
    await webAgentClient.saveCodeIndexConfiguration(first.workspaceId, {
      enabled: true,
      selectedRoots: ["src"],
      languages: ["typescript"],
      exclusionPatterns: [],
      maxFileBytes: 102_400,
    });
    const { user } = renderWithAppProviders(<CodeIndexManagementSection service={webAgentClient} />);

    const firstRow = (await screen.findByText("First")).closest("article");
    expect(firstRow).not.toBeNull();
    await user.click(within(firstRow as HTMLElement).getByRole("button", { name: "刷新文件清单" }));
    await waitFor(() => expect(within(firstRow as HTMLElement).getByText("解析中")).toBeTruthy());

    await user.click(within(firstRow as HTMLElement).getByRole("button", { name: "重建代码索引" }));
    await user.click(within(await screen.findByRole("dialog", { name: "重建代码索引" })).getByRole("button", { name: "确认重建" }));
    await waitFor(() => expect(screen.getByText("扫描中")).toBeTruthy());

    await user.click(within(firstRow as HTMLElement).getByRole("button", { name: "删除代码索引" }));
    await user.click(within(await screen.findByRole("dialog", { name: "删除代码索引" })).getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(screen.queryByText("First")).toBeNull());
    expect(screen.getByText(second.displayName)).toBeTruthy();
  });
});
