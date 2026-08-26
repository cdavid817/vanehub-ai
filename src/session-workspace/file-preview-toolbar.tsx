import { useState } from "react";
import { ChevronDown, ChevronUp, Link2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { FileContent } from "../types/session-workspace";

/**
 * The preview's own controls: find, go to line, what the file is, and where its records are.
 *
 * Separate from the preview so the renderer stays a renderer. The two change for different reasons
 * — one when the way a file is displayed changes, the other when the ways of moving around it do —
 * and the line rule would eventually force this split anyway.
 */
export function PreviewToolbar({
  activeMatch,
  file,
  lineCount,
  matchCount,
  observations,
  onGoToLine,
  onQueryChange,
  onShowEvidence,
  onStepMatch,
  query,
}: {
  activeMatch: number;
  file: FileContent;
  lineCount: number;
  matchCount: number;
  /** How many recorded changes this file has. Zero withholds the evidence action entirely. */
  observations: number;
  onGoToLine: (line: number) => void;
  onQueryChange: (query: string) => void;
  onShowEvidence?: (path: string) => void;
  onStepMatch: (step: number) => void;
  query: string;
}) {
  const { t } = useTranslation();
  const [lineInput, setLineInput] = useState("");

  return (
    <div className="mb-2 flex flex-wrap items-center gap-2 border-b border-border pb-2">
      <span className="min-w-0 flex-1 truncate text-sm font-semibold">{file.path}</span>

      <input
        aria-label={t("sessionTabs.files.preview.find")}
        className="h-7 w-40 rounded border border-border bg-transparent px-2 text-xs outline-none"
        onChange={(event) => onQueryChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key !== "Enter") return;
          event.preventDefault();
          // Shift steps backwards, which is what every find box does. A reader who has learned one
          // should not have to learn this one.
          onStepMatch(event.shiftKey ? -1 : 1);
        }}
        placeholder={t("sessionTabs.files.preview.find")}
        type="text"
        value={query}
      />
      {query.trim() ? (
        <>
          <span className="text-xs tabular-nums text-muted-foreground">
            {/* Absent matches read as "0" rather than as a blank, so a query that found nothing is
                distinguishable from one that has not run. */}
            {matchCount === 0 ? "0" : `${activeMatch + 1}/${matchCount}`}
          </span>
          <IconButton
            disabled={matchCount === 0}
            label={t("sessionTabs.files.preview.previousMatch")}
            onClick={() => onStepMatch(-1)}
          >
            <ChevronUp className="h-3.5 w-3.5" />
          </IconButton>
          <IconButton
            disabled={matchCount === 0}
            label={t("sessionTabs.files.preview.nextMatch")}
            onClick={() => onStepMatch(1)}
          >
            <ChevronDown className="h-3.5 w-3.5" />
          </IconButton>
        </>
      ) : null}

      <input
        aria-label={t("sessionTabs.files.preview.goToLine")}
        className="h-7 w-20 rounded border border-border bg-transparent px-2 text-xs outline-none"
        inputMode="numeric"
        onChange={(event) => setLineInput(event.target.value.replace(/\D/g, ""))}
        onKeyDown={(event) => {
          if (event.key !== "Enter") return;
          event.preventDefault();
          const line = Number.parseInt(lineInput, 10);
          // Clamped by the preview rather than refused here: a reader who types 9999 in a 200-line
          // file meant "the end", and an error message would be answering a question they did not
          // think they were asking.
          if (Number.isFinite(line)) onGoToLine(line);
        }}
        placeholder={t("sessionTabs.files.preview.goToLine")}
        type="text"
        value={lineInput}
      />

      {/* Offered only when something is retained about this file. An action that always appeared
          would send a reader to an empty list for most of the files they open, and they would stop
          believing it the third time. */}
      {onShowEvidence && observations > 0 ? (
        <IconButton
          label={t("sessionTabs.files.preview.showEvidence")}
          onClick={() => onShowEvidence(file.path)}
        >
          <Link2 className="h-3.5 w-3.5" />
          <span className="ml-1 tabular-nums">{observations}</span>
        </IconButton>
      ) : null}

      <PreviewMetadata file={file} lineCount={lineCount} />
    </div>
  );
}

/**
 * What the file is, beside what it says.
 *
 * A byte order mark is invisible and breaks shell scripts and JSON parsers; mixed line endings turn
 * an ordinary edit into a diff that claims every line changed. Both are things a reader can only
 * act on if something tells them, because neither is visible in the content itself.
 */
function PreviewMetadata({ file, lineCount }: { file: FileContent; lineCount: number }) {
  const { t } = useTranslation();
  return (
    <span className="flex w-full items-center gap-3 text-xs text-muted-foreground">
      <span>{t("sessionTabs.files.preview.lines", { count: lineCount })}</span>
      <span>{formatBytes(file.size)}</span>
      {/* Rendered only when there is one. A binary file has no encoding this application
          established, and an "unknown" chip would be a row a reader learns to ignore. */}
      {file.encoding ? <span>{t(`sessionTabs.files.preview.encoding.${file.encoding}`)}</span> : null}
      {file.newline ? <span>{t(`sessionTabs.files.preview.newline.${file.newline}`)}</span> : null}
    </span>
  );
}

function formatBytes(size: number): string {
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KiB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MiB`;
}

function IconButton({
  children,
  disabled = false,
  label,
  onClick,
}: {
  children: React.ReactNode;
  disabled?: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      aria-label={label}
      className="rounded border border-border p-1 text-muted-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
      disabled={disabled}
      onClick={onClick}
      title={label}
      type="button"
    >
      {children}
    </button>
  );
}
