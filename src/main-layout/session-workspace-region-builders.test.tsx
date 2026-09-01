// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";
import { workbenchSelectionKey } from "../types/workbench-selection";
import type { WorkbenchInspection } from "../ui/inspector/use-workbench-inspection";
import { buildConversationSelectionBridge } from "./session-workspace-region-builders";

function inspection(overrides: Partial<WorkbenchInspection> = {}): WorkbenchInspection {
  return {
    detail: { initialLoading: false, refreshing: false, stale: false },
    follow: vi.fn(),
    mode: "overview",
    pin: vi.fn(),
    returnToOverview: vi.fn(),
    selection: null,
    title: "",
    unpin: vi.fn(),
    ...overrides,
  };
}

// task 9.6's "Follow Selection mode": a click follows (never pins) the clicked message/tool, and
// the source object learns whether it is the one currently followed/pinned.
describe("buildConversationSelectionBridge", () => {
  it("returns inert handlers and a null key when there is no displayed session", () => {
    const bridge = buildConversationSelectionBridge(inspection(), null);
    expect(bridge.currentSelectionKey).toBeNull();
    expect(bridge.onSelectMessage).toBeUndefined();
    expect(bridge.onSelectTool).toBeUndefined();
  });

  it("resolves currentSelectionKey only for a message/tool selection matching the displayed session", () => {
    const sessionKind = inspection({ selection: { kind: "session", sessionId: "s1" } });
    expect(buildConversationSelectionBridge(sessionKind, "s1").currentSelectionKey).toBeNull();

    const otherSession = inspection({ selection: { kind: "message", sessionId: "s2", messageId: "m1" } });
    expect(buildConversationSelectionBridge(otherSession, "s1").currentSelectionKey).toBeNull();

    const matchingMessage = inspection({ selection: { kind: "message", sessionId: "s1", messageId: "m1" } });
    expect(buildConversationSelectionBridge(matchingMessage, "s1").currentSelectionKey).toBe(
      workbenchSelectionKey({ kind: "message", sessionId: "s1", messageId: "m1" }),
    );

    const matchingTool = inspection({ selection: { kind: "tool", sessionId: "s1", messageId: "m1", toolCallId: "t1" } });
    expect(buildConversationSelectionBridge(matchingTool, "s1").currentSelectionKey).toBe(
      workbenchSelectionKey({ kind: "tool", sessionId: "s1", messageId: "m1", toolCallId: "t1" }),
    );
  });

  it("routes onSelectMessage/onSelectTool through inspection.follow, never pin", () => {
    const follow = vi.fn();
    const pin = vi.fn();
    const bridge = buildConversationSelectionBridge(inspection({ follow, pin }), "s1");

    bridge.onSelectMessage?.("m1");
    expect(follow).toHaveBeenCalledWith({ kind: "message", sessionId: "s1", messageId: "m1" });

    bridge.onSelectTool?.("m1", "t1");
    expect(follow).toHaveBeenCalledWith({ kind: "tool", sessionId: "s1", messageId: "m1", toolCallId: "t1" });

    expect(pin).not.toHaveBeenCalled();
  });
});
