import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import ReactMarkdown from "react-markdown";
import { FileText } from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { FileContent, SessionDocument } from "../types/session-workspace";
import { PartialNotice, WorkspaceState } from "./workspace-state";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";
import { workspaceQueryKeys } from "./workspace-query-keys";

export function DocumentsTab({
  isVisible = true,
  sessionId,
}: {
  /** False while the panel stays mounted behind another tab. */
  isVisible?: boolean;
  sessionId: string | null;
}) {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<SessionDocument | null>(null);

  // Queries rather than effects, so a change notice can refresh the list and the open document.
  // The imperative version had no key to invalidate, which meant a document an agent had just
  // written stayed missing from the list until the whole tab was rebuilt.
  const listQuery = useQuery({
    // Nothing is discarded when the tab is hidden: the list, the selection, and the rendered
    // document stay exactly as the reader left them, and only the discovery walk stops.
    enabled: Boolean(sessionId) && isVisible,
    queryKey: workspaceQueryKeys.documents(sessionId ?? ""),
    queryFn: () => agentService.listSessionDocuments(sessionId ?? ""),
  });
  const contentQuery = useQuery({
    enabled: Boolean(sessionId) && Boolean(selected),
    queryKey: workspaceQueryKeys.preview(sessionId ?? "", selected?.path ?? ""),
    queryFn: () => agentService.readSessionFile(sessionId ?? "", selected?.path ?? ""),
  });

  const documents: SessionDocument[] = useMemo(() => listQuery.data?.items ?? [], [listQuery.data]);
  const partial = listQuery.data?.truncated ?? false;
  const content: FileContent | null = contentQuery.data ?? null;
  const loading = listQuery.isLoading || contentQuery.isLoading;
  const error: WorkspaceErrorKey | null = listQuery.error
    ? workspaceErrorKey(listQuery.error)
    : contentQuery.error
      ? workspaceErrorKey(contentQuery.error)
      : null;

  useEffect(() => {
    setSelected(null);
  }, [sessionId]);

  useEffect(() => {
    // Retained while the refreshed list still holds it. Reselecting the first row on every refresh
    // would move a reader off the document they were reading whenever an agent wrote another one.
    if (documents.length === 0) return;
    if (selected && documents.some((document) => document.path === selected.path)) return;
    setSelected(documents[0] ?? null);
  }, [documents, selected]);

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  if (loading && documents.length === 0) return <WorkspaceState kind="loading" />;
  if (error && documents.length === 0) return <WorkspaceState kind="error" message={t(error)} />;
  if (documents.length === 0) return <WorkspaceState kind="empty" message={t("sessionTabs.documents.empty")} />;

  return (
    <div className="grid h-full min-h-0 gap-3 lg:grid-cols-[220px_minmax(0,1fr)]">
      <section className="min-h-0 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
        {partial ? <PartialNotice /> : null}
        {documents.map((document) => <button className="flex h-9 w-full items-center gap-2 rounded px-2 text-left text-sm hover:bg-muted" key={document.path} onClick={() => setSelected(document)} type="button"><FileText className="h-4 w-4 text-primary" /><span className="truncate">{document.path}</span></button>)}
      </section>
      <article className="min-h-0 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-4">
        {loading ? <WorkspaceState kind="loading" /> : error ? <WorkspaceState kind="error" message={t(error)} /> : !content ? <WorkspaceState kind="empty" /> : content.status !== "text" ? <WorkspaceState kind="unavailable" message={t(`sessionTabs.files.${content.status}`)} /> : selected?.kind === "markdown" ? (
          <div className="grid max-w-none gap-3 text-sm leading-6 text-foreground [&_a]:text-primary [&_a]:underline [&_code]:rounded [&_code]:bg-muted [&_code]:px-1 [&_h1]:text-2xl [&_h1]:font-semibold [&_h2]:text-xl [&_h2]:font-semibold [&_li]:ml-5 [&_li]:list-disc [&_p]:whitespace-pre-wrap"><ReactMarkdown skipHtml>{content.content ?? ""}</ReactMarkdown></div>
        ) : <pre className="whitespace-pre-wrap wrap-break-word text-sm leading-6">{content.content}</pre>}
      </article>
    </div>
  );
}
