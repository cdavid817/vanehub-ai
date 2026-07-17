import { useCallback, useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { ChevronDown, ChevronRight, File, Folder } from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { DirectoryEntry, FileContent } from "../types/session-workspace";
import { PartialNotice, WorkspaceState } from "./workspace-state";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

export interface TreeRow {
  entry: DirectoryEntry;
  depth: number;
}

export function flattenFileRows(entriesByPath: Record<string, DirectoryEntry[]>, expanded: ReadonlySet<string>) {
  const result: TreeRow[] = [];
  const visit = (parent: string, depth: number) => {
    for (const entry of entriesByPath[parent] ?? []) {
      result.push({ entry, depth });
      if (entry.kind === "directory" && expanded.has(entry.path)) visit(entry.path, depth + 1);
    }
  };
  visit("", 0);
  return result;
}

export function FilesTab({ sessionId }: { sessionId: string | null }) {
  const { t } = useTranslation();
  const [entriesByPath, setEntriesByPath] = useState<Record<string, DirectoryEntry[]>>({});
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [preview, setPreview] = useState<FileContent | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<WorkspaceErrorKey | null>(null);
  const [partial, setPartial] = useState(false);
  const rootQuery = useQuery({
    enabled: Boolean(sessionId),
    queryKey: ["session-workspace", "directory", sessionId, ""],
    queryFn: () => agentService.listSessionDirectory(sessionId ?? "", ""),
  });

  const loadDirectory = useCallback(async (path: string) => {
    if (!sessionId) return;
    const listing = await agentService.listSessionDirectory(sessionId, path);
    setEntriesByPath((current) => ({ ...current, [path]: listing.items }));
    setPartial((current) => current || listing.truncated);
  }, [sessionId]);

  useEffect(() => {
    setEntriesByPath({});
    setExpanded(new Set());
    setSelectedPath(null);
    setPreview(null);
    setError(null);
    setPartial(false);
  }, [sessionId]);

  useEffect(() => {
    if (rootQuery.data) {
      setEntriesByPath({ "": rootQuery.data.items });
      setPartial(rootQuery.data.truncated);
    }
    if (rootQuery.error) setError(workspaceErrorKey(rootQuery.error));
  }, [rootQuery.data, rootQuery.error]);

  useEffect(() => {
    if (!sessionId || !selectedPath) {
      setPreview(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    agentService.readSessionFile(sessionId, selectedPath)
      .then((content) => { if (!cancelled) setPreview(content); })
      .catch((reason: unknown) => { if (!cancelled) setError(workspaceErrorKey(reason)); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [selectedPath, sessionId]);

  const rows = useMemo(() => flattenFileRows(entriesByPath, expanded), [entriesByPath, expanded]);

  async function toggleDirectory(path: string) {
    const next = new Set(expanded);
    if (next.has(path)) next.delete(path);
    else {
      next.add(path);
      if (!entriesByPath[path]) {
        try { await loadDirectory(path); }
        catch (reason) { setError(workspaceErrorKey(reason)); }
      }
    }
    setExpanded(next);
  }

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  if ((loading || rootQuery.isLoading) && !entriesByPath[""]) return <WorkspaceState kind="loading" />;
  if (error && !entriesByPath[""]) return <WorkspaceState kind="error" message={t(error)} />;

  return (
    <div className="grid h-full min-h-0 gap-3 lg:grid-cols-[minmax(180px,0.38fr)_minmax(0,1fr)]">
      <section className="min-h-0 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
        {partial ? <PartialNotice /> : null}
        {rows.length === 0 ? <WorkspaceState kind="empty" message={t("sessionTabs.files.empty")} /> : rows.map(({ entry, depth }) => (
          <button
            className="flex h-8 w-full items-center gap-2 rounded px-2 text-left text-sm hover:bg-muted"
            key={entry.path}
            onClick={() => entry.kind === "directory" ? void toggleDirectory(entry.path) : setSelectedPath(entry.path)}
            type="button"
          >
            <span aria-hidden="true" className="shrink-0 text-muted-foreground">{"·".repeat(depth)}</span>
            {entry.kind === "directory" ? (expanded.has(entry.path) ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />) : <span className="w-3.5" />}
            {entry.kind === "directory" ? <Folder className="h-4 w-4 text-primary" /> : <File className="h-4 w-4 text-muted-foreground" />}
            <span className="truncate">{entry.name}</span>
          </button>
        ))}
      </section>
      <section className="min-h-0 overflow-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
        {loading && selectedPath ? <WorkspaceState kind="loading" /> : error ? <WorkspaceState kind="error" message={t(error)} /> : !preview ? <WorkspaceState kind="empty" message={t("sessionTabs.files.select")} /> : preview.status !== "text" ? <WorkspaceState kind="unavailable" message={t(`sessionTabs.files.${preview.status}`)} /> : (
          <><h3 className="mb-3 truncate text-sm font-semibold">{preview.path}</h3><pre className="whitespace-pre-wrap break-words font-mono text-xs leading-5">{preview.content}</pre></>
        )}
      </section>
    </div>
  );
}
