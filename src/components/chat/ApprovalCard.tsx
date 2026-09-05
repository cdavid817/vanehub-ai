import { useEffect, useState } from "react";
import { Check, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { permissionsService } from "../../services/runtime-permissions-client";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import {
  approvalIsUnresolved,
  type ApprovalResolutionOutcome,
  type ApprovalScope,
  type PendingApprovalEntry,
  type RiskLevel,
} from "../../types/permissions";

const riskTone: Record<RiskLevel, "muted" | "default" | "warning" | "danger"> = {
  L0: "muted", L1: "default", L2: "warning", L3: "danger",
};
const scopeOptions: ApprovalScope[] = ["once", "session", "project", "global"];

/**
 * How each outcome is shown. `tone` matters as much as the wording: only `delivered` may read as a
 * success, because it is the only one where the tool actually ran. A `delivery_failed` decision is
 * durable and must not be presented as "try again" — offering the buttons back would invite a
 * second decision for a request that already has one.
 */
const outcomeTone: Record<ApprovalResolutionOutcome, "muted" | "default" | "warning" | "danger" | "success"> = {
  delivered: "success",
  stale: "muted",
  delivery_failed: "warning",
  resolving: "default",
  already_resolved: "muted",
  not_found: "muted",
  denied_fail_closed: "danger",
  unknown: "warning",
};

export function ApprovalCard({ sessionId, callId }: { sessionId: string; callId: string }) {
  const { t } = useTranslation();
  const [pending, setPending] = useState<PendingApprovalEntry | null>(null);
  const [scope, setScope] = useState<ApprovalScope>("once");
  const [submitting, setSubmitting] = useState<"approve" | "deny" | null>(null);
  const [outcome, setOutcome] = useState<ApprovalResolutionOutcome | null>(null);

  useEffect(() => {
    let cancelled = false;
    void permissionsService.listPendingApprovals().then((entries) => {
      if (!cancelled) setPending(entries.find((entry) => entry.sessionId === sessionId && entry.callId === callId) ?? null);
    });
    return () => { cancelled = true; };
  }, [sessionId, callId]);

  /**
   * Reconciles an outcome this client cannot act on by asking what the backend holds.
   *
   * The pull is the correctness boundary, not the response: a dropped response and a request
   * somebody else resolved look identical from here, and only the list can tell them apart. If the
   * request is gone, the decision landed and the controls must stay closed.
   */
  async function reconcile(): Promise<ApprovalResolutionOutcome> {
    const entries = await permissionsService.listPendingApprovals().catch(() => null);
    if (!entries) return "unknown";
    return entries.some((entry) => entry.id === pending?.id) ? "resolving" : "already_resolved";
  }

  async function resolve(approved: boolean) {
    if (!pending || submitting !== null) return;
    setSubmitting(approved ? "approve" : "deny");
    try {
      const result = await permissionsService
        .resolvePendingApproval(pending.id, approved, scope)
        // A thrown call is the ambiguous case the retry rules exist for: the decision may or may
        // not have committed, so this client must not assume either.
        .catch((): ApprovalResolutionOutcome => "unknown");
      setOutcome(approvalIsUnresolved(result) ? await reconcile() : result);
    } finally {
      setSubmitting(null);
    }
  }

  // Reopened only for the two outcomes that mean nobody has an answer yet. Every other outcome —
  // including the failures — is terminal for this request.
  const retryable = outcome === null || approvalIsUnresolved(outcome);
  const controlsDisabled = submitting !== null || !pending || !retryable;

  return (
    <div className="flex flex-col gap-2 border-t border-warning/30 bg-warning/5 px-3 py-2">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-muted-foreground">{t("chat.toolApproval.prompt")}</span>
        {pending ? <Badge tone={riskTone[pending.riskLevel]}>{t(`chat.toolApproval.riskLevel.${pending.riskLevel}`)}</Badge> : null}
      </div>
      {pending ? (
        <dl className="grid grid-cols-[auto_1fr] gap-x-2 gap-y-1 text-[11px] text-muted-foreground">
          <dt className="font-medium">{t("chat.toolApproval.agent")}</dt><dd className="truncate font-mono">{pending.agentId}</dd>
          <dt className="font-medium">{t("chat.toolApproval.action")}</dt><dd className="truncate font-mono">{pending.action}</dd>
          <dt className="font-medium">{t("chat.toolApproval.resource")}</dt><dd className="truncate font-mono">{pending.resource}</dd>
        </dl>
      ) : null}
      <div className="flex flex-wrap items-center gap-1">
        <span className="mr-1 text-muted-foreground">{t("chat.toolApproval.rememberLabel")}</span>
        {scopeOptions.map((option) => (
          <Button aria-pressed={scope === option} disabled={controlsDisabled} key={option} onClick={() => setScope(option)} size="sm" variant={scope === option ? "default" : "outline"}>
            {t(`chat.toolApproval.scope.${option}`)}
          </Button>
        ))}
      </div>
      {outcome ? (
        <p aria-live="polite" className="text-[11px]" data-testid="approval-outcome" data-outcome={outcome}>
          <Badge tone={outcomeTone[outcome]}>{t(`chat.toolApproval.outcome.${outcome}`)}</Badge>
        </p>
      ) : null}
      <div className="flex items-center gap-2">
        <Button className="ml-auto" disabled={controlsDisabled} onClick={() => void resolve(true)} size="sm" variant="outline">
          <Check className="h-3.5 w-3.5" aria-hidden="true" />{t("chat.toolApproval.approve")}
        </Button>
        <Button disabled={controlsDisabled} onClick={() => void resolve(false)} size="sm" variant="outline">
          <X className="h-3.5 w-3.5" aria-hidden="true" />{t("chat.toolApproval.deny")}
        </Button>
      </div>
    </div>
  );
}
