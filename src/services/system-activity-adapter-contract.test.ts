import { beforeEach, describe, expect, it } from "vitest";
import { webSystemActivityClient } from "./web-system-activity-client";
import {
  resetWebSystemActivityForTest,
  seedWebSystemActivityEventForTest,
} from "./web-system-activity-state";

describe("System activity Web adapter contract", () => {
  beforeEach(() => resetWebSystemActivityForTest());

  it("creates sessions lazily from committed activity and marks provenance", async () => {
    expect(await webSystemActivityClient.listSystemActivitySessions()).toHaveLength(0);
    seedWebSystemActivityEventForTest("workspace", "workspace-one", "run_completed");
    const sessions = await webSystemActivityClient.listSystemActivitySessions();
    expect(sessions).toHaveLength(1);
    expect(sessions[0]).toMatchObject({
      kind: "system-activity",
      scopeKind: "workspace",
      canonicalScopeId: "workspace-one",
      unreadCount: 1,
      mockProvenance: "web_simulation",
    });
    // System session ids are namespaced so interactive session commands can refuse them.
    expect(sessions[0].sessionId.startsWith("system-activity-v1-")).toBe(true);
  });

  it("pages and filters the timeline without mutating it", async () => {
    const session = seedWebSystemActivityEventForTest("workspace", "ws", "run_completed");
    seedWebSystemActivityEventForTest("workspace", "ws", "breaker_opened", "error");
    seedWebSystemActivityEventForTest("workspace", "ws", "run_completed");
    const page = await webSystemActivityClient.querySystemActivityTimeline({
      sessionId: session.sessionId,
      pageSize: 2,
    });
    if (page.kind !== "page") throw new Error("expected page");
    expect(page.entries).toHaveLength(2);
    expect(page.nextCursor).toBe("2");
    const errorsOnly = await webSystemActivityClient.querySystemActivityTimeline({
      sessionId: session.sessionId,
      severities: ["error"],
    });
    if (errorsOnly.kind !== "page") throw new Error("expected page");
    expect(errorsOnly.entries).toHaveLength(1);
    expect(errorsOnly.entries[0].envelope.eventCode).toBe("breaker_opened");
  });

  it("keeps the read cursor monotonic and refuses stale revisions", async () => {
    const session = seedWebSystemActivityEventForTest("workspace", "ws", "run_completed");
    seedWebSystemActivityEventForTest("workspace", "ws", "run_completed");
    const initial = await webSystemActivityClient.getSystemActivityReadState(session.sessionId);
    expect(initial.unreadCount).toBe(2);
    const read = await webSystemActivityClient.advanceSystemActivityReadCursor(
      session.sessionId,
      2,
      initial.revision,
    );
    expect(read.unreadCount).toBe(0);
    const rewound = await webSystemActivityClient.advanceSystemActivityReadCursor(
      session.sessionId,
      1,
      read.revision,
    );
    expect(rewound.highestReadSequence).toBe(2);
    await expect(
      webSystemActivityClient.advanceSystemActivityReadCursor(session.sessionId, 2, 999),
    ).rejects.toThrow("system-activity-conflict");
    const unread = await webSystemActivityClient.markSystemActivityUnread(
      session.sessionId,
      2,
      rewound.revision,
    );
    expect(unread.unreadCount).toBe(1);
  });

  it("updates preferences optimistically and reports conflicts", async () => {
    const preferences = {
      scopeKind: "workspace" as const,
      canonicalScopeId: "ws",
      visible: true,
      minimumTimelineSeverity: "info" as const,
      notificationThreshold: "warning" as const,
      digestCadence: "hourly" as const,
      readRetentionDays: 90,
      detailRetentionDays: 90,
      exportItemLimit: 100,
      exportSizeLimitBytes: 1024,
      revision: 0,
    };
    const updated = await webSystemActivityClient.updateSystemActivityPreferences(preferences);
    expect(updated).toMatchObject({ outcome: "updated" });
    const conflict = await webSystemActivityClient.updateSystemActivityPreferences(preferences);
    expect(conflict.outcome).toBe("conflict");
    expect(
      await webSystemActivityClient.getSystemActivityPreferences("workspace", "ws"),
    ).toMatchObject({ digestCadence: "hourly" });
  });

  it("simulates the shadow rebuild lifecycle with activation gating", async () => {
    const session = seedWebSystemActivityEventForTest("workspace", "ws", "run_completed");
    const rebuild = await webSystemActivityClient.beginSystemActivityRebuild("workspace", "ws", 100);
    await expect(
      webSystemActivityClient.activateSystemActivityRebuild(rebuild.rebuildId),
    ).rejects.toThrow("system-activity-conflict");
    let step = await webSystemActivityClient.advanceSystemActivityRebuild(rebuild.rebuildId, 10);
    while (step.step === "running") {
      step = await webSystemActivityClient.advanceSystemActivityRebuild(rebuild.rebuildId, 10);
    }
    expect(step.step).toBe("validating");
    expect(
      (await webSystemActivityClient.validateSystemActivityRebuild(rebuild.rebuildId)).step,
    ).toBe("ready");
    expect(
      (await webSystemActivityClient.activateSystemActivityRebuild(rebuild.rebuildId)).step,
    ).toBe("active");
    const sessions = await webSystemActivityClient.listSystemActivitySessions();
    expect(sessions[0].activeGenerationId).toBe(rebuild.shadowGenerationId);
    await expect(
      webSystemActivityClient.cancelSystemActivityRebuild(rebuild.rebuildId),
    ).rejects.toThrow("system-activity-conflict");
    const untouched = await webSystemActivityClient.getSystemActivityReadState(session.sessionId);
    expect(untouched.unreadCount).toBe(1);
  });

  it("exports with limits, completeness, and the retention disclosure", async () => {
    const session = seedWebSystemActivityEventForTest("workspace", "ws", "run_completed");
    seedWebSystemActivityEventForTest("workspace", "ws", "run_completed");
    expect(await webSystemActivityClient.chooseSystemActivityExportTarget("json")).toBe(
      "/exports/skill-evolution-activity.json",
    );
    seedWebSystemActivityEventForTest("workspace", "ws", "run_completed");
    const record = await webSystemActivityClient.exportSystemActivity({
      exportId: "export-1",
      query: { sessionId: session.sessionId },
      format: "json",
      locale: "zh-CN",
      targetPath: "/exports/activity.json",
      itemLimit: 2,
    });
    expect(record).toMatchObject({
      itemCount: 2,
      complete: false,
      redactionVersion: "activity-redaction-v1",
      outsideAutomaticRetention: true,
    });
  });

  it("opens notifications only after the timeline item is visible", async () => {
    const session = seedWebSystemActivityEventForTest("workspace", "ws", "breaker_opened", "error");
    const entries = await webSystemActivityClient.querySystemActivityTimeline({
      sessionId: session.sessionId,
    });
    if (entries.kind !== "page") throw new Error("expected page");
    const requestId = `activity-notification:${entries.entries[0].envelope.eventId}`;
    expect(
      await webSystemActivityClient.openSystemActivityNotification(requestId, 0),
    ).toEqual({ kind: "pending" });
    const opened = await webSystemActivityClient.openSystemActivityNotification(requestId, 1);
    expect(opened.kind).toBe("opened");
    // Dismissal never deletes activity.
    await webSystemActivityClient.dismissSystemActivityNotification(requestId);
    const after = await webSystemActivityClient.querySystemActivityTimeline({
      sessionId: session.sessionId,
    });
    if (after.kind !== "page") throw new Error("expected page");
    expect(after.entries).toHaveLength(1);
  });
});
