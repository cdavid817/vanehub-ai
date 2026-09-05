import { beforeEach, describe, expect, it, vi } from "vitest";
import { webPermissionsClient } from "./web-permissions-client";
import {
  resetWebApprovalResolutionsForTest,
  webApprovalClaims,
  webApprovalDeliveryFaults,
  webApprovalResolutions,
  webPendingApprovals,
} from "./web-permissions-mock-state";

vi.mock("./web-agent-client", () => ({
  resolveWebMockToolApproval: vi.fn(() => true),
}));

const { resolveWebMockToolApproval } = await import("./web-agent-client");
const deliverMock = vi.mocked(resolveWebMockToolApproval);

function seedPending(requestId: string) {
  webPendingApprovals.set(requestId, {
    sessionId: "session-1",
    messageId: "message-1",
    toolName: "file",
    agentId: "agent-1",
    action: "file.write",
    resource: "src/lib.rs",
    riskLevel: "L2",
    createdAt: "0",
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  deliverMock.mockReturnValue(true);
  webPendingApprovals.clear();
  resetWebApprovalResolutionsForTest();
});

/**
 * Web/mock is a UI-development aid, not a security boundary — nothing here executes anything. What
 * it has to reproduce is the *shape* of resolving, because the UI is written against that shape: a
 * mock that answered "done" to every call would let a duplicate-click or stale-state bug pass every
 * Web-mode test and surface only on the desktop client.
 */
describe("webPermissionsClient.resolvePendingApproval", () => {
  it("delivers once and activates the remembered grant only on acknowledgement", async () => {
    seedPending("approval-1");

    const outcome = await webPermissionsClient.resolvePendingApproval("approval-1", true, "session");

    expect(outcome).toBe("delivered");
    const resolution = webApprovalResolutions.get("approval-1");
    expect(resolution?.state).toBe("delivered");
    expect(resolution?.grant).toEqual({ active: true });
  });

  it("writes no grant for a Once decision", async () => {
    seedPending("approval-1");

    await webPermissionsClient.resolvePendingApproval("approval-1", true, "once");

    expect(webApprovalResolutions.get("approval-1")?.grant).toBeNull();
  });

  it("reports the existing resolution instead of making a second one", async () => {
    seedPending("approval-1");
    await webPermissionsClient.resolvePendingApproval("approval-1", true, "session");

    // A different decision, deliberately: the retry must not be able to change the answer.
    const second = await webPermissionsClient.resolvePendingApproval("approval-1", false, "global");

    expect(second).toBe("already_resolved");
    expect(deliverMock).toHaveBeenCalledTimes(1);
    expect(webApprovalResolutions.get("approval-1")?.effect).toBe("allow");
    expect(webApprovalResolutions.get("approval-1")?.scope).toBe("session");
  });

  it("reports a claimed request as resolving rather than deciding it again", async () => {
    seedPending("approval-1");
    // The interleaving stated rather than raced for: another caller holds the claim.
    webApprovalClaims.set("approval-1", "web-resolution-99");

    const outcome = await webPermissionsClient.resolvePendingApproval("approval-1", true, "session");

    expect(outcome).toBe("resolving");
    expect(deliverMock).not.toHaveBeenCalled();
    expect(webApprovalResolutions.has("approval-1")).toBe(false);
  });

  it("commits a stale outcome without delivering or granting", async () => {
    seedPending("approval-1");
    webApprovalDeliveryFaults.set("approval-1", "stale");

    const outcome = await webPermissionsClient.resolvePendingApproval("approval-1", true, "global");

    expect(outcome).toBe("stale");
    expect(deliverMock).not.toHaveBeenCalled();
    expect(webApprovalResolutions.get("approval-1")?.grant).toBeNull();
  });

  it("keeps a failed delivery durable with its grant inactive", async () => {
    seedPending("approval-1");
    webApprovalDeliveryFaults.set("approval-1", "delivery_failed");

    const outcome = await webPermissionsClient.resolvePendingApproval("approval-1", true, "project");

    expect(outcome).toBe("delivery_failed");
    const resolution = webApprovalResolutions.get("approval-1");
    expect(resolution?.state).toBe("delivery_failed");
    // The decision is durable; the grant is not visible, because nobody received it.
    expect(resolution?.grant).toEqual({ active: false });
  });

  it("reports a stale resolution as stale on retry, not as already resolved", async () => {
    seedPending("approval-1");
    webApprovalDeliveryFaults.set("approval-1", "stale");
    await webPermissionsClient.resolvePendingApproval("approval-1", true, "global");

    // A reader acts differently on the two: one means somebody answered, the other means there was
    // nobody left to answer.
    expect(await webPermissionsClient.resolvePendingApproval("approval-1", true, "global")).toBe("stale");
  });

  it("treats an unknown request as not found rather than silently succeeding", async () => {
    expect(await webPermissionsClient.resolvePendingApproval("never-existed", true, "once")).toBe("not_found");
    expect(deliverMock).not.toHaveBeenCalled();
  });

  it("releases the claim after a failed delivery so the ledger is the only gate", async () => {
    seedPending("approval-1");
    deliverMock.mockReturnValue(false);

    const outcome = await webPermissionsClient.resolvePendingApproval("approval-1", true, "session");

    expect(outcome).toBe("delivery_failed");
    // The claim is gone, but the committed resolution is what stops a second decision — the same
    // division of responsibility the native flow has.
    expect(webApprovalClaims.has("approval-1")).toBe(false);
    expect(await webPermissionsClient.resolvePendingApproval("approval-1", true, "session")).toBe("already_resolved");
  });
});
