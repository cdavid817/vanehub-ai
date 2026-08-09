// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createAgentServiceDouble, renderWithAppProviders } from "../test/render";
import type { CodeIndexPhase, CodeIndexWorkspace } from "../types/code-index";
import { SessionCodeIndexPane } from "./session-code-index-pane";

function workspace(phase: CodeIndexPhase = "awaiting_embedding_confirmation"): CodeIndexWorkspace {
  return {
    workspaceId: "workspace-stable-a",
    canonicalRoot: "D:/code/app",
    displayName: "App",
    origin: "automatic",
    enabled: true,
    mode: "semantic",
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

describe("SessionCodeIndexPane", () => {
  it("shows only the workspace bound to the active session path", async () => {
    const service = createAgentServiceDouble({
      listCodeIndexWorkspaces: async () => [{ ...workspace(), canonicalRoot: "D:/code/other", displayName: "Other" }, workspace()],
      getRetrievalConfiguration: async () => ({ sourceProfileId: null, embeddingModel: null, automaticCodeIndexMode: "disabled" }),
    });
    renderWithAppProviders(<SessionCodeIndexPane service={service} workspacePath={"D:\\code\\app\\"} />);

    expect(await screen.findByText("D:/code/app")).toBeTruthy();
    expect(screen.queryByText("D:/code/other")).toBeNull();
    expect(screen.getByText("16/18")).toBeTruthy();
    expect(screen.getByText("20/54")).toBeTruthy();
    expect(screen.getByText("脱敏次数").nextSibling?.textContent).toBe("3");
  });

  it("configures the current workspace without exposing a global workspace picker", async () => {
    const saveCodeIndexConfiguration = vi.fn(async (_workspaceId, configuration) => ({ ...workspace(), ...configuration }));
    const service = createAgentServiceDouble({
      listCodeIndexWorkspaces: async () => [workspace()],
      getRetrievalConfiguration: async () => ({ sourceProfileId: "profile-a", embeddingModel: "model-a", automaticCodeIndexMode: "semantic" }),
      saveCodeIndexConfiguration,
    });
    const { user } = renderWithAppProviders(<SessionCodeIndexPane service={service} workspacePath="D:/code/app" />);

    await user.click(await screen.findByRole("button", { name: "配置代码索引" }));
    const dialog = screen.getByRole("dialog", { name: "配置 App" });
    await user.click(within(dialog).getByRole("radio", { name: /仅本地/ }));
    await user.click(within(dialog).getByRole("button", { name: "保存配置" }));

    await waitFor(() => expect(saveCodeIndexConfiguration).toHaveBeenCalledWith("workspace-stable-a", expect.objectContaining({ mode: "local" })));
    expect(screen.queryByRole("button", { name: "添加工作区" })).toBeNull();
  });

  it("requires privacy acknowledgement before semantic embedding", async () => {
    const confirmCodeIndexEmbedding = vi.fn(async () => ({ profileId: "profile-a", model: "model-a", generation: 4 }));
    const service = createAgentServiceDouble({
      listCodeIndexWorkspaces: async () => [workspace()],
      getRetrievalConfiguration: async () => ({ sourceProfileId: "profile-a", embeddingModel: "model-a", automaticCodeIndexMode: "semantic" }),
      confirmCodeIndexEmbedding,
    });
    const { user } = renderWithAppProviders(<SessionCodeIndexPane service={service} workspacePath="D:/code/app" />);

    await user.click(await screen.findByRole("button", { name: "查看并确认" }));
    const dialog = screen.getByRole("dialog", { name: "确认代码 Embedding" });
    const confirm = within(dialog).getByRole("button", { name: "确认并开始" });
    expect((confirm as HTMLButtonElement).disabled).toBe(true);
    await user.click(within(dialog).getByRole("checkbox"));
    await user.click(confirm);
    await waitFor(() => expect(confirmCodeIndexEmbedding).toHaveBeenCalledWith("workspace-stable-a", "profile-a", "model-a", 4));
  });

  it("closes delete confirmation before native cleanup settles", async () => {
    let finishDelete: (() => void) | undefined;
    const deleteCodeIndexWorkspace = vi.fn(() => new Promise<void>((resolve) => {
      finishDelete = resolve;
    }));
    const service = createAgentServiceDouble({
      listCodeIndexWorkspaces: async () => [workspace("ready")],
      getRetrievalConfiguration: async () => ({ sourceProfileId: null, embeddingModel: null, automaticCodeIndexMode: "local" }),
      deleteCodeIndexWorkspace,
    });
    const { user } = renderWithAppProviders(<SessionCodeIndexPane service={service} workspacePath="D:/code/app" />);

    await user.click(await screen.findByRole("button", { name: "删除代码索引" }));
    const dialog = screen.getByRole("dialog", { name: "删除代码索引" });
    await user.click(within(dialog).getByRole("button", { name: "确认删除" }));

    expect(deleteCodeIndexWorkspace).toHaveBeenCalledWith("workspace-stable-a");
    expect(screen.queryByRole("dialog", { name: "删除代码索引" })).toBeNull();
    expect((screen.getByRole("button", { name: "配置代码索引" }) as HTMLButtonElement).disabled).toBe(true);

    finishDelete?.();
    await waitFor(() => expect((screen.getByRole("button", { name: "配置代码索引" }) as HTMLButtonElement).disabled).toBe(false));
  });
});
