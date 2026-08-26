import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Check, ChevronDown, ChevronRight, Copy, File, Folder } from "lucide-react";
import { useTranslation } from "react-i18next";
import { copyFileReferencePath, writeFileReferenceDrag } from "../services/file-reference-transfer";
import { agentService } from "../services/runtime-agent-client";
import { PartialNotice, WorkspaceState } from "./workspace-state";
import { workspaceErrorKey } from "./workspace-error";
import { parentDirectoryOf, selectionStillExists } from "./workspace-invalidation-targets";
import { workspaceQueryKeys } from "./workspace-query-keys";
import { useWorkspaceFileTree } from "./use-workspace-file-tree";

export { flattenFileRows, type TreeRow } from "./use-workspace-file-tree";

export function FilesTab({
  isVisible = true,
  sessionId,
}: {
  /** False while the panel stays mounted behind another tab. */
  isVisible?: boolean;
  sessionId: string | null;
}) {
  const { t } = useTranslation();
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [copiedPath, setCopiedPath] = useState<string | null>(null);
  const tree = useWorkspaceFileTree(sessionId, isVisible);

  const previewQuery = useQuery({
    enabled: Boolean(sessionId) && Boolean(selectedPath),
    queryKey: workspaceQueryKeys.preview(sessionId ?? "", selectedPath ?? ""),
    queryFn: () => agentService.readSessionFile(sessionId ?? "", selectedPath ?? ""),
  });

  useEffect(() => {
    setSelectedPath(null);
  }, [sessionId]);

  useEffect(() => {
    // Retained while its parent listing still holds it, dropped when the refreshed listing says it
    // is gone. Reacting to the change notice instead would drop a selection the moment a file was
    // *edited* — the moment a reader most wants to keep looking at it.
    if (!selectedPath) return;
    const siblings = tree.entriesByPath[parentDirectoryOf(selectedPath)];
    if (!selectionStillExists(selectedPath, siblings)) setSelectedPath(null);
  }, [selectedPath, tree.entriesByPath]);

  const preview = previewQuery.data ?? null;
  const error = tree.error ?? (previewQuery.error ? workspaceErrorKey(previewQuery.error) : null);

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  if (tree.isLoading && !tree.hasRoot) return <WorkspaceState kind="loading" />;
  if (error && !tree.hasRoot) return <WorkspaceState kind="error" message={t(error)} />;

  return (
    <div className="grid h-full min-h-0 gap-3 lg:grid-cols-[minmax(180px,0.38fr)_minmax(0,1fr)]">
      <section className="min-h-0 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
        {tree.truncated ? <PartialNotice /> : null}
        {error ? <p className="mb-2 rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground" role="alert">{t(error)}</p> : null}
        {tree.rows.length === 0 ? <WorkspaceState kind="empty" message={t("sessionTabs.files.empty")} /> : tree.rows.map(({ entry, depth }) => (
          <div className="group flex h-8 w-full items-center rounded pr-1 hover:bg-muted" key={entry.path}>
            <button
              className="flex h-8 min-w-0 flex-1 items-center gap-2 rounded px-2 text-left text-sm"
              // Only a file is referenceable, so only a file is draggable.
              draggable={entry.kind === "file"}
              onClick={() => entry.kind === "directory" ? tree.toggleDirectory(entry.path) : setSelectedPath(entry.path)}
              onDragStart={(event) => writeFileReferenceDrag(event.dataTransfer, entry.path)}
              type="button"
            >
              <span aria-hidden="true" className="shrink-0 text-muted-foreground">{"·".repeat(depth)}</span>
              {entry.kind === "directory" ? (tree.isOpen(entry.path) ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />) : <span className="w-3.5" />}
              {entry.kind === "directory" ? <Folder className="h-4 w-4 text-primary" /> : <File className="h-4 w-4 text-muted-foreground" />}
              <span className="truncate">{entry.name}</span>
            </button>
            {entry.kind === "file" ? (
              <button
                className="shrink-0 rounded p-1 text-muted-foreground opacity-0 focus-visible:opacity-100 group-hover:opacity-100 hover:text-foreground"
                data-testid={`copy-path-${entry.path}`}
                onClick={() => { void copyFileReferencePath(entry.path).then(() => setCopiedPath(entry.path)).catch(() => setCopiedPath(null)); }}
                title={t("sessionTabs.files.copyPath")}
                type="button"
              >
                {copiedPath === entry.path ? <Check className="h-3.5 w-3.5 text-primary" aria-hidden="true" /> : <Copy className="h-3.5 w-3.5" aria-hidden="true" />}
                <span className="sr-only">{copiedPath === entry.path ? t("sessionTabs.files.copyPathDone") : t("sessionTabs.files.copyPath")}</span>
              </button>
            ) : null}
          </div>
        ))}
      </section>
      <section className="min-h-0 overflow-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
        {previewQuery.isLoading && selectedPath ? <WorkspaceState kind="loading" /> : error ? <WorkspaceState kind="error" message={t(error)} /> : !preview ? <WorkspaceState kind="empty" message={t("sessionTabs.files.select")} /> : preview.status !== "text" ? <WorkspaceState kind="unavailable" message={t(`sessionTabs.files.${preview.status}`)} /> : (
          <><h3 className="mb-3 truncate text-sm font-semibold">{preview.path}</h3><pre className="whitespace-pre-wrap wrap-break-word font-mono text-xs leading-5">{preview.content}</pre></>
        )}
      </section>
    </div>
  );
}
