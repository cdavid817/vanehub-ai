import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { SessionDocument } from "../types/session-workspace";
import { WorkspaceState } from "./workspace-state";
import { WorkspaceCoverageNotice } from "./workspace-coverage-notice";
import { workspaceErrorKey } from "./workspace-error";
import { workspaceQueryKeys } from "./workspace-query-keys";
import { documentOutline } from "./document-outline";
import { DocumentSidebar } from "./document-sidebar";
import { DocumentViewer, type DocumentMode } from "./document-viewer";
import { useFilePreview } from "./use-file-preview";
import {
  useWorkspaceCapabilities,
  WorkspaceCapabilityNotice,
} from "./workspace-capability-notice";

/** How many documents the Recent list holds. Longer than this is a second copy of the full list. */
const MAX_RECENT_DOCUMENTS = 5;

export function DocumentsTab({
  isVisible = true,
  onOpenChanges,
  sessionId,
}: {
  /** False while the panel stays mounted behind another tab. */
  isVisible?: boolean;
  /** Absent where nothing owns the tabs, in which case the action is not offered. */
  onOpenChanges?: (path: string) => void;
  sessionId: string | null;
}) {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<SessionDocument | null>(null);
  const [query, setQuery] = useState("");
  const [mode, setMode] = useState<DocumentMode>("preview");
  const [recentPaths, setRecentPaths] = useState<readonly string[]>([]);
  const [scrollToAnchor, setScrollToAnchor] = useState<string | null>(null);
  const [scrollToLine, setScrollToLine] = useState<number | null>(null);

  const { capabilities } = useWorkspaceCapabilities(isVisible ? sessionId : null);
  // Read here rather than by the viewer: the same status decides whether the action is offered, and
  // a second read would be a second answer that can disagree with the first.
  const statusQuery = useQuery({
    enabled: Boolean(sessionId) && isVisible,
    queryKey: workspaceQueryKeys.gitStatus(sessionId ?? ""),
    queryFn: () => agentService.getSessionGitStatus(sessionId ?? ""),
  });
  const listQuery = useQuery({
    // Nothing is discarded when the tab is hidden: the list, the selection, and the rendered
    // document stay exactly as the reader left them, and only the discovery walk stops.
    enabled: Boolean(sessionId) && isVisible,
    queryKey: workspaceQueryKeys.documents(sessionId ?? ""),
    queryFn: () => agentService.listSessionDocuments(sessionId ?? "", `documents-${sessionId ?? ""}`),
  });
  // The same retention the file preview uses: switching documents, refreshing, and a failed read
  // all leave the last readable one on screen rather than blanking the panel.
  const preview = useFilePreview(sessionId, selected?.path ?? null);

  const documents = useMemo(() => listQuery.data?.items ?? [], [listQuery.data]);
  const outline = useMemo(
    () => documentOutline(preview.shown?.content ?? ""),
    [preview.shown?.content],
  );

  /** Whether Git currently reports the open document as changed. */
  const isChanged = useMemo(
    () =>
      Boolean(selected && statusQuery.data?.items.some((entry) => entry.path === selected.path)),
    [selected, statusQuery.data],
  );

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return needle ? documents.filter((item) => item.path.toLowerCase().includes(needle)) : documents;
  }, [documents, query]);

  const recent = useMemo(
    () =>
      recentPaths
        .map((path) => documents.find((item) => item.path === path))
        // A remembered path whose document is gone is dropped rather than shown as a dead row: the
        // list is a shortcut, and a shortcut to nothing is worse than no shortcut.
        .filter((item): item is SessionDocument => Boolean(item)),
    [documents, recentPaths],
  );

  useEffect(() => {
    setSelected(null);
    setRecentPaths([]);
  }, [sessionId]);

  useEffect(() => {
    // Retained while the refreshed list still holds it. Reselecting the first row on every refresh
    // would move a reader off the document they were reading whenever an agent wrote another one.
    if (documents.length === 0) return;
    if (selected && documents.some((document) => document.path === selected.path)) return;
    setSelected(documents[0] ?? null);
  }, [documents, selected]);

  const openDocument = (document: SessionDocument) => {
    setSelected(document);
    setScrollToAnchor(null);
    setScrollToLine(null);
    setRecentPaths((current) =>
      [document.path, ...current.filter((path) => path !== document.path)].slice(
        0,
        MAX_RECENT_DOCUMENTS,
      ),
    );
  };

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  // Asked before the failure rather than after it: a remote host that cannot read documents
  // produces an empty list, and an empty list is indistinguishable from a workspace that has none.
  if (capabilities && !capabilities.readTextFiles.available) {
    return (
      <WorkspaceCapabilityNotice
        capability={capabilities.readTextFiles}
        targetLabel={capabilities.targetLabel}
      />
    );
  }
  if (listQuery.isLoading && documents.length === 0) return <WorkspaceState kind="loading" />;
  if (listQuery.error && documents.length === 0) {
    return <WorkspaceState kind="error" message={t(workspaceErrorKey(listQuery.error))} />;
  }
  if (documents.length === 0) {
    return <WorkspaceState kind="empty" message={t("sessionTabs.documents.empty")} />;
  }

  return (
    <div className="grid h-full min-h-0 gap-3 lg:grid-cols-[240px_minmax(0,1fr)]">
      <div className="flex min-h-0 flex-col gap-2">
        {listQuery.data?.coverage && listQuery.data.coverage.state !== "complete" ? (
          <WorkspaceCoverageNotice
            provider={capabilities?.provider}
            reason="document-walk"
            reasonCode={listQuery.data.coverage.reasonCode}
          />
        ) : listQuery.data?.truncated ? (
          <WorkspaceCoverageNotice provider={capabilities?.provider} reason="document-walk" />
        ) : null}
        <DocumentSidebar
          documents={filtered}
          onSelect={openDocument}
          onSelectHeading={(entry) => {
            // Both, because the reader may switch modes afterwards and the outline entry should
            // still mean the same place.
            setScrollToAnchor(entry.anchor);
            setScrollToLine(entry.line);
          }}
          outline={outline}
          query={query}
          recent={recent}
          selectedPath={selected?.path ?? null}
          setQuery={setQuery}
        />
      </div>
      {preview.shown ? (
        <DocumentViewer
          content={preview.shown}
          document={selected}
          mode={mode}
          onModeChange={setMode}
          onOpenChanges={
            // Offered only when the document is actually among the working tree's changes. An
            // action that always appeared would open Changes on a file it does not list, which
            // reads as Changes being broken rather than as the document being unmodified.
            isChanged && onOpenChanges && selected
              ? () => onOpenChanges(selected.path)
              : undefined
          }
          outline={outline}
          scrollToAnchor={scrollToAnchor}
          scrollToLine={scrollToLine}
          status={preview.status}
        />
      ) : preview.status.kind === "failed" ? (
        // The list stays usable: a document that could not be read must not take away the ability
        // to pick a different one.
        <WorkspaceState kind="error" message={t(preview.status.reason)} />
      ) : (
        <WorkspaceState kind="loading" />
      )}
    </div>
  );
}
