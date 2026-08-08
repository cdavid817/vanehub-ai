import { describe, expect, it } from "vitest";
import type { CodeIndexConfigurationInput } from "../types/code-index";
import {
  normalizeCodeIndexConfiguration,
  normalizeCodeIndexStatus,
  normalizeCodeIndexWorkspace,
  normalizeCodeIndexWorkspaces,
} from "./code-index-contract";

const status = {
  phase: "ready",
  totalFiles: 3,
  processedFiles: 3,
  failedFiles: 1,
  totalChunks: 5,
  processedChunks: 5,
  pendingChunks: 0,
  indexedChunks: 4,
  failedChunks: 1,
  redactionCount: 2,
  estimatedEmbeddingRequests: 1,
  lastFailureCategory: null,
  updatedAt: "2026-08-08T08:00:00Z",
};

const workspace = {
  workspaceId: "opaque-workspace-id",
  canonicalRoot: "D:/code/app",
  displayName: "App",
  enabled: true,
  selectedRoots: ["src"],
  languages: ["rust"],
  exclusionPatterns: ["**/*.generated.rs"],
  maxFileBytes: 102_400,
  indexVersion: "1",
  generation: 2,
  status,
};

function configuration(overrides: Partial<CodeIndexConfigurationInput> = {}): CodeIndexConfigurationInput {
  return {
    enabled: true,
    selectedRoots: [".\\src\\", "src"],
    languages: ["rust"],
    exclusionPatterns: ["dist/**"],
    maxFileBytes: 102_400,
    ...overrides,
  };
}

describe("code-index contract", () => {
  it("normalizes relative roots and rejects invalid patterns or languages", () => {
    expect(normalizeCodeIndexConfiguration(configuration()).selectedRoots).toEqual(["src"]);
    expect(() => normalizeCodeIndexConfiguration(configuration({
      languages: ["rust", "brainfuck" as "rust"],
    }))).toThrow("unsupported value");
    expect(() => normalizeCodeIndexConfiguration(configuration({
      exclusionPatterns: [""],
    }))).toThrow("1 to 256");
    expect(() => normalizeCodeIndexConfiguration(configuration({
      selectedRoots: ["../secrets"],
    }))).toThrow("inside the workspace");
  });

  it("rejects unknown phases and inconsistent or unsafe counts", () => {
    expect(() => normalizeCodeIndexStatus({ ...status, phase: "sleeping" })).toThrow("invalid code-index response");
    expect(() => normalizeCodeIndexStatus({ ...status, totalChunks: 4 })).toThrow("invalid code-index response");
    expect(() => normalizeCodeIndexStatus({ ...status, pendingChunks: -1 })).toThrow("invalid code-index response");
  });

  it("rejects invalid and duplicate workspace identities", () => {
    expect(() => normalizeCodeIndexWorkspace({ ...workspace, workspaceId: "" })).toThrow("invalid code-index response");
    expect(() => normalizeCodeIndexWorkspaces([workspace, workspace])).toThrow("invalid code-index response");
  });
});
