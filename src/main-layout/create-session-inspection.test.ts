import { describe, expect, it } from "vitest";
import { createInspectionTracker } from "./create-session-inspection";

/**
 * A directory inspection is an answer to a question about one specific path.
 *
 * Two inspections in flight can return in either order — the second path is often on a warm cache
 * while the first is still walking a cold network share. Applying whichever lands last attributes
 * one directory's Git capability to another, and the dialog then offers worktree creation for a
 * folder that is not a repository, or hides it for one that is.
 */
describe("inspection attribution", () => {
  it("keeps the current path's result when an earlier one resolves later", () => {
    const tracker = createInspectionTracker();
    const forA = tracker.begin("D:/a");
    const forB = tracker.begin("D:/b");

    // B resolves first, then A -- the ordering the audit reproduced.
    expect(tracker.accepts(forB, "D:/b")).toBe(true);
    expect(tracker.accepts(forA, "D:/a")).toBe(false);
  });

  it("rejects a result whose path no longer matches, even at the current request id", () => {
    const tracker = createInspectionTracker();
    const ticket = tracker.begin("D:/a");

    // Same request, different path: the user edited the field while it was in flight.
    expect(tracker.accepts(ticket, "D:/other")).toBe(false);
  });

  it("invalidates everything in flight when the surface is reset", () => {
    const tracker = createInspectionTracker();
    const ticket = tracker.begin("D:/a");

    tracker.reset();

    // Reopening must not be populated by an inspection the previous session started.
    expect(tracker.accepts(ticket, "D:/a")).toBe(false);
  });

  it("accepts the newest request for its own path", () => {
    const tracker = createInspectionTracker();
    tracker.begin("D:/a");
    const current = tracker.begin("D:/b");

    expect(tracker.accepts(current, "D:/b")).toBe(true);
  });
});
