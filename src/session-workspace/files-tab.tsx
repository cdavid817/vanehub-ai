import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { agentService } from "../services/runtime-agent-client";
import { Check, ChevronDown, ChevronRight, Copy, File, Folder } from "lucide-react";
import { useTranslation } from "react-i18next";
import { copyFileReferencePath, writeFileReferenceDrag } from "../services/file-reference-transfer";
import { WorkspaceState } from "./workspace-state";
import { WorkspaceCoverageNotice } from "./workspace-coverage-notice";
import { parentDirectoryOf, selectionStillExists } from "./workspace-invalidation-targets";
import { useWorkspaceFileTree } from "./use-workspace-file-tree";
import { workspaceQueryKeys } from "./workspace-query-keys";
import { QuickOpenDialog } from "./quick-open-dialog";
import { ContentSearchPanel } from "./content-search-panel";
import { FilesToolbar } from "./files-toolbar";
import { FilePreview } from "./file-preview";
import { useFilePreview } from "./use-file-preview";
import { useWorkspaceCapabilities } from "./workspace-capability-notice";

export { flattenFileRows, type TreeRow } from "./use-workspace-file-tree";


export function FilesTab({
  isVisible = true,
  onNavigateToShell,
  onShowEvidence,
  sessionId,
}: {
  /** False while the panel stays mounted behind another tab. */
  isVisible?: boolean;
  /** Absent where nothing owns the tabs, in which case opening a Shell simply creates one. */
  onNavigateToShell?: () => void;
  /** Absent where nothing owns the evidence scope, in which case the action is not offered. */
  onShowEvidence?: (path: string) => void;
  sessionId: string | null;
}) {
  const { t } = useTranslation();
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [copiedPath, setCopiedPath] = useState<string | null>(null);
  const [quickOpen, setQuickOpen] = useState(false);
  const [contentSearch, setContentSearch] = useState(false);
  /**
   * Which line the preview should show first.
   *
   * Held beside the selection rather than folded into it, because they change for different
   * reasons: picking a file in the tree selects without a line, and a content match selects with
   * one. A single value would make "no particular line" and "line 1" the same state.
   */
  const [previewLine, setPreviewLine] = useState<number | null>(null);
  const tree = useWorkspaceFileTree(sessionId, isVisible);
  // Read here rather than inside the toolbar: the same answer decides whether a reveal is possible
  // and, later, what the panel says when a capability is missing. Two reads would be two answers.
  const { capabilities } = useWorkspaceCapabilities(isVisible ? sessionId : null);

  const preview = useFilePreview(sessionId, selectedPath);
  // Asked per selection rather than per file in the tree: the answer is only needed for the one
  // being previewed, and asking for every row would be a query per listing.
  const evidenceLinks = useQuery({
    enabled: Boolean(sessionId) && Boolean(selectedPath),
    queryKey: workspaceQueryKeys.fileEvidence(sessionId ?? "", selectedPath ?? ""),
    queryFn: () => agentService.getFileEvidenceLinks(sessionId ?? "", selectedPath ?? ""),
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

  // The tree's failures still take the panel; a preview failure does not, because the reader may
  // still be reading the file it could not replace.
  const error = tree.error;

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  if (tree.isLoading && !tree.hasRoot) return <WorkspaceState kind="loading" />;
  if (error && !tree.hasRoot) return <WorkspaceState kind="error" message={t(error)} />;

  return (
    <div className="relative grid h-full min-h-0 gap-3 lg:grid-cols-[minmax(180px,0.38fr)_minmax(0,1fr)]">
      <QuickOpenDialog
        isOpen={quickOpen}
        onClose={() => setQuickOpen(false)}
        onSelect={(match) => {
          // A directory is revealed rather than previewed: there is nothing to show in the preview
          // pane for a folder, and opening one would leave the reader looking at an empty panel.
          if (match.kind === "directory") tree.revealDirectory(match.path);
          else {
            tree.revealDirectory(parentDirectoryOf(match.path));
            setSelectedPath(match.path);
          }
        }}
        sessionId={sessionId}
      />
      <ContentSearchPanel
        isOpen={contentSearch}
        onClose={() => setContentSearch(false)}
        onSelect={(match) => {
          tree.revealDirectory(parentDirectoryOf(match.path));
          setSelectedPath(match.path);
          setPreviewLine(match.line);
        }}
        sessionId={sessionId}
      />
      <section className="min-h-0 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
        <FilesToolbar
          isRemote={capabilities?.provider === "ssh"}
          onContentSearch={() => setContentSearch(true)}
          onQuickOpen={() => setQuickOpen(true)}
          onShellOpened={() => onNavigateToShell?.()}
          selectedPath={selectedPath}
          sessionId={sessionId}
        />
        {tree.incompleteReason ? (
          <WorkspaceCoverageNotice
            provider={capabilities?.provider}
            reason="directory-incomplete"
            reasonCode={tree.incompleteReason}
          />
        ) : tree.truncated ? (
          <WorkspaceCoverageNotice provider={capabilities?.provider} reason="directory-page" />
        ) : null}
        {error ? <p className="mb-2 rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground" role="alert">{t(error)}</p> : null}
        {tree.rows.length === 0 ? <WorkspaceState kind="empty" message={t("sessionTabs.files.empty")} /> : tree.rows.map(({ entry, depth }) => (
          <div className="group flex h-8 w-full items-center rounded pr-1 hover:bg-muted" key={entry.path}>
            <button
              className="flex h-8 min-w-0 flex-1 items-center gap-2 rounded px-2 text-left text-sm"
              // Only a file is referenceable, so only a file is draggable.
              draggable={entry.kind === "file"}
              onClick={() => {
                if (entry.kind === "directory") {
                  tree.toggleDirectory(entry.path);
                  return;
                }
                setSelectedPath(entry.path);
                // Picking a file in the tree is not a request to go anywhere in it. Keeping the
                // previous match's line would drop the reader partway down an unrelated file.
                setPreviewLine(null);
              }}
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
        {error ? (
          <WorkspaceState kind="error" message={t(error)} />
        ) : preview.isEmpty && preview.status.kind === "loading" ? (
          <WorkspaceState kind="loading" />
        ) : preview.isEmpty && preview.status.kind === "failed" ? (
          <WorkspaceState kind="error" message={t(preview.status.reason)} />
        ) : !preview.shown ? (
          <WorkspaceState kind="empty" message={t("sessionTabs.files.select")} />
        ) : preview.shown.status !== "text" ? (
          <WorkspaceState kind="unavailable" message={t(`sessionTabs.files.${preview.shown.status}`)} />
        ) : (
          <FilePreview
            file={preview.shown}
            observations={evidenceLinks.data?.observations ?? 0}
            onShowEvidence={onShowEvidence}
            status={preview.status}
            targetLine={previewLine}
          />
        )}
      </section>
    </div>
  );
}
