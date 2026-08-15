import { useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../../services/runtime-agent-client";
import { Button } from "../ui/button";

/**
 * Reads the proposed plan straight out of the tool call's own input, the way `QuestionCard` reads
 * its question: the model already sent everything the user needs in order to decide, so there is
 * no record to fetch behind the call (`add-agent-plan-exit-request`).
 */
export function parseProposedPlan(input: unknown): string | null {
  if (!input || typeof input !== "object" || Array.isArray(input)) return null;
  const record = input as Record<string, unknown>;
  const plan = typeof record.plan === "string" ? record.plan.trim() : "";
  return plan ? plan : null;
}

export function PlanExitCard({
  sessionId,
  callId,
  input,
}: {
  sessionId: string;
  callId: string;
  input: unknown;
}) {
  const { t } = useTranslation();
  const plan = parseProposedPlan(input);
  const [submitting, setSubmitting] = useState(false);

  if (!plan) return null;

  async function decide(approved: boolean) {
    if (submitting) return;
    setSubmitting(true);
    try {
      // Delivery alone; the session leaves plan mode by observing the resolved tool block, not
      // from here. See `plan-exit-signal` for why the decision is read off the block instead.
      await agentService.resolvePlanExit(sessionId, callId, approved);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex flex-col gap-2 border-t border-primary/30 bg-primary/5 px-3 py-2" data-testid="tool-plan-exit">
      <p className="whitespace-pre-wrap text-xs text-foreground">{plan}</p>
      <div className="flex flex-wrap gap-2">
        <Button disabled={submitting} onClick={() => void decide(true)} size="sm">
          {t("chat.toolPlanExit.approve")}
        </Button>
        <Button disabled={submitting} onClick={() => void decide(false)} size="sm" variant="outline">
          {t("chat.toolPlanExit.decline")}
        </Button>
      </div>
    </div>
  );
}
