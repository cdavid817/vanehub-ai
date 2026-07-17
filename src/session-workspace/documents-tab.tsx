import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import { FileText } from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { FileContent, SessionDocument } from "../types/session-workspace";
import { PartialNotice, WorkspaceState } from "./workspace-state";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

export function DocumentsTab({ sessionId }: { sessionId: string | null }) {
  const { t } = useTranslation();
  const [documents, setDocuments] = useState<SessionDocument[]>([]);
  const [selected, setSelected] = useState<SessionDocument | null>(null);
  const [content, setContent] = useState<FileContent | null>(null);
  const [partial, setPartial] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<WorkspaceErrorKey | null>(null);

  useEffect(() => {
    setDocuments([]); setSelected(null); setContent(null); setPartial(false); setError(null);
    if (!sessionId) return;
    let cancelled = false;
    setLoading(true);
    agentService.listSessionDocuments(sessionId).then((result) => {
      if (cancelled) return;
      setDocuments(result.items); setPartial(result.truncated); setSelected(result.items[0] ?? null);
    }).catch((reason: unknown) => {
      if (!cancelled) setError(workspaceErrorKey(reason));
    }).finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [sessionId]);

  useEffect(() => {
    if (!sessionId || !selected) { setContent(null); return; }
    let cancelled = false;
    setLoading(true); setError(null);
    agentService.readSessionFile(sessionId, selected.path).then((result) => {
      if (!cancelled) setContent(result);
    }).catch((reason: unknown) => {
      if (!cancelled) setError(workspaceErrorKey(reason));
    }).finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [selected, sessionId]);

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
        ) : <pre className="whitespace-pre-wrap break-words text-sm leading-6">{content.content}</pre>}
      </article>
    </div>
  );
}
