import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { tauriAgentClient } from "./tauri-agent-client";

const status = {
  phase: "disabled",
  totalFiles: 0,
  processedFiles: 0,
  failedFiles: 0,
  totalChunks: 0,
  processedChunks: 0,
  pendingChunks: 0,
  indexedChunks: 0,
  failedChunks: 0,
  redactionCount: 0,
  estimatedEmbeddingRequests: 0,
  lastFailureCategory: null,
  updatedAt: "2026-08-08T08:00:00Z",
};

const workspace = {
  workspaceId: "workspace-a",
  canonicalRoot: "D:/code/app",
  displayName: "App",
  enabled: false,
  mode: "local",
  selectedRoots: [""],
  languages: ["rust"],
  exclusionPatterns: [],
  maxFileBytes: 102_400,
  indexVersion: "1",
  generation: 0,
  status,
};

describe("Tauri code-index adapter", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((command: string) => {
      if (command === "list_code_index_workspaces") return Promise.resolve([workspace]);
      if (command === "get_code_index_status" || command === "refresh_code_index_workspace") return Promise.resolve(status);
      if (command === "confirm_code_index_embedding") {
        return Promise.resolve({ profileId: "profile-a", model: "model-a", generation: 0 });
      }
      if (command === "list_code_index_audit") return Promise.resolve([]);
      if (command === "delete_code_index_workspace") return Promise.resolve(undefined);
      return Promise.resolve(workspace);
    });
  });

  it("maps every workspace operation to its registered command", async () => {
    await tauriAgentClient.listCodeIndexWorkspaces();
    await tauriAgentClient.getCodeIndexWorkspace("workspace-a");
    await tauriAgentClient.registerCodeIndexWorkspace("D:/code/app", "App");
    await tauriAgentClient.saveCodeIndexConfiguration("workspace-a", {
      enabled: false,
      mode: "semantic",
      selectedRoots: ["."],
      languages: ["rust"],
      exclusionPatterns: [],
      maxFileBytes: 102_400,
    });
    await tauriAgentClient.refreshCodeIndexWorkspace("workspace-a");
    await tauriAgentClient.confirmCodeIndexEmbedding("workspace-a", "profile-a", "model-a", 0);
    await tauriAgentClient.getCodeIndexStatus("workspace-a");
    await tauriAgentClient.listCodeIndexAudit("workspace-a", 20);
    await tauriAgentClient.rebuildCodeIndexWorkspace("workspace-a");
    await tauriAgentClient.disableCodeIndexWorkspace("workspace-a");
    await tauriAgentClient.deleteCodeIndexWorkspace("workspace-a");

    expect(invokeMock.mock.calls).toEqual([
      ["list_code_index_workspaces"],
      ["get_code_index_workspace", { workspaceId: "workspace-a" }],
      ["register_code_index_workspace", { root: "D:/code/app", displayName: "App" }],
      ["save_code_index_configuration", { workspaceId: "workspace-a", configuration: {
        enabled: false, mode: "semantic", selectedRoots: [""], languages: ["rust"], exclusionPatterns: [], maxFileBytes: 102_400,
      } }],
      ["refresh_code_index_workspace", { workspaceId: "workspace-a" }],
      ["confirm_code_index_embedding", { workspaceId: "workspace-a", profileId: "profile-a", model: "model-a", generation: 0 }],
      ["get_code_index_status", { workspaceId: "workspace-a" }],
      ["list_code_index_audit", { workspaceId: "workspace-a", limit: 20 }],
      ["rebuild_code_index_workspace", { workspaceId: "workspace-a" }],
      ["disable_code_index_workspace", { workspaceId: "workspace-a" }],
      ["delete_code_index_workspace", { workspaceId: "workspace-a" }],
    ]);
  });

  it("rejects malformed native status before it reaches consumers", async () => {
    invokeMock.mockResolvedValueOnce({ ...status, phase: "unknown" });
    await expect(tauriAgentClient.getCodeIndexStatus("workspace-a"))
      .rejects.toThrow("invalid code-index response");
  });
});
