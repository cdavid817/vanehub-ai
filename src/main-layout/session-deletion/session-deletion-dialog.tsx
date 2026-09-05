import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../components/ui/application-dialog";
import { Button } from "../../components/ui/button";
import { normalizeDisplayPath } from "../../lib/session-path";
import type { SessionDeletionPreview } from "../../types/session-deletion";
import { anyRemovalChosen, canSubmit, confirmLabelKey, retryAllowed } from "./session-deletion-model";
import { SessionDeletionResult } from "./session-deletion-result";
import { SessionDeletionWorktreeRow } from "./session-deletion-worktree-row";
import type { SessionDeletionController } from "./use-session-deletion";

function PreviewSummary({ preview }: { preview: SessionDeletionPreview }) {
  const { t } = useTranslation();
  const projectSessions = preview.sessions.filter((session) => session.workspaceKind === "project");
  const remoteSessions = preview.sessions.filter((session) => session.workspaceKind === "remote");
  return (
    <div className="grid gap-1 text-xs text-muted-foreground">
      <p>{t("sessionDeletion.irreversible")}</p>
      {projectSessions.length > 0 ? <p data-testid="session-deletion-project-note">{t("sessionDeletion.projectKept")}</p> : null}
      {remoteSessions.length > 0 ? <p data-testid="session-deletion-remote-note">{t("sessionDeletion.remoteKept")}</p> : null}
      {preview.worktrees.length > 0 ? <p>{t("sessionDeletion.worktreeDefaultKept")}</p> : null}
      {preview.runtimeEffect === "simulated" ? <p data-testid="session-deletion-simulated">{t("sessionDeletion.simulatedNotice")}</p> : null}
      {preview.sessions.length > 1 ? (
        <ul className="max-h-32 overflow-y-auto pl-4">
          {preview.sessions.map((session) => (
            <li className="truncate" key={session.sessionId} title={session.displayPath ? normalizeDisplayPath(session.displayPath) : undefined}>{session.title}</li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

/** The one confirmation every visible delete entry point opens. */
export function SessionDeletionDialog({ controller }: { controller: SessionDeletionController }) {
  const { t } = useTranslation();
  const { state } = controller;
  if (state.status === "closed") return null;
  const executing = state.status === "executing";
  const sessions = state.sessions;
  const title = sessions.length === 1
    ? t("sessionDeletion.titleSingle", { title: sessions[0].title })
    : t("sessionDeletion.titleBatch", { count: sessions.length });
  const removalChosen = state.status === "ready" && anyRemovalChosen(state.choices);
  const submittable = state.status === "ready" && canSubmit(state.preview, state.choices);

  const footer = (
    <div className="flex flex-wrap justify-end gap-2">
      {state.status === "settled" && retryAllowed(state.operation) ? (
        <Button data-testid="session-deletion-retry" onClick={() => void controller.retry()} size="sm" type="button" variant="outline">
          {t("sessionDeletion.retry")}
        </Button>
      ) : null}
      {state.status === "ready" || state.status === "preview-failed" ? (
        <Button data-testid="session-deletion-refresh" onClick={controller.refresh} size="sm" type="button" variant="outline">
          {t("sessionDeletion.recheck")}
        </Button>
      ) : null}
      <Button
        data-dialog-autofocus
        data-testid="session-deletion-cancel"
        disabled={executing}
        onClick={controller.close}
        size="sm"
        type="button"
        variant="outline"
      >
        {state.status === "settled" ? t("sessionDeletion.close") : t("layout.cancel")}
      </Button>
      {state.status === "ready" || state.status === "loading" ? (
        <Button
          className="bg-destructive text-destructive-foreground"
          data-testid="session-deletion-confirm"
          disabled={!submittable}
          onClick={() => void controller.confirm()}
          size="sm"
          type="button"
        >
          {t(confirmLabelKey(removalChosen))}
        </Button>
      ) : null}
    </div>
  );

  return (
    <ApplicationDialog
      closeDisabled={executing}
      description={state.status === "ready" || state.status === "loading" ? t("sessionDeletion.description") : undefined}
      footer={footer}
      maxWidth="max-w-lg"
      onClose={controller.close}
      title={title}
    >
      <div className="grid gap-3" data-status={state.status} data-testid="session-deletion-dialog">
        {state.status === "loading" ? <p aria-live="polite" className="text-xs text-muted-foreground" role="status">{t("sessionDeletion.checking")}</p> : null}
        {state.status === "preview-failed" ? (
          <p className="wrap-break-word text-xs text-destructive" role="alert">{t("sessionDeletion.previewFailed", { reason: state.error })}</p>
        ) : null}
        {state.status === "ready" ? (
          <>
            <PreviewSummary preview={state.preview} />
            {state.preview.worktrees.map((worktree) => (
              <SessionDeletionWorktreeRow
                choice={state.choices[worktree.worktreeKey] ?? { remove: false, acknowledgedFingerprint: null }}
                disabled={false}
                key={worktree.worktreeKey}
                onAcknowledge={(acknowledged) => controller.acknowledgeIgnored(worktree, acknowledged)}
                onToggle={() => controller.toggleWorktree(worktree)}
                worktree={worktree}
              />
            ))}
            {state.error ? <p className="wrap-break-word text-xs text-destructive" role="alert">{t("sessionDeletion.executeFailed", { reason: state.error })}</p> : null}
          </>
        ) : null}
        {state.status === "executing" ? <SessionDeletionResult operation={state.operation} sessions={sessions} /> : null}
        {state.status === "settled" ? <SessionDeletionResult operation={state.operation} sessions={sessions} /> : null}
      </div>
    </ApplicationDialog>
  );
}
