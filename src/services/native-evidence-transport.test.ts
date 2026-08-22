import { describe, expect, it, vi } from "vitest";
import { createFixtureEvidenceTransport } from "../contracts/fixtures/session-workspace-evidence-transport";
import { evidenceSessionIdSchema } from "../contracts/session-workspace-evidence-ids";
import type { ExecutionEvidenceNotice, Unsubscribe } from "../types/session-workspace-evidence";
import type { NativeEvidenceTransport } from "./native-evidence-transport";
import { EvidenceUnavailableError } from "./native-evidence-transport";
import { createTauriSessionWorkspaceEvidenceClient } from "./tauri-session-workspace-evidence-client";

const sessionId = evidenceSessionIdSchema.parse("session-1");

/**
 * A transport whose bootstrap resolves only when the test says so, which is the one way to observe
 * the window between registering the listener and learning the watermark. Every notice published
 * during that window has to survive it.
 */
function deferredBootstrapTransport(watermarkSequence: number) {
  const fixture = createFixtureEvidenceTransport();
  let release: (() => void) | undefined;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  const transport: NativeEvidenceTransport = {
    async invokeEvidence(command, payload) {
      if (command === "get_evidence_subscription_bootstrap") {
        await gate;
        return { sessionId, watermarkSequence, coverage: { state: "complete", reasonCodes: [], truncated: false } };
      }
      return fixture.invokeEvidence(command, payload);
    },
    subscribeEvidenceNotices: (handler) => fixture.subscribeEvidenceNotices(handler),
  };
  return { transport, publish: fixture.publish, release: () => release?.() };
}

function notice(sequence: number) {
  return { kind: "record-appended", sequence, sessionId, occurredAt: "2026-08-22T10:00:00.000Z" };
}

describe("native evidence subscription ordering", () => {
  it("registers the listener before the bootstrap so nothing published in between is lost", async () => {
    const { transport, publish, release } = deferredBootstrapTransport(0);
    const service = createTauriSessionWorkspaceEvidenceClient(transport);
    const seen: ExecutionEvidenceNotice[] = [];

    const pending = service.subscribeExecutionEvidence({ sessionId }, (n) => seen.push(n));
    // The bootstrap has not answered yet. If the listener were registered after it, these would
    // have arrived before anything was listening and would be gone with no trace in the sequence.
    await Promise.resolve();
    publish(notice(1));
    publish(notice(2));
    expect(seen).toHaveLength(0);

    release();
    await pending;

    expect(seen.map((n) => n.sequence)).toEqual([1, 2]);
  });

  it("discards buffered notices the watermark says the first page already covers", async () => {
    const { transport, publish, release } = deferredBootstrapTransport(5);
    const service = createTauriSessionWorkspaceEvidenceClient(transport);
    const seen: ExecutionEvidenceNotice[] = [];

    const pending = service.subscribeExecutionEvidence({ sessionId }, (n) => seen.push(n));
    await Promise.resolve();
    publish(notice(4));
    publish(notice(5));
    publish(notice(6));
    release();
    await pending;

    // 4 and 5 are already in the store the caller read; only 6 is new.
    expect(seen.map((n) => n.sequence)).toEqual([6]);
  });

  it("replays the buffer in sequence order so arrival order cannot fake a gap", async () => {
    const { transport, publish, release } = deferredBootstrapTransport(0);
    const service = createTauriSessionWorkspaceEvidenceClient(transport);
    const seen: ExecutionEvidenceNotice[] = [];

    const pending = service.subscribeExecutionEvidence({ sessionId }, (n) => seen.push(n));
    await Promise.resolve();
    publish(notice(2));
    publish(notice(1));
    release();
    await pending;

    expect(seen.map((n) => `${n.kind}:${n.sequence}`)).toEqual([
      "record-appended:1",
      "record-appended:2",
    ]);
  });

  it("still resumes from a caller-supplied sequence rather than the watermark", async () => {
    const { transport, publish, release } = deferredBootstrapTransport(0);
    const service = createTauriSessionWorkspaceEvidenceClient(transport);
    const seen: ExecutionEvidenceNotice[] = [];

    const pending = service.subscribeExecutionEvidence(
      { sessionId, fromSequence: 3 },
      (n) => seen.push(n),
    );
    release();
    await pending;
    publish(notice(3));
    publish(notice(4));

    // The caller knows what it rendered; the watermark only knows what the store holds.
    expect(seen.map((n) => n.sequence)).toEqual([4]);
  });

  // React cleanup runs more than once in development and after a fast re-render.
  it("tolerates a repeated unsubscribe", async () => {
    const fixture = createFixtureEvidenceTransport();
    const service = createTauriSessionWorkspaceEvidenceClient(fixture);
    const unsubscribe = await service.subscribeExecutionEvidence({ sessionId }, () => undefined);
    unsubscribe();
    expect(() => unsubscribe()).not.toThrow();
  });

  // A bootstrap that fails must not take the subscription with it: replaying a notice the caller
  // already has is de-duplicated downstream, whereas skipping one loses it for good.
  it("falls back to replaying rather than skipping when the bootstrap fails", async () => {
    const fixture = createFixtureEvidenceTransport();
    const transport: NativeEvidenceTransport = {
      invokeEvidence: (command, payload) =>
        command === "get_evidence_subscription_bootstrap"
          ? Promise.reject(new EvidenceUnavailableError("evidence_unavailable"))
          : fixture.invokeEvidence(command, payload),
      subscribeEvidenceNotices: (handler) => fixture.subscribeEvidenceNotices(handler),
    };
    const service = createTauriSessionWorkspaceEvidenceClient(transport);
    const seen: ExecutionEvidenceNotice[] = [];

    await service.subscribeExecutionEvidence({ sessionId }, (n) => seen.push(n));
    fixture.publish(notice(1));

    expect(seen.map((n) => n.sequence)).toEqual([1]);
  });
});

