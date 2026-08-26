import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { PreviewLineRow } from "../components/chat/PreviewLineRow";
import { highlightFileLines } from "../services/code-highlighting";
import type { FileContent } from "../types/session-workspace";
import { WorkspaceState } from "./workspace-state";
import { PreviewToolbar } from "./file-preview-toolbar";
import type { PreviewStatus } from "./use-file-preview";

/**
 * A read-only file, with the things that make a long one usable.
 *
 * Highlighting, the line rows, and the "does not wrap, number stays pinned" behaviour are the same
 * ones the chat preview already uses. Reusing them rather than writing a second renderer is the
 * point: two of them would drift, and the one that drifts is always the one with fewer readers.
 *
 * Nothing here can edit. That is 12.13's rule and it shows up as an absence — no save, no input
 * bound to content, no mutation in the service calls this makes.
 */
export function FilePreview({
  file,
  observations = 0,
  onShowEvidence,
  status,
  targetLine,
}: {
  file: FileContent;
  /** How many recorded changes this file has. Zero withholds the evidence action entirely. */
  observations?: number;
  /** Absent where nothing owns the evidence scope, in which case the action is not offered. */
  onShowEvidence?: (path: string) => void;
  /** Why this may not be the file that was last asked for. */
  status: PreviewStatus;
  /** A line to reveal on open, from a content-search result. */
  targetLine: number | null;
}) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [activeMatch, setActiveMatch] = useState(0);
  const [selectedLine, setSelectedLine] = useState<number | null>(null);
  const scrollTarget = useRef<HTMLDivElement>(null);

  const content = file.content ?? "";
  // Memoized on the content, not on every render: highlighting a thousand-line file is real work,
  // and a find that re-ran it on each keystroke would make typing feel like the file was reloading.
  const lines = useMemo(() => highlightFileLines(file.path, content), [content, file.path]);

  /** Which lines contain the find query, 1-based, in order. */
  const matches = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return [];
    return content
      .split("\n")
      .map((line, index) => (line.toLowerCase().includes(needle) ? index + 1 : 0))
      .filter((line) => line > 0);
  }, [content, query]);

  // Back to the first match whenever the query changes. Keeping the index would leave "3 of 2" on
  // screen, or step to a match that is no longer one.
  useEffect(() => {
    setActiveMatch(0);
  }, [query]);

  const revealed = matches.length > 0 ? (matches[activeMatch] ?? null) : (selectedLine ?? targetLine);

  useEffect(() => {
    // Called only when it exists. Scrolling is a convenience — the line is already marked — and a
    // preview that threw because a host lacked one helper would take away the whole file to save
    // the scroll.
    scrollTarget.current?.scrollIntoView?.({ block: "center" });
  }, [revealed]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PreviewNotice status={status} />
      <PreviewToolbar
        activeMatch={activeMatch}
        file={file}
        lineCount={lines.length}
        matchCount={matches.length}
        observations={observations}
        onGoToLine={(line) => {
          setQuery("");
          setSelectedLine(Math.min(Math.max(line, 1), lines.length));
        }}
        onQueryChange={setQuery}
        onShowEvidence={onShowEvidence}
        onStepMatch={(step) => {
          if (matches.length === 0) return;
          // Wrapped, unlike the result lists: a find walks one file repeatedly, and stopping at the
          // last match would make a reader scroll back to the top by hand every time.
          setActiveMatch((current) => (current + step + matches.length) % matches.length);
        }}
        query={query}
      />
      <div className="min-h-0 flex-1 overflow-auto font-mono text-xs">
        {lines.map((line) => (
          <div
            key={line.number}
            ref={line.number === revealed ? scrollTarget : undefined}
          >
            <PreviewLineRow
              html={line.html}
              number={line.number}
              onSelect={() => {
                setSelectedLine(line.number);
              }}
              selected={line.number === revealed}
            />
          </div>
        ))}
        {lines.length === 0 ? (
          <WorkspaceState kind="empty" message={t("sessionTabs.files.preview.emptyFile")} />
        ) : null}
      </div>
    </div>
  );
}

/**
 * Why the content below may not answer the question that was just asked.
 *
 * Rendered above the path rather than replacing it, so the header keeps naming the file that is
 * actually on screen. A banner that swapped the label would be the same thing as showing the wrong
 * file: the content would be one file's and the name another's, and nothing would say which.
 */
function PreviewNotice({ status }: { status: PreviewStatus }) {
  const { t } = useTranslation();
  if (status.kind === "current") return null;
  return (
    <p
      className="mb-2 rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground"
      role="status"
    >
      {status.kind === "refreshing"
        ? t("sessionTabs.files.preview.refreshing")
        : status.kind === "loading"
          ? t("sessionTabs.files.preview.showingPrevious", { path: status.pendingPath })
          : t("sessionTabs.files.preview.loadFailed", {
              path: status.pendingPath,
              reason: t(status.reason),
            })}
    </p>
  );
}
