import { afterEach, describe, expect, it, vi } from "vitest";
import { resetWebRetrievalForTest, searchWebCodeIndex, webAgentClient } from "./web-agent-client";

afterEach(() => {
  resetWebRetrievalForTest();
  vi.restoreAllMocks();
});

describe("Web code-index client", () => {
  it("advances explicit mock phases without filesystem or network access", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch");
    const workspace = await webAgentClient.registerCodeIndexWorkspace("D:/code/app", "App");
    const configured = await webAgentClient.saveCodeIndexConfiguration(workspace.workspaceId, {
      enabled: true,
      mode: "semantic",
      selectedRoots: ["src"],
      languages: ["typescript", "rust"],
      exclusionPatterns: ["dist/**"],
      maxFileBytes: 102_400,
    });
    expect(configured.status.phase).toBe("scanning");
    await expect(webAgentClient.refreshCodeIndexWorkspace(workspace.workspaceId))
      .resolves.toMatchObject({ phase: "parsing", totalFiles: 18 });
    await expect(webAgentClient.refreshCodeIndexWorkspace(workspace.workspaceId))
      .resolves.toMatchObject({ phase: "awaiting_embedding_confirmation", totalChunks: 54 });

    await webAgentClient.saveRetrievalConfiguration("profile-a", "model-a");
    await expect(webAgentClient.confirmCodeIndexEmbedding(
      workspace.workspaceId, "profile-a", "model-a", configured.generation,
    )).resolves.toEqual({ profileId: "profile-a", model: "model-a", generation: configured.generation });
    await expect(webAgentClient.refreshCodeIndexWorkspace(workspace.workspaceId))
      .resolves.toMatchObject({ phase: "ready", indexedChunks: 54, pendingChunks: 0 });
    expect(searchWebCodeIndex(workspace.workspaceId, "handle_login"))
      .toMatchObject([{ filePath: "src/auth.ts", symbolName: "handle_login", matchedVia: "hybrid" }]);
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("invalidates confirmation on rebuild and isolates deletion by workspace", async () => {
    const first = await webAgentClient.registerCodeIndexWorkspace("D:/code/first", "First");
    const second = await webAgentClient.registerCodeIndexWorkspace("D:/code/second", "Second");
    const configured = await webAgentClient.saveCodeIndexConfiguration(first.workspaceId, {
      enabled: true,
      mode: "semantic",
      selectedRoots: [""],
      languages: ["rust"],
      exclusionPatterns: [],
      maxFileBytes: 102_400,
    });
    const rebuilt = await webAgentClient.rebuildCodeIndexWorkspace(first.workspaceId);
    expect(rebuilt).toMatchObject({ generation: configured.generation + 1, status: { phase: "scanning" } });
    await webAgentClient.saveRetrievalConfiguration("profile-a", "model-a");
    await expect(webAgentClient.confirmCodeIndexEmbedding(
      first.workspaceId, "profile-a", "model-a", configured.generation,
    )).rejects.toThrow("stale");
    await expect(webAgentClient.listCodeIndexAudit(first.workspaceId))
      .resolves.toMatchObject([{ event: "rebuilt" }]);

    await webAgentClient.deleteCodeIndexWorkspace(first.workspaceId);
    await expect(webAgentClient.listCodeIndexWorkspaces()).resolves.toEqual([second]);
  });

  it("completes local indexing without an embedding model or confirmation", async () => {
    const workspace = await webAgentClient.registerCodeIndexWorkspace("D:/code/local", "Local");
    expect(workspace.mode).toBe("local");
    await webAgentClient.saveCodeIndexConfiguration(workspace.workspaceId, {
      enabled: true,
      mode: "local",
      selectedRoots: ["src"],
      languages: ["typescript"],
      exclusionPatterns: [],
      maxFileBytes: 102_400,
    });

    await webAgentClient.refreshCodeIndexWorkspace(workspace.workspaceId);
    await expect(webAgentClient.refreshCodeIndexWorkspace(workspace.workspaceId)).resolves.toMatchObject({
      phase: "ready",
      pendingChunks: 0,
      indexedChunks: 54,
      estimatedEmbeddingRequests: 0,
    });
    await expect(webAgentClient.confirmCodeIndexEmbedding(
      workspace.workspaceId, "profile-a", "model-a", 1,
    )).rejects.toThrow("do not use embedding confirmation");
    expect(searchWebCodeIndex(workspace.workspaceId, "handle_login"))
      .toMatchObject([{ matchedVia: "keyword" }]);
  });
});
