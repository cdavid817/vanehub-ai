import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import { RichMarkdown } from "../components/chat/RichMarkdown";
import type { FileContent, SessionDocument } from "../types/session-workspace";
import { FilePreview } from "./file-preview";
import type { OutlineEntry } from "./document-outline";
import type { PreviewStatus } from "./use-file-preview";
import { WorkspaceState } from "./workspace-state";

export type DocumentMode = "preview" | "source";

/**
 * A document, rendered or as it was written.
 *
 * Source mode is the file preview from 12.7 rather than a second renderer: a reader who wants the
 * source of a Markdown file wants the same line numbers, highlighting, and find they get on any
 * other file, and building a lesser version here would be a worse answer that also has to be
 * maintained.
 *
 * Preview mode is the application's one safe Markdown renderer, not a local copy of it. That one
 * already decides how a link opens, how an image source is checked, how code is highlighted, how
 * math is typeset, and what a `mermaid` fence becomes — and raw HTML is inert there because
 * `rehype-raw` is deliberately absent, so a `<script>` in a document is text.
 *
 * The version this replaced was a bare `ReactMarkdown` with a handful of Tailwind classes. It
 * rendered the same documents less well and, more to the point, it was a second place where those
 * five decisions were made. The one that gets forgotten in a second place is always the safety one.
 *
 * The only thing added is an id per heading, so the outline has somewhere to scroll to.
 */
export function DocumentViewer({
  content,
  document,
  mode,
  onModeChange,
  onOpenChanges,
  outline,
  scrollToAnchor,
  scrollToLine,
  status,
}: {
  content: FileContent;
  document: SessionDocument | null;
  mode: DocumentMode;
  onModeChange: (mode: DocumentMode) => void;
  /** Absent unless Git reports this document as changed. */
  onOpenChanges?: () => void;
  outline: readonly OutlineEntry[];
  /** The heading Preview should reveal, or null. */
  scrollToAnchor: string | null;
  /** The line Source should reveal, or null. */
  scrollToLine: number | null;
  status: PreviewStatus;
}) {
  const { t } = useTranslation();
  const container = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (mode !== "preview" || !scrollToAnchor) return;
    // Scoped to this container rather than the document, so activating an outline entry moves the
    // article and not the workspace shell around it.
    const heading = container.current?.querySelector(`#${CSS.escape(scrollToAnchor)}`);
    heading?.scrollIntoView?.({ block: "start" });
  }, [mode, scrollToAnchor]);

  const isMarkdown = document?.kind === "markdown";

  return (
    <article className="flex min-h-0 flex-col rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
      {/* Offered only where the two modes differ. A plain text file rendered as Markdown is the
          same text, and a toggle that changed nothing would be a control a reader stops trusting. */}
      {isMarkdown || onOpenChanges ? (
        <div className="flex items-center gap-1 border-b border-border p-2">
          {(isMarkdown ? (["preview", "source"] as const) : []).map((value) => (
            <button
              className={cn(
                "rounded border border-border px-2 py-1 text-xs text-muted-foreground hover:bg-muted",
                mode === value && "bg-muted text-primary",
              )}
              key={value}
              onClick={() => onModeChange(value)}
              type="button"
            >
              {t(`sessionTabs.documents.mode.${value}`)}
            </button>
          ))}
          {onOpenChanges ? (
            <button
              className="ml-auto rounded border border-border px-2 py-1 text-xs text-muted-foreground hover:bg-muted"
              onClick={onOpenChanges}
              type="button"
            >
              {t("sessionTabs.documents.openChanges")}
            </button>
          ) : null}
        </div>
      ) : null}

      <div className="min-h-0 flex-1 overflow-y-auto" ref={container}>
        {content.status !== "text" ? (
          <WorkspaceState kind="unavailable" message={t(`sessionTabs.files.${content.status}`)} />
        ) : mode === "source" || !isMarkdown ? (
          <FilePreview file={content} status={status} targetLine={scrollToLine} />
        ) : (
          <RichMarkdown className="p-4 text-sm" headingIds={outline.map((entry) => entry.anchor)}>
            {content.content ?? ""}
          </RichMarkdown>
        )}
      </div>
    </article>
  );
}
