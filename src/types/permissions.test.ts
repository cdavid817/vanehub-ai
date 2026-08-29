import { describe, expect, it } from "vitest";
import {
  approvalIsUnresolved,
  approvalResolutionOutcomes,
  normalizeApprovalResolutionOutcome,
} from "./permissions";

/**
 * The native command returns these tokens verbatim. The matching Rust list is asserted by
 * `every_outcome_token_is_stable` in `resolve_approval_tests.rs`; a token added on one side without
 * the other degrades to `unknown` here rather than to a wrong claim about whether the tool ran.
 */
describe("approval resolution outcomes", () => {
  it("pins the token set the native command is allowed to return", () => {
    expect([...approvalResolutionOutcomes]).toEqual([
      "delivered",
      "stale",
      "delivery_failed",
      "resolving",
      "already_resolved",
      "not_found",
    ]);
  });

  it("maps a token from a newer native build to unknown rather than to a success", () => {
    expect(normalizeApprovalResolutionOutcome("delivered")).toBe("delivered");
    expect(normalizeApprovalResolutionOutcome("denied_fail_closed")).toBe("unknown");
    expect(normalizeApprovalResolutionOutcome("")).toBe("unknown");
    // The dangerous mistake would be defaulting to the nearest-looking known token.
    expect(normalizeApprovalResolutionOutcome("delivered_late")).toBe("unknown");
  });

  it("treats only resolving and unknown as leaving the request open", () => {
    const open = approvalResolutionOutcomes.filter((outcome) => approvalIsUnresolved(outcome));
    expect(open).toEqual(["resolving"]);
    expect(approvalIsUnresolved("unknown")).toBe(true);
    // A durable-but-undelivered decision is closed: offering the controls back would invite a
    // second decision for a request that already has one.
    expect(approvalIsUnresolved("delivery_failed")).toBe(false);
  });
});
