import { CheckCircle2, History, RotateCcw, ShieldAlert } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import { normalizeSkillOverlayError } from "../../../services/skill-overlay-error";
import type {
  SkillOverlayDetail,
  SkillOverlayHistoryEntry,
  SkillOverlayHistoryIntegrity,
  SkillOverlayMutationSummary,
  SkillOverlayTargetInput,
} from "../../../types/skill-overlay";
import { SkillOverlayHistoryEntryCard } from "./skill-overlay-history-entry";
import { SkillOverlayHistoryRevertDialog } from "./skill-overlay-history-revert-dialog";
import { SKILL_OVERLAY_PINNED_DESCRIPTION_ID } from "./skill-overlay-pinned-notice";

const HISTORY_PAGE_SIZE = 20;

interface RevertSelection {
  mutation: SkillOverlayMutationSummary;
  trigger: HTMLElement | null;
}

export function SkillOverlayHistory({ detail, target, onCommitted, onRefresh }: {
  detail: SkillOverlayDetail;
  target: SkillOverlayTargetInput;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
}) {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<SkillOverlayHistoryEntry[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [integrity, setIntegrity] = useState<SkillOverlayHistoryIntegrity>("verified");
  const [pending, setPending] = useState<"initial" | "more" | null>("initial");
  const [error, setError] = useState<string | null>(null);
  const [lastRevertedRevision, setLastRevertedRevision] = useState<number | null>(null);
  const [selection, setSelection] = useState<RevertSelection | null>(null);
  const mountedRef = useRef(true);
  const queryPage = useCallback((cursor: string | null) => agentService.getSkillOverlayHistory({
    target: {
      skillId: target.skillId,
      scope: target.scope,
      workspacePath: target.scope === "project" ? target.workspacePath ?? null : null,
    },
    cursor,
    limit: HISTORY_PAGE_SIZE,
  }), [target.scope, target.skillId, target.workspacePath]);

  const loadInitial = useCallback(async () => {
    setPending("initial");
    setError(null);
    try {
      const page = await queryPage(null);
      if (!mountedRef.current) return;
      setEntries(page.entries);
      setNextCursor(page.nextCursor);
      setIntegrity(page.integrity);
    } catch (caught) {
      if (!mountedRef.current) return;
      setError(normalizeSkillOverlayError(caught).message);
    } finally {
      if (mountedRef.current) setPending(null);
    }
  }, [queryPage]);

  useEffect(() => {
    mountedRef.current = true;
    setLastRevertedRevision(null);
    void loadInitial();
    return () => { mountedRef.current = false; };
  }, [loadInitial]);

  async function loadMore() {
    if (!nextCursor) return;
    setPending("more");
    setError(null);
    try {
      const page = await queryPage(nextCursor);
      setEntries((current) => mergeEntries(current, page.entries));
      setNextCursor(page.nextCursor);
      setIntegrity((current) => current === "verified" ? page.integrity : current);
    } catch (caught) {
      setError(normalizeSkillOverlayError(caught).message);
    } finally {
      setPending(null);
    }
  }

  function reverted(revision: number) {
    setLastRevertedRevision(revision);
    void loadInitial();
  }

  const verified = integrity === "verified";
  const revertable = detail.mutations.filter((mutation) => mutation.scope === target.scope && mutation.state !== "reverted");
  return <section aria-labelledby="skill-overlay-history-title" className="rounded-md border border-border bg-muted/10 p-3">
    <div className="flex flex-wrap items-start justify-between gap-2">
      <div>
        <h5 className="flex items-center gap-2 text-xs font-semibold" id="skill-overlay-history-title"><History className="h-4 w-4" />{t("skills.overlay.history.title")}</h5>
        <p className="mt-1 text-[11px] leading-4 text-muted-foreground">{t("skills.overlay.history.description")}</p>
      </div>
      <Badge tone={verified ? "success" : "danger"}>
        {verified ? <CheckCircle2 className="mr-1 h-3 w-3" /> : <ShieldAlert className="mr-1 h-3 w-3" />}
        {t(verified ? "skills.overlay.history.verified" : "skills.overlay.history.failed")}
      </Badge>
    </div>

    {!verified ? <div className="mt-3 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive" role="alert">
      <p className="font-semibold">{t("skills.overlay.history.integrityFailure")}</p>
      <p className="mt-1 break-all font-mono">{integrity.slice("failed:".length)}</p>
      <p className="mt-1 leading-5">{t("skills.overlay.history.integrityFailureHint")}</p>
    </div> : null}
    {lastRevertedRevision != null ? <p className="mt-3 rounded-md border border-success/40 bg-success/10 p-3 text-xs" role="status">
      {t("skills.overlay.history.revertSuccess", { revision: lastRevertedRevision })}
    </p> : null}
    {error ? <div className="mt-3 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive" role="alert">
      <p>{t("skills.overlay.history.loadError")}</p><p className="mt-1">{error}</p>
      <Button className="mt-2" disabled={pending !== null} onClick={() => void loadInitial()} size="sm" variant="outline">{t("featureLoad.retry")}</Button>
    </div> : null}

    {pending === "initial" && entries.length === 0 ? <p className="mt-3 text-xs text-muted-foreground" role="status">{t("skills.overlay.history.loading")}</p> : null}
    {pending !== "initial" && entries.length === 0 && !error ? <p className="mt-3 rounded-md border border-dashed border-border p-3 text-xs text-muted-foreground">{t("skills.overlay.history.empty")}</p> : null}
    {entries.length > 0 ? <ol className="relative mt-4 space-y-3 before:absolute before:bottom-3 before:left-2.5 before:top-3 before:w-px before:bg-border">
      {entries.map((entry) => <SkillOverlayHistoryEntryCard entry={entry} key={entry.eventId} />)}
    </ol> : null}
    {nextCursor ? <div className="mt-3 flex justify-center border-t border-border pt-3">
      <Button disabled={pending !== null} onClick={() => void loadMore()} variant="outline">{t(pending === "more" ? "skills.overlay.history.loadingMore" : "skills.overlay.history.loadMore")}</Button>
    </div> : null}

    {revertable.length > 0 ? <div className="mt-4 border-t border-border pt-4">
      <h6 className="text-xs font-semibold">{t("skills.overlay.history.revertableTitle")}</h6>
      <p className="mt-1 text-[11px] leading-4 text-muted-foreground">{t("skills.overlay.history.revertableDescription")}</p>
      <ul className="mt-3 grid gap-2 sm:grid-cols-2">
        {revertable.map((mutation) => <li className="flex min-w-0 items-center justify-between gap-2 rounded-md border border-border bg-background p-3" key={mutation.id}>
          <div className="min-w-0"><p className="truncate font-mono text-xs" title={mutation.id}>{mutation.id}</p><p className="mt-1 text-[11px] text-muted-foreground">{t(`skills.overlay.mutations.${mutation.kind}`)} · {t(`skills.overlay.history.mutationState.${mutation.state}`)}</p></div>
          <Button aria-describedby={detail.summary.pinned ? SKILL_OVERLAY_PINNED_DESCRIPTION_ID : undefined} disabled={!verified || detail.summary.pinned} onClick={(event) => setSelection({ mutation, trigger: event.currentTarget })} size="sm" variant="outline"><RotateCcw />{t("skills.overlay.history.revert")}</Button>
        </li>)}
      </ul>
    </div> : null}

    {selection ? <SkillOverlayHistoryRevertDialog detail={detail} mutation={selection.mutation} onClose={() => setSelection(null)} onCommitted={onCommitted} onRefresh={onRefresh} onReverted={reverted} returnFocus={selection.trigger} target={target} /> : null}
  </section>;
}

function mergeEntries(current: SkillOverlayHistoryEntry[], next: SkillOverlayHistoryEntry[]) {
  const existing = new Set(current.map((entry) => entry.eventId));
  return [...current, ...next.filter((entry) => !existing.has(entry.eventId))];
}
