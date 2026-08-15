import { describe, expect, it } from "vitest";
import { nextSlashTabRequestState, type SlashTabRequestState } from "./slash-tab-request";

const logsRequest = { tab: "logs" as const, nonce: 1 };

describe("nextSlashTabRequestState", () => {
  it("keeps the request when the displayed session id has not changed", () => {
    const state: SlashTabRequestState = { request: logsRequest, trackedSessionId: "session-a" };
    expect(nextSlashTabRequestState(state.request, state.trackedSessionId, "session-a")).toEqual(state);
  });

  it("clears the request and retargets tracking when the displayed session id changes", () => {
    expect(nextSlashTabRequestState(logsRequest, "session-a", "session-b"))
      .toEqual({ request: null, trackedSessionId: "session-b" });
  });

  // Reproduces the review finding. model.activeSessionId is what d58e765f tracked; inspecting
  // session B's loop while session A stays active in the sidebar never changes it (switchSession
  // also no-ops when re-clicking the already-active session), so a guard keyed on it never fires
  // across this round trip and the stale "logs" request survives — landing on Logs instead of
  // Chat after the user backs out of inspection.
  it("documents the bug: tracking a session id loop inspection never changes lets a stale request survive the round trip", () => {
    const sidebarActiveSessionId = "session-a"; // never moves: the inspected session (B) isn't it
    let state: SlashTabRequestState = { request: logsRequest, trackedSessionId: "session-a" };
    state = nextSlashTabRequestState(state.request, state.trackedSessionId, sidebarActiveSessionId); // enter inspection of B
    state = nextSlashTabRequestState(state.request, state.trackedSessionId, sidebarActiveSessionId); // exit back to A
    expect(state.request).toEqual(logsRequest); // wrong: "logs" outlives the round trip
  });

  it("fixes it: tracking the displayed session id clears the request entering inspection, and it stays cleared on exit", () => {
    let state: SlashTabRequestState = { request: logsRequest, trackedSessionId: "session-a" };
    state = nextSlashTabRequestState(state.request, state.trackedSessionId, "session-b"); // enter: displayedSession -> B
    expect(state.request).toBeNull();
    state = nextSlashTabRequestState(state.request, state.trackedSessionId, "session-a"); // exit: displayedSession -> A
    expect(state.request).toBeNull(); // correct: does not resurrect "logs"
  });
});
