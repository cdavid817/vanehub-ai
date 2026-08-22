import { useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { ReviewAction, ReviewAnchor, ReviewDecision, ReviewDiffHunk } from "../types/code-review";
import { cn } from "../lib/utils";
import { WorkspaceState } from "./workspace-state";
import { useCodeReview } from "./use-code-review";
import { useReviewAction } from "./use-review-action";
import { ApplicationDialog } from "../components/ui/application-dialog";

export function ReviewCenter({ sessionId }: { sessionId: string }) {
  const { t } = useTranslation();
  const state = useCodeReview(sessionId);
  const [mode, setMode] = useState<"unified" | "split">("unified");
  const [draft, setDraft] = useState("");
  const [anchor, setAnchor] = useState<Omit<ReviewAnchor, "state"> | null>(null);
  const [receipt, setReceipt] = useState<string | null>(null);
  const review = state.review;
  if (state.loading) return <WorkspaceState kind="loading" />;
  if (!review) return <WorkspaceState kind="error" message={state.error ?? t("sessionTabs.review.error")} />;
  if (review.files.length === 0) return <WorkspaceState kind="empty" message={t("sessionTabs.changes.clean")} />;
  return <ReviewCenterContent mode={mode} setMode={setMode} draft={draft} setDraft={setDraft} anchor={anchor} setAnchor={setAnchor} receipt={receipt} setReceipt={setReceipt} sessionId={sessionId} state={state} review={review} />;
}

function ReviewCenterContent({ mode, setMode, draft, setDraft, anchor, setAnchor, receipt, setReceipt, sessionId, state, review }: { mode: "unified" | "split"; setMode: (value: "unified" | "split") => void; draft: string; setDraft: (value: string) => void; anchor: Omit<ReviewAnchor, "state"> | null; setAnchor: (value: Omit<ReviewAnchor, "state"> | null) => void; receipt: string | null; setReceipt: (value: string | null) => void; sessionId: string; state: ReturnType<typeof useCodeReview>; review: NonNullable<ReturnType<typeof useCodeReview>["review"]> }) {
  const { t } = useTranslation();
  const actionState = useReviewAction(review.id);
  const [railOpen, setRailOpen] = useState(true);
  const [pending, setPending] = useState<{ kind: "revert"; hunk?: string } | { kind: "stale-feedback" } | null>(null);
  const [hunkError, setHunkError] = useState<string | null>(null);
  const selectedIndex = review.files.findIndex((file) => file.path === state.selectedPath);
  const move = (offset: number) => state.setSelectedPath(review.files[(selectedIndex + offset + review.files.length) % review.files.length].path);
  // What the panel renders, not an apply-ready patch: no file or hunk headers, and truncated
  // content is copied as truncated. Copy Standard Patch is the witnessed one; Task Group 13
  // implements the backend operation that can produce it.
  const copyDisplayedLines = () => navigator.clipboard.writeText(state.diff?.hunks.flatMap((hunk) => hunk.lines.map((line) => line.content)).join("\n") ?? "");

  const decide = async (decision: ReviewDecision) => state.replaceReview(
    await agentService.setCodeReviewDecision(review.id, decision),
  );
  // Hunk acceptance is its own operation. Routing it through `decide` accepted the whole review.
  const decideHunk = async (hunkFingerprint: string, decision: ReviewDecision) => {
    if (!state.selectedPath) return;
    setHunkError(null);
    try {
      await agentService.setCodeReviewHunkDecision({
        reviewId: review.id,
        relativePath: state.selectedPath,
        hunkFingerprint,
        expectedSnapshotFingerprint: review.fingerprint,
        decision,
      });
    } catch (reason: unknown) {
      setHunkError(String(reason).includes("review_hunk_decision_unavailable")
        ? t("sessionTabs.review.hunkDecisionUnavailable")
        : t("sessionTabs.review.hunkDecisionFailed"));
    }
  };
  const submitComment = async () => {
    if (!anchor || !draft.trim()) return;
    await agentService.addCodeReviewComment({ reviewId: review.id, anchor, body: draft });
    setDraft(""); setAnchor(null);
    state.replaceReview(await agentService.getCodeReview(review.id));
  };
  const sendFeedback = async (acknowledgeStale = false) => {
    try {
      await agentService.sendCodeReviewFeedback(review.id, acknowledgeStale);
    } catch (reason: unknown) {
      if (!String(reason).includes("stale")) throw reason;
      setPending({ kind: "stale-feedback" });
    }
  };
  const revert = (hunkFingerprint?: string) => setPending({ kind: "revert", hunk: hunkFingerprint });
  const performRevert = async (hunkFingerprint?: string) => {
    if (!state.selectedPath) return;
    const result = await agentService.revertCodeReviewChange({
      sessionId,
      path: state.selectedPath,
      expectedSnapshot: review.fingerprint,
      hunkFingerprint,
      confirmed: true,
    });
    setReceipt(result.simulated ? t("sessionTabs.review.simulatedRevert") : t("sessionTabs.review.reverted"));
    await state.open();
  };
  return (
    <div className={cn("grid h-full min-h-0 gap-3", railOpen && "md:grid-cols-[minmax(180px,240px)_minmax(0,1fr)]")} data-testid="review-center">
      {railOpen ? <aside className="max-h-40 min-h-0 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2 md:max-h-none">
        <p className="truncate px-2 pb-1 text-xs text-muted-foreground">{review.workspaceId}</p>
        <p className="px-2 pb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">{t("sessionTabs.review.files", { count: review.files.length })}</p>
        {review.files.map((file) => <button className={cn("block w-full truncate rounded px-2 py-2 text-left text-sm hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring", state.selectedPath === file.path && "bg-muted text-primary")} key={file.path} onClick={() => state.setSelectedPath(file.path)} type="button">{file.path}</button>)}
      </aside> : null}
      <section className="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
        <header className="flex flex-wrap items-center gap-2 border-b border-border p-2">
          <button aria-expanded={railOpen} className="rounded border border-border px-2 py-1 text-xs" onClick={() => setRailOpen((value) => !value)} type="button">{t("sessionTabs.review.toggleFiles")}</button>
          <strong className="mr-auto min-w-0 truncate text-sm">{state.selectedPath}</strong>
          <button aria-label={t("sessionTabs.review.previous")} className="rounded border border-border px-2 py-1 text-xs" onClick={() => move(-1)} type="button">←</button>
          <button aria-label={t("sessionTabs.review.next")} className="rounded border border-border px-2 py-1 text-xs" onClick={() => move(1)} type="button">→</button>
          <button className="rounded border border-border px-2 py-1 text-xs" onClick={() => void copyDisplayedLines()} type="button">{t("sessionTabs.review.copyDisplayedLines")}</button>
          <button className="rounded border border-border px-2 py-1 text-xs" disabled title={t("sessionTabs.review.copyStandardPatchPending")} type="button">{t("sessionTabs.review.copyStandardPatch")}</button>
          <Toggle active={mode === "unified"} label={t("sessionTabs.changes.unified")} onClick={() => setMode("unified")} />
          <Toggle active={mode === "split"} label={t("sessionTabs.changes.split")} onClick={() => setMode("split")} />
          <button className="rounded border border-destructive/50 px-2 py-1 text-xs text-destructive" onClick={() => revert()} type="button">{t("sessionTabs.review.revertFile")}</button>
        </header>
        <div className="min-h-0 flex-1 overflow-auto p-3" data-mode={mode}>
          {state.refreshing ? <p className="text-xs text-muted-foreground">{t("sessionTabs.state.loading")}</p> : null}
          {state.error ? <p role="alert" className="text-sm text-destructive">{state.error}</p> : null}
          {state.diff?.binary || state.diff?.oversized ? <WorkspaceState kind="unavailable" message={t(state.diff.binary ? "sessionTabs.files.binary" : "sessionTabs.files.oversized")} /> : null}
          {state.diff?.truncated ? <p className="text-xs text-warning">{t("sessionTabs.review.truncated")}</p> : null}
          {hunkError ? <p role="alert" className="text-sm text-destructive">{hunkError}</p> : null}
          {state.diff?.hunks.map((hunk) => <ReviewHunk hunk={hunk} key={hunk.fingerprint} mode={mode} onAccept={() => void decideHunk(hunk.fingerprint, "accepted")} onAnchor={setAnchor} onRevert={() => revert(hunk.fingerprint)} path={state.selectedPath ?? ""} />)}
        </div>
        <footer className="space-y-2 border-t border-border p-3">
          {receipt ? <p role="status" className="text-xs text-muted-foreground">{receipt}</p> : null}
          {actionState.operation ? <div aria-live="polite" className="rounded border border-border p-2 text-xs"><p>{actionState.operation.message}: {actionState.operation.status}</p>{actionState.operation.logs.map((log) => <pre className="overflow-x-auto whitespace-pre-wrap" key={`${log.timestamp}-${log.line}`}>{log.line}</pre>)}{!["succeeded", "failed", "cancelled"].includes(actionState.operation.status) ? <button className="text-destructive" onClick={() => void actionState.cancel()} type="button">{t("confirmation.cancel")}</button> : null}</div> : null}
          {actionState.error ? <p role="alert" className="text-xs text-destructive">{actionState.error}</p> : null}
          {review.findings.map((finding) => <p className="rounded border border-border p-2 text-xs" key={finding.id}>{finding.severity}: {finding.title}</p>)}
          {review.comments.map((comment) => <div className="flex items-start gap-2 rounded border border-border p-2 text-xs" key={comment.id}><input aria-label={t("sessionTabs.review.selectComment")} checked={comment.selected} onChange={(event) => void agentService.selectCodeReviewComment(review.id, comment.id, event.target.checked).then(state.replaceReview)} type="checkbox" /><p className="min-w-0 flex-1 break-words">{comment.body}</p>{comment.status === "active" ? <button className="text-primary" onClick={() => void agentService.resolveCodeReviewComment(review.id, comment.id).then(state.replaceReview)} type="button">{t("sessionTabs.review.resolve")}</button> : <span>{t("sessionTabs.review.resolved")}</span>}</div>)}
          {anchor ? <textarea aria-label={t("sessionTabs.review.comment")} className="min-h-20 w-full rounded border border-border bg-background p-2 text-sm" maxLength={8192} onChange={(event) => setDraft(event.target.value)} value={draft} /> : null}
          <div className="flex flex-wrap gap-2">
            {anchor ? <button className="rounded bg-primary px-3 py-1 text-xs text-primary-foreground" onClick={() => void submitComment()} type="button">{t("sessionTabs.review.addComment")}</button> : null}
            <button className="rounded border border-border px-3 py-1 text-xs" onClick={() => void decide("changes-requested")} type="button">{t("sessionTabs.review.requestChanges")}</button>
            <button className="rounded bg-primary px-3 py-1 text-xs text-primary-foreground" onClick={() => void decide("accepted")} type="button">{t("sessionTabs.review.accept")}</button>
            {(["review-agent", "tests", "security"] as ReviewAction[]).map((action) => <button className="rounded border border-border px-3 py-1 text-xs" key={action} onClick={() => void actionState.start(action)} type="button">{t(`sessionTabs.review.action.${action}`)}</button>)}
            <button className="rounded border border-border px-3 py-1 text-xs" onClick={() => void sendFeedback()} type="button">{t("sessionTabs.review.sendFeedback")}</button>
          </div>
        </footer>
      </section>
      {pending ? <ApplicationDialog footer={<div className="flex justify-end gap-2"><button className="rounded border border-border px-3 py-1 text-sm" onClick={() => setPending(null)} type="button">{t("confirmation.cancel")}</button><button className="rounded bg-destructive px-3 py-1 text-sm text-destructive-foreground" data-dialog-autofocus onClick={() => { const action = pending; setPending(null); if (action.kind === "revert") void performRevert(action.hunk); else void sendFeedback(true); }} type="button">{t("confirmation.confirm")}</button></div>} onClose={() => setPending(null)} title={t(pending.kind === "revert" ? "sessionTabs.review.revertFile" : "sessionTabs.review.sendFeedback")}><p className="text-sm text-muted-foreground">{t(pending.kind === "revert" ? "sessionTabs.review.revertConfirm" : "sessionTabs.review.staleConfirm")}</p></ApplicationDialog> : null}
    </div>
  );
}

function ReviewHunk({ hunk, mode, path, onAccept, onAnchor, onRevert }: { hunk: ReviewDiffHunk; mode: "unified" | "split"; path: string; onAccept: () => void; onAnchor: (anchor: Omit<ReviewAnchor, "state">) => void; onRevert: () => void }) {
  const { t } = useTranslation();
  const choose = (index: number) => {
    const line = hunk.lines[index];
    onAnchor({ filePath: path, side: line.kind === "deletion" ? "old" : "new", startLine: line.oldLineNumber ?? line.newLineNumber ?? 1, endLine: line.oldLineNumber ?? line.newLineNumber ?? 1, hunkFingerprint: hunk.fingerprint, contextFingerprint: hunk.contextFingerprints[index] ?? hunk.fingerprint });
  };
  return <article className="mb-3 min-w-max overflow-hidden rounded border border-border"><div className="flex items-center justify-between gap-2 bg-muted px-2 py-1"><code className="text-xs">{hunk.header}</code><span><button aria-label={t("sessionTabs.review.acceptHunk")} className="mr-2 text-primary" onClick={onAccept} type="button">✓</button><button aria-label={t("sessionTabs.review.revertHunk")} className="text-destructive" onClick={onRevert} type="button">↶</button></span></div>{mode === "unified" ? hunk.lines.map((line, index) => <button className={cn("grid w-full grid-cols-[4rem_4rem_minmax(20rem,1fr)] font-mono text-xs hover:bg-primary/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring", line.kind === "addition" && "bg-success/10", line.kind === "deletion" && "bg-destructive/10")} key={`${hunk.fingerprint}-${index}`} onClick={() => choose(index)} type="button"><span>{line.oldLineNumber}</span><span>{line.newLineNumber}</span><span className="whitespace-pre text-left">{line.content}</span></button>) : <div className="grid min-w-[48rem] grid-cols-2 divide-x divide-border">{hunk.lines.map((line, index) => <button className={cn("col-span-2 grid grid-cols-2 font-mono text-xs hover:bg-primary/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring", line.kind === "addition" && "bg-success/10", line.kind === "deletion" && "bg-destructive/10")} key={`${hunk.fingerprint}-split-${index}`} onClick={() => choose(index)} type="button"><span className="grid grid-cols-[4rem_1fr] px-1 text-left"><span>{line.oldLineNumber}</span><span className="whitespace-pre">{line.kind === "addition" ? "" : line.content}</span></span><span className="grid grid-cols-[4rem_1fr] px-1 text-left"><span>{line.newLineNumber}</span><span className="whitespace-pre">{line.kind === "deletion" ? "" : line.content}</span></span></button>)}</div>}</article>;
}

function Toggle({ active, label, onClick }: { active: boolean; label: string; onClick: () => void }) {
  return <button aria-pressed={active} className={cn("rounded border border-border px-2 py-1 text-xs", active && "bg-primary text-primary-foreground")} onClick={onClick} type="button">{label}</button>;
}
