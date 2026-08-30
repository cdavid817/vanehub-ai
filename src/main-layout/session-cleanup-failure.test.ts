import { describe, expect, it } from "vitest";
import {
  isSessionShellCleanupIncomplete,
  sessionCleanupFailureKey,
} from "./session-cleanup-failure";

/**
 * Archive and delete are strict about retained Shells now, so a refusal is an ordinary outcome and
 * the user has to be told which one it is. Before this, both mutations had no error handler at all
 * — the click produced nothing, which reads as a click that did not register.
 */
describe("session cleanup failure", () => {
  it("recognises the refusal wherever a layer wrapped it", () => {
    // The code crosses two contexts and a command boundary, and each is entitled to add a sentence
    // around it. Matching the whole message would break on the first prefix somebody adds, and the
    // failure mode of that is the silence this module exists to remove.
    expect(isSessionShellCleanupIncomplete(new Error("session_shell_cleanup_incomplete"))).toBe(true);
    expect(
      isSessionShellCleanupIncomplete(new Error("Validation: session_shell_cleanup_incomplete")),
    ).toBe(true);
    expect(isSessionShellCleanupIncomplete("session_shell_cleanup_incomplete")).toBe(true);
  });

  it("does not claim an unrelated failure is a cleanup refusal", () => {
    expect(isSessionShellCleanupIncomplete(new Error("database is locked"))).toBe(false);
    expect(isSessionShellCleanupIncomplete(new Error(""))).toBe(false);
    expect(isSessionShellCleanupIncomplete(undefined)).toBe(false);
  });

  it("gives archive and delete their own copy", () => {
    const refusal = new Error("session_shell_cleanup_incomplete");
    expect(sessionCleanupFailureKey(refusal, "archive")).toBe("layout.archiveBlockedByShellCleanup");
    expect(sessionCleanupFailureKey(refusal, "delete")).toBe("layout.deleteBlockedByShellCleanup");
  });

  it("falls back to the generic title for anything else", () => {
    // "Something went wrong" and "this is still finishing, try again shortly" are different
    // situations, and only the second is actionable. Using the cleanup copy for an unrelated
    // failure would tell the user to wait for something that is never going to happen.
    const unrelated = new Error("database is locked");
    expect(sessionCleanupFailureKey(unrelated, "archive")).toBe("app.error.title");
    expect(sessionCleanupFailureKey(unrelated, "delete")).toBe("app.error.title");
  });
});
