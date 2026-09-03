import { useTranslation } from "react-i18next";

export interface ScheduledTaskSessionLinkProps {
  sessionId: string | null;
  /** 19.11/19.6's "Session... links": intentionally optional and, in this pass, never supplied by
   *  `ScheduledTasksPanel`'s own current sole caller (`runs-destination.tsx`) -- wiring a real
   *  cross-domain "open this session" navigation reaches into `src/main-layout/`, which is out of
   *  scope for this task batch. The affordance is built and tested end-to-end down to this exact
   *  callback boundary; only the final connection from `App.tsx`'s own router is left undone,
   *  matching `personalization/memory-detail-panel.tsx`'s own identical `onOpenSession?:` shape
   *  (also optional there, also rendered as plain text without it). */
  onOpenSession?: (sessionId: string) => void;
}

/** Shared by the detail view's own "latest run" line and every `ScheduledTaskHistory` row, so a
 *  session reference never renders two different ways depending on which one shows it. */
export function ScheduledTaskSessionLink({ onOpenSession, sessionId }: ScheduledTaskSessionLinkProps) {
  const { t } = useTranslation();
  if (!sessionId) return <span className="text-muted-foreground">—</span>;
  if (!onOpenSession) return <span className="font-mono text-[11px] text-muted-foreground">{sessionId}</span>;
  return (
    <button
      className="ucd-interactive text-left text-xs underline underline-offset-2"
      onClick={() => onOpenSession(sessionId)}
      type="button"
    >
      {t("scheduledTasks.history.openSession")}
    </button>
  );
}