describe("production native evidence transport", () => {
  async function loadTransport(invokeImpl: (command: string, payload: unknown) => Promise<unknown>) {
    vi.resetModules();
    vi.doMock("@tauri-apps/api/core", () => ({ invoke: invokeImpl }));
    vi.doMock("@tauri-apps/api/event", () => ({
      listen: (): Promise<Unsubscribe> => Promise.resolve(() => undefined),
    }));
    const module = await import("./tauri-native-evidence-transport");
    return module.createNativeEvidenceTransport();
  }

  it("invokes each registered evidence command by name", async () => {
    const calls: string[] = [];
    const transport = await loadTransport(async (command) => {
      calls.push(command);
      return {};
    });

    await transport.invokeEvidence("get_workspace_evidence_summary", { sessionId });
    await transport.invokeEvidence("list_execution_records", { scope: { sessionId } });
    await transport.invokeEvidence("get_execution_record", { sessionId, recordId: "r-1" });
    await transport.invokeEvidence("get_evidence_subscription_bootstrap", { sessionId });

    expect(calls).toEqual([
      "get_workspace_evidence_summary",
      "list_execution_records",
      "get_execution_record",
      "get_evidence_subscription_bootstrap",
    ]);
  });

  /**
   * The report command is not registered until 10.8. Invoking it would return Tauri's opaque
   * "unknown command" string, which a panel cannot tell apart from a runtime fault — so the
   * refusal is typed here and `invoke` is never reached.
   */
  it("refuses the session-run report with a stable reason code and never invokes it", async () => {
    const calls: string[] = [];
    const transport = await loadTransport(async (command) => {
      calls.push(command);
      return {};
    });

    await expect(
      transport.invokeEvidence("get_session_run_report", { sessionId }),
    ).rejects.toMatchObject({ reasonCode: "native_report_not_initialized" });
    expect(calls).toEqual([]);
  });

  it("surfaces a native reason code and discards any message that came with it", async () => {
    const transport = await loadTransport(() =>
      Promise.reject({ reasonCode: "evidence_record_not_found" }),
    );

    await expect(
      transport.invokeEvidence("get_execution_record", { sessionId, recordId: "r-1" }),
    ).rejects.toMatchObject({ reasonCode: "evidence_record_not_found" });
  });

  // A framework string is not translated and may name internals, so it is collapsed rather than
  // shown.
  it("collapses an untyped framework failure to a generic code", async () => {
    const transport = await loadTransport(() =>
      Promise.reject("thread panicked at src-tauri/src/contexts/.../repository.rs"),
    );

    const error = await transport
      .invokeEvidence("list_execution_records", {})
      .catch((value: unknown) => value);

    // Asserted by shape rather than by class: `vi.resetModules()` loads a second copy of the
    // module that declares the error, so `instanceof` compares two identical-but-distinct classes.
    const refusal = error as EvidenceUnavailableError;
    expect(refusal.name).toBe("EvidenceUnavailableError");
    expect(refusal.reasonCode).toBe("evidence_unavailable");
    expect(refusal.message).not.toContain("repository.rs");
  });
});
