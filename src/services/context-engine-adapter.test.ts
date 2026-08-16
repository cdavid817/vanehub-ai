import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { tauriAgentClient } from "./tauri-agent-client";
import { webAgentClient } from "./web-agent-client";

describe("Context Engine service adapters", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ items: [], nextCursor: null });
  });

  it("keeps Tauri list and detail calls behind the shared service boundary", async () => {
    const query = { sessionId: "session-1", cursor: null, limit: 20 };
    await tauriAgentClient.listContextEvidenceManifests(query);
    await tauriAgentClient.getContextEvidenceManifest("generation-1");
    expect(invokeMock.mock.calls).toEqual([
      ["list_context_evidence_manifests", { input: query }],
      ["get_context_evidence_manifest", { generationId: "generation-1" }],
    ]);
  });

  it("provides deterministic Web parity and bounded empty pagination", async () => {
    const first = await webAgentClient.listContextEvidenceManifests({ cursor: null, limit: 1 });
    const repeated = await webAgentClient.listContextEvidenceManifests({ cursor: null, limit: 1 });
    expect(repeated).toEqual(first);
    expect(first.items[0]?.runtime).toBe("web-mock");
    await expect(webAgentClient.getContextEvidenceManifest("unknown")).resolves.toBeNull();
    await expect(webAgentClient.listContextEvidenceManifests({ cursor: "end", limit: 1 }))
      .resolves.toEqual({ items: [], nextCursor: null });
  });
});
