import { useTranslation } from "react-i18next";
import { normalizeDisplayPath } from "../../lib/session-path";
import type { Session } from "../../types/agent";
import type { SessionDeletionOperation } from "../../types/session-deletion";
import { dbEffectKey, errorKey, outcomeKey, phaseKey, worktreeEffectKey } from "./session-deletion-model";

/**
 * Progress while the operation runs and the per-group result when it ends. Phases are the ones
 * the backend recorded; there is no percentage because nothing here knows how long Git takes.
 */
export function SessionDeletionResult({
  operation,
  sessions,
}: {
  operation: SessionDeletionOperation | null;
  sessions: Session[];
}) {
  const { t } = useTranslation();
  const titles = new Map(sessions.map((session) => [session.id, session.title]));
  if (!operation) {
    return <p aria-live="polite" className="text-xs text-muted-foreground" role="status">{t("sessionDeletion.phase.accepted")}</p>;
  }
  const running = operation.outcome === "pending";
  return (
    <div aria-live="polite" className="grid gap-2 text-xs" data-outcome={operation.outcome} data-testid="session-deletion-result" role="status">
      <p className="font-medium">
        {running ? t(phaseKey(operation.phase)) : t(outcomeKey(operation.outcome))}
        {operation.runtimeEffect === "simulated" ? ` · ${t("sessionDeletion.simulated")}` : ""}
      </p>
      <ul className="grid gap-2">
        {operation.groups.map((group) => (
          <li className="grid gap-0.5 rounded-md border border-border p-2" data-group-status={group.status} key={group.groupId}>
            <span className="break-all">
              {group.sessionIds.map((id) => titles.get(id) ?? id).join(", ")}
            </span>
            <span className="text-muted-foreground">
              {t(`sessionDeletion.groupStatus.${group.status}`)}
              {running && group.status === "running" ? ` · ${t(phaseKey(group.phase))}` : ""}
            </span>
            {group.worktreeId ? (
              <span className="text-muted-foreground">
                {t("sessionDeletion.worktree.directory")} {t(worktreeEffectKey(group.worktreeEffect))}
                {group.retainedPath && group.worktreeEffect !== "removed" ? ` · ${normalizeDisplayPath(group.retainedPath)}` : ""}
              </span>
            ) : null}
            <span className="text-muted-foreground">{t("sessionDeletion.sessionData")} {t(dbEffectKey(group.dbEffect))}</span>
            {group.errorCode ? <span className="text-destructive">{t(errorKey(group.errorCode), { defaultValue: t("sessionDeletion.error.unknown", { code: group.errorCode }) })}</span> : null}
          </li>
        ))}
      </ul>
      {operation.outcome === "needs_attention" ? <p className="text-destructive">{t("sessionDeletion.needsAttentionHint")}</p> : null}
    </div>
  );
}
