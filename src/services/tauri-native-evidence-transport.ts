import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Unsubscribe } from "../types/session-workspace-evidence";
import {
  EvidenceUnavailableError,
  type EvidenceCommandName,
  type NativeEvidenceTransport,
} from "./native-evidence-transport";

/**
 * The native event channel. It has to match `EVIDENCE_EVENT_CHANNEL` in the Rust notice publisher
 * verbatim; a mismatch produces a subscription that never fires and never errors.
 */
export const EVIDENCE_EVENT_CHANNEL = "execution-evidence:event";

/**
 * The commands registered in the Rust core registry. A command absent from this set is refused
 * here rather than passed to `invoke()`, because Tauri answers an unregistered command with an
 * opaque framework string the UI cannot tell apart from a runtime fault.
 */
const REGISTERED_EVIDENCE_COMMANDS: ReadonlySet<EvidenceCommandName> = new Set([
  "get_workspace_evidence_summary",
  "list_execution_records",
  "get_execution_record",
  "get_evidence_subscription_bootstrap",
]);

/**
 * Why the report is named rather than simply missing from the set above: the panel needs to say
 * "this build has no report yet", not "something failed". Task 10.8 registers the command and
 * removes this branch.
 */
const NATIVE_REPORT_NOT_INITIALIZED = "native_report_not_initialized";

/**
 * A native failure that already carries a reason code arrives as `{ reasonCode }`, which is the
 * only shape the Rust handlers return. Anything else — a framework string, a panic message — is
 * collapsed to a generic code rather than surfaced, since its text is not translated and may name
 * internals.
 */
function toEvidenceError(error: unknown): EvidenceUnavailableError {
  if (typeof error === "object" && error !== null && "reasonCode" in error) {
    const reasonCode = (error as { reasonCode: unknown }).reasonCode;
    if (typeof reasonCode === "string" && reasonCode.length > 0) {
      return new EvidenceUnavailableError(reasonCode);
    }
  }
  return new EvidenceUnavailableError("evidence_unavailable");
}

/** The production transport: real `invoke`, real event channel. */
export function createNativeEvidenceTransport(): NativeEvidenceTransport {
  return {
    async invokeEvidence(command: EvidenceCommandName, payload: unknown): Promise<unknown> {
      if (command === "get_session_run_report") {
        throw new EvidenceUnavailableError(NATIVE_REPORT_NOT_INITIALIZED);
      }
      if (!REGISTERED_EVIDENCE_COMMANDS.has(command)) {
        throw new EvidenceUnavailableError("evidence_unavailable");
      }
      try {
        return await invoke(command, payload as Record<string, unknown>);
      } catch (error) {
        throw toEvidenceError(error);
      }
    },

    async subscribeEvidenceNotices(handler: (payload: unknown) => void): Promise<Unsubscribe> {
      return listen<unknown>(EVIDENCE_EVENT_CHANNEL, (event) => handler(event.payload));
    },
  };
}
