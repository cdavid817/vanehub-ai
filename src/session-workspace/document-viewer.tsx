import { useEffect, useRef } from "react";
import ReactMarkdown from "react-markdown";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
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
 * Preview mode keeps the existing safe Markdown path — `skipHtml`, the same renderers for links,
 * images, code, math, and Mermaid. The only addition is an id on each heading, so the outline has
 * somewhere to scroll to.
 */
export function DocumentViewer({
  content,
  document,
  mode,
  onModeChange,
  outline,
  scrollToAnchor,
  scrollToLine,
  status,
}: {
  content: FileContent;
  document: SessionDocument | null;
  mode: DocumentMode;
  onModeChange: (mode: DocumentMode) => void;
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
      {isMarkdown ? (
        <div className="flex items-center gap-1 border-b border-border p-2">
          {(["preview", "source"] as const).map((value) => (
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
        </div>
      ) : null}

      <div className="min-h-0 flex-1 overflow-y-auto" ref={container}>
        {content.status !== "text" ? (
          <WorkspaceState kind="unavailable" message={t(`sessionTabs.files.${content.status}`)} />
        ) : mode === "source" || !isMarkdown ? (
          <FilePreview file={content} status={status} targetLine={scrollToLine} />
        ) : (
          <div className="grid max-w-none gap-3 p-4 text-sm leading-6 text-foreground [&_a]:text-primary [&_a]:underline [&_code]:rounded [&_code]:bg-muted [&_code]:px-1 [&_h1]:text-2xl [&_h1]:font-semibold [&_h2]:text-xl [&_h2]:font-semibold [&_li]:ml-5 [&_li]:list-disc [&_p]:whitespace-pre-wrap">
            <ReactMarkdown components={headingComponents(outline)} skipHtml>
              {content.content ?? ""}
            </ReactMarkdown>
          </div>
        )}
      </div>
    </article>
  );
}

/**
 * Heading renderers that carry the outline's anchors.
 *
 * The id comes from the outline by position rather than being re-derived from the rendered text.
 * The renderer sees children after Markdown parsing — emphasis and links are already elements — so
 * reconstructing the source string here would be a second parser, and the two would disagree
 * exactly on the headings that contain markup.
 */
function headingComponents(outline: readonly OutlineEntry[]) {
  let consumed = 0;
  const heading = (Tag: "h1" | "h2" | "h3" | "h4" | "h5" | "h6") =>
    function Heading({ children }: { children?: React.ReactNode }) {
      const anchor = outline[consumed]?.anchor;
      consumed += 1;
      return <Tag id={anchor}>{children}</Tag>;
    };
  return {
    h1: heading("h1"),
    h2: heading("h2"),
    h3: heading("h3"),
    h4: heading("h4"),
    h5: heading("h5"),
    h6: heading("h6"),
  };
}
