import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { GitDiffResult, GitDiffSource, GitStatusEntry } from "../types/session-workspace";
import { cn } from "../lib/utils";
import { DiffView, type DiffViewMode } from "./diff-view";
import { gitStatusPresentation } from "./git-status-presentation";
import { PartialNotice, WorkspaceState } from "./workspace-state";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";
import { ReviewCenter } from "./review-center";

export function ChangesTab({
  isVisible = true,
  sessionId,
}: {
  /** False while the panel stays mounted behind another tab. */
  isVisible?: boolean;
  sessionId: string | null;
}) {
  if (sessionId && typeof agentService.openCodeReview === "function") return <ReviewCenter sessionId={sessionId} />;
  return <LegacyChangesTab isVisible={isVisible} sessionId={sessionId} />;
}

function LegacyChangesTab({ isVisible, sessionId }: { isVisible: boolean; sessionId: string | null }) {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<GitStatusEntry | null>(null);
  const [source, setSource] = useState<GitDiffSource>("working");
  const [mode, setMode] = useState<DiffViewMode>("unified");
  const [diff, setDiff] = useState<GitDiffResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<WorkspaceErrorKey | null>(null);
  const initialized = useRef(false);
  const statusQuery = useQuery({
    // Disabled rather than unmounted while hidden: the status list stays cached and on screen, and
    // the tab stops re-reading the working tree behind another panel.
    enabled: Boolean(sessionId) && isVisible,
    queryKey: ["session-workspace", "git-status", sessionId],
    queryFn: () => agentService.getSessionGitStatus(sessionId ?? ""),
  });

  useEffect(() => {
    setSelected(null); setDiff(null); setError(null);
    initialized.current = false;
  }, [sessionId]);

  useEffect(() => {
    if (statusQuery.data) {
      if (!initialized.current) {
        setSelected(statusQuery.data.items[0] ?? null);
        initialized.current = true;
      }
    }
    if (statusQuery.error) setError(workspaceErrorKey(statusQuery.error));
  }, [statusQuery.data, statusQuery.error]);

  useEffect(() => {
    if (!sessionId || !selected) { setDiff(null); return; }
    let cancelled = false; setLoading(true); setError(null);
    agentService.getSessionGitDiff(sessionId, selected.path, source).then((result) => { if (!cancelled) setDiff(result); })
      .catch((reason: unknown) => { if (!cancelled) setError(workspaceErrorKey(reason)); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [selected, sessionId, source]);

  const gitStatus = statusQuery.data;

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  if ((loading || statusQuery.isLoading) && !gitStatus) return <WorkspaceState kind="loading" />;
  if (error && !gitStatus) return <WorkspaceState kind="error" message={t(error)} />;
  if (gitStatus && !gitStatus.isGit) return <WorkspaceState kind="empty" message={t("sessionTabs.changes.notGit")} />;
  if (gitStatus && gitStatus.items.length === 0) return <WorkspaceState kind="empty" message={t("sessionTabs.changes.clean")} />;

  return (
    <div className="grid h-full min-h-0 gap-3 lg:grid-cols-[220px_minmax(0,1fr)]">
      <section className="min-h-0 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
        {gitStatus?.truncated ? <PartialNotice /> : null}
        <p className="mb-2 truncate px-2 text-xs text-muted-foreground">{gitStatus?.branch ?? t("sessionTabs.changes.detached")}</p>
        {gitStatus?.items.map((entry) => (
          <FileRow entry={entry} isSelected={selected?.path === entry.path} key={entry.path} onClick={() => setSelected(entry)} />
        ))}
      </section>
      <section className="flex min-h-0 flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
        <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border p-2">
          <span className="truncate text-sm font-semibold">{selected?.path}</span>
          <div className="flex gap-1">
            {(["working", "staged"] as const).map((value) => <Toggle active={source === value} key={value} label={t(`sessionTabs.changes.${value}`)} onClick={() => setSource(value)} />)}
            {(["unified", "split"] as const).map((value) => <Toggle active={mode === value} key={value} label={t(`sessionTabs.changes.${value}`)} onClick={() => setMode(value)} />)}
          </div>
        </div>
        <div className="min-h-0 flex-1 overflow-auto p-3">
          {diff?.truncated ? <PartialNotice /> : null}
          <DiffBody diff={diff} error={error} loading={loading} mode={mode} />
        </div>
      </section>
    </div>
  );
}

function FileRow({ entry, isSelected, onClick }: { entry: GitStatusEntry; isSelected: boolean; onClick: () => void }) {
  const { t } = useTranslation();
  const presentation = gitStatusPresentation(entry);
  const label = entry.previousPath ? `${entry.previousPath} → ${entry.path}` : entry.path;
  return (
    <button
      className={cn("flex w-full items-center justify-between gap-2 rounded px-2 py-2 text-left text-sm hover:bg-muted", isSelected && "bg-muted text-primary")}
      onClick={onClick}
      type="button"
    >
      <span className="min-w-0 truncate">{label}</span>
      <span className="flex shrink-0 items-center gap-1 text-xs">
        <span className="font-mono">{presentation.code}</span>
        <span className="text-muted-foreground">{presentation.kinds.map((kind) => t(`sessionTabs.changes.status.${kind}`)).join("/")}</span>
      </span>
    </button>
  );
}

function DiffBody({ diff, error, loading, mode }: { diff: GitDiffResult | null; error: WorkspaceErrorKey | null; loading: boolean; mode: DiffViewMode }) {
  const { t } = useTranslation();
  if (loading) return <WorkspaceState kind="loading" />;
  if (error) return <WorkspaceState kind="error" message={t(error)} />;
  if (!diff || diff.files.length === 0) return <WorkspaceState kind="empty" message={t("sessionTabs.changes.noDiff")} />;
  return (
    <>
      {diff.files.map((file) =>
        file.binary || file.oversized
          ? <WorkspaceState key={file.newPath} kind="unavailable" message={t(file.binary ? "sessionTabs.files.binary" : "sessionTabs.files.oversized")} />
          : <DiffView file={file} key={file.newPath} mode={mode} />
      )}
    </>
  );
}

function Toggle({ active, label, onClick }: { active: boolean; label: string; onClick: () => void }) {
  return (
    <button className={cn("h-7 rounded border border-border px-2 text-xs", active ? "bg-primary text-primary-foreground" : "bg-background text-muted-foreground")} onClick={onClick} type="button">
      {label}
    </button>
  );
}
