import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock } = vi.hoisted(() => ({ invokeMock: vi.fn(), listenMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));

import {
  normalizeCuratorNotification,
  subscribeTauriSkillCuratorNotifications,
} from "./tauri-skill-curator-notifications";

const safeEvent = {
  schemaVersion: 1,
  eventKind: "pending_review",
  candidateId: "candidate-1",
  candidateRevision: 2,
  workspaceId: "workspace-1",
  skillId: "review",
  overlayScope: "project",
  state: "ready_for_review",
  risk: "medium",
  route: "needs_human_review",
  navigationTarget: { kind: "candidate_review", candidateId: "candidate-1" },
};

describe("Tauri Skill Curator notifications", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
  });

  it("accepts only the versioned safe projection", () => {
    expect(normalizeCuratorNotification(safeEvent)).toEqual(safeEvent);
    expect(normalizeCuratorNotification({ ...safeEvent, rationale: "private draft rationale" })).toBeUndefined();
    expect(normalizeCuratorNotification({ ...safeEvent, navigationTarget: { ...safeEvent.navigationTarget, note: "secret" } })).toBeUndefined();
  });

  it("attaches the listener before requesting pending delivery", async () => {
    const unlisten = vi.fn();
    let listener: ((event: { payload: unknown }) => void) | undefined;
    listenMock.mockImplementation(async (_name, callback) => {
      listener = callback;
      return unlisten;
    });
    invokeMock.mockResolvedValue({ delivered: 1, failed: 0 });
    const handler = vi.fn();

    const unsubscribe = await subscribeTauriSkillCuratorNotifications(handler);
    listener?.({ payload: safeEvent });

    expect(listenMock.mock.invocationCallOrder[0]).toBeLessThan(invokeMock.mock.invocationCallOrder[0]);
    expect(invokeMock).toHaveBeenCalledWith("dispatch_skill_curator_notifications");
    expect(handler).toHaveBeenCalledWith(safeEvent);
    unsubscribe();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it("isolates recovery failure and ignores invalid native events", async () => {
    let listener: ((event: { payload: unknown }) => void) | undefined;
    listenMock.mockImplementation(async (_name, callback) => {
      listener = callback;
      return vi.fn();
    });
    invokeMock.mockRejectedValue(new Error("event bus unavailable"));
    const handler = vi.fn();

    await expect(subscribeTauriSkillCuratorNotifications(handler)).resolves.toEqual(expect.any(Function));
    listener?.({ payload: { ...safeEvent, candidateRevision: 0 } });
    expect(handler).not.toHaveBeenCalled();
  });
});
