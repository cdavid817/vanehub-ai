import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock } = vi.hoisted(() => ({ invokeMock: vi.fn(), listenMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));

import {
  normalizeEvolutionNotification,
  tauriSkillEvolutionOrchestrationClient,
} from "./tauri-skill-evolution-orchestration-client";

const safeEvent = {
  schemaVersion: 1,
  eventId: "breaker_opened:breaker-one:2",
  eventKind: "breaker_opened",
  workspaceId: "workspace-one",
  runId: null,
  applicationId: null,
  probationId: null,
  breakerId: "breaker-one",
  skillId: "skill-one",
  safeReasonCode: "integrity_failure",
  probationEndsAtMs: null,
  entityRevision: 2,
};

describe("Tauri Skill evolution orchestration adapter", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
  });

  it("accepts only the bounded safe notification projection", () => {
    expect(normalizeEvolutionNotification(safeEvent)).toEqual(safeEvent);
    expect(normalizeEvolutionNotification({ ...safeEvent, diff: "private" })).toBeNull();
    expect(normalizeEvolutionNotification({ ...safeEvent, entityRevision: -1 })).toBeNull();
  });

  it("listens before recovering durable notifications and isolates recovery failure", async () => {
    let listener: ((event: { payload: unknown }) => void) | undefined;
    const unlisten = vi.fn();
    listenMock.mockImplementation(async (_name, callback) => {
      listener = callback;
      return unlisten;
    });
    invokeMock.mockRejectedValue(new Error("unavailable"));
    const handler = vi.fn();
    const unsubscribe = await tauriSkillEvolutionOrchestrationClient
      .subscribeEvolutionNotifications(handler);
    listener?.({ payload: safeEvent });
    expect(handler).toHaveBeenCalledWith(safeEvent);
    expect(invokeMock).toHaveBeenCalledWith("dispatch_skill_evolution_notifications");
    expect(listenMock.mock.invocationCallOrder[0]).toBeLessThan(invokeMock.mock.invocationCallOrder[0]);
    unsubscribe();
    expect(unlisten).toHaveBeenCalledOnce();
  });
});
