import { describe, expect, it } from "vitest";
import { sessionTabDefinitions } from "./session-tab-bar";
import { effectiveSeatId, showsSeatSwitcher, tabScope } from "./tab-scope";
import {
  capabilityScopeDisagreements,
  lookupWorkspaceTabCapability,
  showsWorkspaceSeatSwitcher,
  WORKSPACE_TAB_CAPABILITIES,
  workspaceTabCapability,
} from "./workspace-tab-capability";

describe("workspace tab capabilities", () => {
  it("registers exactly the tabs the bar renders", () => {
    // A tab in the bar without an entry would fall back to whatever a caller guessed, and the
    // guess would look right until the day it did not.
    expect(Object.keys(WORKSPACE_TAB_CAPABILITIES).sort()).toEqual(
      sessionTabDefinitions.map(({ id }) => id).sort(),
    );
    for (const { id } of sessionTabDefinitions) {
      expect(workspaceTabCapability(id).id).toBe(id);
    }
  });

  it("refuses to answer for a tab nobody registered", () => {
    // A permissive default is the dangerous answer: an unregistered panel would inherit "no live
    // work, session-scoped", keep its subscription running while hidden, and never show a seat
    // switcher it needed.
    expect(lookupWorkspaceTabCapability("onepiece-scratch")).toBeNull();
    expect(lookupWorkspaceTabCapability("logs")).toBe(WORKSPACE_TAB_CAPABILITIES.logs);
    expect(lookupWorkspaceTabCapability("toString")).toBeNull();
  });

  it("marks a shell as needing one concrete seat and terminal history as accepting all", () => {
    expect(workspaceTabCapability("shell").seatMode).toBe("required");
    expect(workspaceTabCapability("terminal").seatMode).toBe("optional");
    expect(workspaceTabCapability("logs").seatMode).toBe("optional");
    expect(workspaceTabCapability("traces").seatMode).toBe("none");
    expect(workspaceTabCapability("report").seatMode).toBe("none");
  });

  it("keeps a live attachment only where ending it would end the user's work", () => {
    expect(workspaceTabCapability("shell").retention).toBe("keep-live");
    expect(workspaceTabCapability("chat").retention).toBe("keep-live");
    for (const id of ["changes", "documents", "files", "terminal", "logs", "traces", "report"] as const) {
      expect(workspaceTabCapability(id).retention, id).toBe("keep-state");
    }
    // Nothing is thrown away on a tab switch, which is what makes a hidden panel's form survive.
    // Widened to strings on purpose: `satisfies` keeps the literal types, so comparing against
    // "unmount" directly is a type error rather than an assertion.
    const declared = new Set(
      Object.values(WORKSPACE_TAB_CAPABILITIES).map((entry) => String(entry.retention)),
    );
    expect(declared.has("unmount")).toBe(false);
  });

  it("agrees with the scope destination table", () => {
    expect(capabilityScopeDisagreements()).toEqual([]);
    expect(workspaceTabCapability("chat").consumesScope).toBe(false);
  });

  it("backs the seat helpers instead of a second list", () => {
    expect(tabScope("shell")).toBe("seat");
    expect(tabScope("traces")).toBe("session");
    expect(showsSeatSwitcher("logs", 2)).toBe(true);
    // One seat means one option, so the control would be a statement with no alternative.
    expect(showsSeatSwitcher("logs", 1)).toBe(false);
    expect(showsWorkspaceSeatSwitcher("report", 3)).toBe(false);
  });

  it("never narrows a session-scoped tab to the seat a switcher happens to hold", () => {
    const seats = [{ seatId: "seat-1", agentId: "a", roleId: null }, { seatId: "seat-2", agentId: "b", roleId: null }];
    expect(effectiveSeatId("logs", seats, 1)).toBe("seat-2");
    expect(effectiveSeatId("traces", seats, 1)).toBeNull();
  });
});
