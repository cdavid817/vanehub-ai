import { useEffect, useState, type ReactNode } from "react";
import { MessageSquareText, ThumbsDown, ThumbsUp, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useConfirmation } from "../ui/use-confirmation";
import { agentService } from "../../services/runtime-agent-client";
import type { MessageFeedback, MessageFeedbackState } from "../../types/chat";
import { cn } from "../../lib/utils";

function errorText(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "object" && cause !== null && "code" in cause) {
    return String((cause as { code: unknown }).code);
  }
  return String(cause);
}

export function MessageFeedbackControls({
  feedback: initialFeedback,
  messageId,
}: {
  feedback?: MessageFeedback;
  messageId: string;
}) {
  const { t } = useTranslation();
  const { confirm, confirmationDialog } = useConfirmation();
  const [feedback, setFeedback] = useState(initialFeedback);
  const [correction, setCorrection] = useState(initialFeedback?.correctionNote ?? "");
  const [authorizeReusableGuidance, setAuthorizeReusableGuidance] = useState(false);
  const [editing, setEditing] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setFeedback(initialFeedback);
    setCorrection(initialFeedback?.correctionNote ?? "");
    setAuthorizeReusableGuidance(false);
  }, [initialFeedback]);

  const persist = async (
    state: MessageFeedbackState | null,
    correctionNote?: string,
    authorize = false,
  ) => {
    if (
      feedback?.state && state !== feedback.state
      && !(await confirm({ title: t("chat.feedback.replaceConfirm") }))
    ) {
      return;
    }
    setPending(true);
    setError(null);
    try {
      const saved = await agentService.saveMessageFeedback({
        messageId,
        expectedRevision: feedback?.revision ?? 0,
        state,
        ...(correctionNote ? { correctionNote } : {}),
        ...(authorize ? { authorizeReusableGuidance: true } : {}),
      });
      setFeedback(saved);
      setCorrection(saved.correctionNote ?? "");
      setEditing(false);
      setAuthorizeReusableGuidance(false);
    } catch (cause) {
      // Desktop command errors reject with `{ code }` objects; String() would render
      // "[object Object]" and misclassify every conflict as a generic save failure.
      const text = errorText(cause);
      setError(text.includes("feedback-conflict") ? t("chat.feedback.conflict") : t("chat.feedback.saveFailed"));
    } finally {
      setPending(false);
    }
  };

  const revokeAuthorization = async () => {
    if (!(await confirm({ title: t("chat.feedback.authorizationRevokeConfirm") }))) return;
    setPending(true);
    setError(null);
    try {
      await agentService.revokeReusableGuidanceAuthorization({
        messageId,
        expectedFeedbackRevision: feedback?.revision ?? 0,
      });
      if (feedback) {
        const revoked = { ...feedback };
        delete revoked.reusableGuidanceAuthorization;
        setFeedback(revoked);
      }
    } catch (cause) {
      // Desktop command errors reject with `{ code }` objects; String() would render
      // "[object Object]" and misclassify every conflict as a generic save failure.
      const text = errorText(cause);
      setError(text.includes("feedback-conflict") ? t("chat.feedback.conflict") : t("chat.feedback.saveFailed"));
    } finally {
      setPending(false);
    }
  };

  const toggle = (state: MessageFeedbackState) => {
    void persist(feedback?.state === state ? null : state);
  };

  return (
    <div className="mt-2 border-t border-border/60 pt-2" data-testid="message-feedback-controls">
      {confirmationDialog}
      <div className="flex flex-wrap items-center gap-1">
        <FeedbackButton active={feedback?.state === "helpful"} disabled={pending} label={t("chat.feedback.helpful")} onClick={() => toggle("helpful")}>
          <ThumbsUp className="h-3.5 w-3.5" aria-hidden="true" />
        </FeedbackButton>
        <FeedbackButton active={feedback?.state === "unhelpful"} disabled={pending} label={t("chat.feedback.unhelpful")} onClick={() => toggle("unhelpful")}>
          <ThumbsDown className="h-3.5 w-3.5" aria-hidden="true" />
        </FeedbackButton>
        <FeedbackButton active={feedback?.state === "corrected"} disabled={pending} label={t("chat.feedback.corrected")} onClick={() => setEditing(true)}>
          <MessageSquareText className="h-3.5 w-3.5" aria-hidden="true" />
        </FeedbackButton>
        {feedback?.state ? (
          <button aria-label={t("chat.feedback.clear")} className="ml-auto rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground" disabled={pending} onClick={() => void persist(null)} type="button">
            <X className="h-3.5 w-3.5" aria-hidden="true" />
          </button>
        ) : null}
      </div>
      {editing ? (
        <form className="mt-2 grid gap-2" onSubmit={(event) => {
          event.preventDefault();
          if (correction.trim()) {
            void persist("corrected", correction.trim(), authorizeReusableGuidance);
          }
        }}>
          <label className="text-xs font-medium" htmlFor={`feedback-correction-${messageId}`}>{t("chat.feedback.correctionLabel")}</label>
          <textarea autoFocus className="min-h-20 resize-y rounded-md border border-input bg-background px-2 py-1.5 text-xs" id={`feedback-correction-${messageId}`} maxLength={1_000} onChange={(event) => setCorrection(event.target.value)} placeholder={t("chat.feedback.correctionPlaceholder")} value={correction} />
          <label className="flex items-start gap-2 rounded-md border border-border/70 bg-muted/30 p-2 text-xs">
            <input checked={authorizeReusableGuidance} className="mt-0.5 h-4 w-4 accent-primary" onChange={(event) => setAuthorizeReusableGuidance(event.target.checked)} type="checkbox" />
            <span><span className="block font-medium">{t("chat.feedback.authorizationLabel")}</span><span className="mt-0.5 block text-muted-foreground">{t("chat.feedback.authorizationDisclosure")}</span></span>
          </label>
          <div className="flex justify-end gap-2">
            <button className="h-7 rounded px-2 text-xs hover:bg-muted" disabled={pending} onClick={() => setEditing(false)} type="button">{t("chat.feedback.cancel")}</button>
            <button className="h-7 rounded bg-primary px-2 text-xs text-primary-foreground disabled:opacity-50" disabled={pending || !correction.trim()} type="submit">{pending ? t("chat.feedback.saving") : t("chat.feedback.save")}</button>
          </div>
        </form>
      ) : null}
      {feedback?.reusableGuidanceAuthorization && !editing ? (
        <div className="mt-2 flex items-center justify-between gap-2 rounded-md bg-primary/5 px-2 py-1.5 text-xs">
          <span className="text-muted-foreground">{t("chat.feedback.authorizationActive")}</span>
          <button className="font-medium text-destructive hover:underline" disabled={pending} onClick={() => void revokeAuthorization()} type="button">{t("chat.feedback.authorizationRevoke")}</button>
        </div>
      ) : null}
      {error ? <p className="mt-1 text-xs text-destructive" role="alert">{error} {t("chat.feedback.retry")}</p> : null}
    </div>
  );
}

function FeedbackButton({
  active,
  children,
  disabled,
  label,
  onClick,
}: {
  active: boolean;
  children: ReactNode;
  disabled: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button aria-pressed={active} className={cn("inline-flex h-7 items-center gap-1 rounded-md px-2 text-xs text-muted-foreground hover:bg-muted hover:text-foreground", active && "bg-primary/10 text-primary")} disabled={disabled} onClick={onClick} type="button">
      {children}{label}
    </button>
  );
}
