import { useCallback, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type { LoopAdmission, LoopControlAction, LoopControlEnvelope } from "../types/loop";

export interface LoopAdmissionTarget {
  action: LoopControlAction;
  definitionId?: string;
  runId?: string;
  expectedRevision: number;
}

interface PendingAdmission {
  target: LoopAdmissionTarget;
  admission: LoopAdmission;
}

/**
 * The two-step gate every control transition passes through. Strict-mode transitions receive
 * their envelope immediately; audit-mode transitions pause on the admission facts until the
 * person acknowledges them, and the resulting receipt is bound to exactly that transition.
 */
export function useLoopAdmission(onAdmitted: (target: LoopAdmissionTarget, envelope: LoopControlEnvelope) => Promise<void>) {
  const [pending, setPending] = useState<PendingAdmission | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const request = useCallback(async (target: LoopAdmissionTarget) => {
    setBusy(true);
    setError(null);
    try {
      const admission = await agentService.prepareLoopAdmission({
        action: target.action,
        definitionId: target.definitionId ?? null,
        runId: target.runId ?? null,
        expectedRevision: target.expectedRevision,
      });
      if (!admission.assessment.satisfiesRequestedMode) {
        setError(admission.assessment.blockers.join("\n"));
        return;
      }
      if (admission.acknowledgementRequired) {
        setPending({ target, admission });
        return;
      }
      await onAdmitted(target, { expectedRevision: admission.expectedRevision });
    } catch (requestError) {
      setError(requestError instanceof Error ? requestError.message : String(requestError));
    } finally {
      setBusy(false);
    }
  }, [onAdmitted]);

  const acknowledge = useCallback(async () => {
    if (!pending?.admission.challengeId) return;
    setBusy(true);
    setError(null);
    try {
      const receipt = await agentService.acknowledgeLoopAudit(pending.admission.challengeId);
      setPending(null);
      await onAdmitted(pending.target, {
        expectedRevision: receipt.expectedRevision,
        auditAcknowledgementId: receipt.acknowledgementId,
      });
    } catch (acknowledgeError) {
      setError(acknowledgeError instanceof Error ? acknowledgeError.message : String(acknowledgeError));
    } finally {
      setBusy(false);
    }
  }, [onAdmitted, pending]);

  const dismiss = useCallback(() => { setPending(null); setError(null); }, []);

  return { acknowledge, busy, dismiss, error, pending: pending?.admission ?? null, request, setError };
}
