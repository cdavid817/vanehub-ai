import { describe, expect, it, vi } from "vitest";
import type { ActivityNavigationKind } from "./activity-contracts";
import { openActivityNavigation, resolveActivityNavigation } from "./activity-navigation";

const kinds: ActivityNavigationKind[] = [
  "run", "evidence", "assessment", "dossier", "generation_job", "curator_candidate",
  "overlay_history", "skill", "probation", "breaker",
];

describe("system activity navigation", () => {
  it("resolves every allowlisted detail kind to an immutable view descriptor", () => {
    for (const kind of kinds) {
      const destination = resolveActivityNavigation({ kind, stableId: `${kind}-1`, childId: "child-1" });
      expect(destination).toEqual({
        mode: "view", detailKind: kind, stableId: `${kind}-1`, childId: "child-1",
      });
      expect(Object.isFrozen(destination)).toBe(true);
    }
  });

  it("rejects unknown, malformed, and action-bearing links", () => {
    expect(resolveActivityNavigation({ kind: "approve", stableId: "candidate-1" })).toBeNull();
    expect(resolveActivityNavigation({ kind: "run", stableId: "run-1", action: "retry" })).toBeNull();
    expect(resolveActivityNavigation({ kind: "breaker", stableId: "../breaker" })).toBeNull();
  });

  it("can only call the supplied navigator and never a mutation operation", () => {
    const navigate = vi.fn();
    const approve = vi.fn();
    const retry = vi.fn();
    const cancel = vi.fn();
    const apply = vi.fn();
    const acknowledgeBreaker = vi.fn();
    const revert = vi.fn();

    expect(openActivityNavigation({ kind: "curator_candidate", stableId: "candidate-1" }, navigate)).toBe(true);
    expect(navigate).toHaveBeenCalledWith({
      mode: "view", detailKind: "curator_candidate", stableId: "candidate-1",
    });
    for (const mutation of [approve, retry, cancel, apply, acknowledgeBreaker, revert]) {
      expect(mutation).not.toHaveBeenCalled();
    }
  });
});
