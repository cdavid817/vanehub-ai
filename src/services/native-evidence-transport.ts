import type { Unsubscribe } from "../types/session-workspace-evidence";

/**
 * The native commands the evidence client will call once they exist. Naming them here rather than
 * inline keeps the set that has to be registered visible in one place.
 */
export type EvidenceCommandName =
  | "get_workspace_evidence_summary"
  | "list_execution_records"
  | "get_execution_record"
  | "get_evidence_subscription_bootstrap"
  | "get_session_run_report";

/**
 * The seam between the evidence client and the desktop runtime.
 *
 * Injecting it is what lets Group 2 finish and be tested before any command is registered: the
 * client's parsing, paging, and subscription behaviour are exercised against a fixture transport,
 * while the binding the application actually uses answers with a typed reason code. Without the
 * seam the only way to test the client would be to call `invoke()` for a command Tauri does not
 * know, which returns an opaque framework error — a failure the UI cannot distinguish from a
 * runtime fault.
 */
export interface NativeEvidenceTransport {
  invokeEvidence(command: EvidenceCommandName, payload: unknown): Promise<unknown>;
  subscribeEvidenceNotices(handler: (payload: unknown) => void): Promise<Unsubscribe>;
}

/**
 * A refusal the UI can localize, as distinct from a thrown framework string. `reasonCode` is one
 * of the stable codes with an `evidence.reason.*` translation.
 */
export class EvidenceUnavailableError extends Error {
  readonly reasonCode: string;

  constructor(reasonCode: string, message?: string) {
    super(message ?? reasonCode);
    this.name = "EvidenceUnavailableError";
    this.reasonCode = reasonCode;
  }
}

export function isEvidenceUnavailableError(value: unknown): value is EvidenceUnavailableError {
  return value instanceof EvidenceUnavailableError;
}

/**
 * The binding for a runtime with no native evidence at all — the Web/mock adapter's neighbour, and
 * the fallback a test uses to drive the refusal path. The desktop client binds the real transport;
 * this one refuses uniformly rather than invoking anything, so a panel shows "not available in
 * this runtime" instead of a framework error that reads like a crash.
 */
export const unavailableEvidenceTransport: NativeEvidenceTransport = {
  invokeEvidence(command) {
    return Promise.reject(
      new EvidenceUnavailableError(
        "evidence_unavailable",
        `Native evidence command is not registered yet: ${command}`,
      ),
    );
  },
  subscribeEvidenceNotices() {
    // Resolving with a no-op unsubscribe rather than rejecting: a panel that subscribes on mount
    // should render its empty state, not an error boundary, while the capability is pending.
    return Promise.resolve(() => undefined);
  },
};
