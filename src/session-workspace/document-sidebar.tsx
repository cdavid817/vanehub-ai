import { FileText, History } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { SessionDocument } from "../types/session-workspace";
import type { OutlineEntry } from "./document-outline";

/**
 * Finding a document, and finding a place in one.
 *
 * The three lists are stacked rather than tabbed. A reader looking for a heading has already chosen
 * the document, and a tabbed sidebar would make them leave the outline to see which document they
 * are in — which is the one thing the outline's context depends on.
 */
export function DocumentSidebar({
  documents,
  onSelect,
  onSelectHeading,
  outline,
  query,
  recent,
  selectedPath,
  setQuery,
}: {
  documents: readonly SessionDocument[];
  onSelect: (document: SessionDocument) => void;
  onSelectHeading: (entry: OutlineEntry) => void;
  outline: readonly OutlineEntry[];
  query: string;
  /** Documents opened this session, most recent first. */
  recent: readonly SessionDocument[];
  selectedPath: string | null;
  setQuery: (query: string) => void;
}) {
  const { t } = useTranslation();

  return (
    <section className="flex min-h-0 flex-col gap-2 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
      <input
        aria-label={t("sessionTabs.documents.filter")}
        className="h-7 w-full rounded border border-border bg-transparent px-2 text-xs outline-none"
        onChange={(event) => setQuery(event.target.value)}
        placeholder={t("sessionTabs.documents.filter")}
        type="text"
        value={query}
      />

      {/* Only while there is something to show. A permanently empty "Recent" heading would be a
          section a reader learns to skip, and it is empty for most of a session's life. */}
      {recent.length > 0 && !query.trim() ? (
        <>
          <Heading icon={<History className="h-3 w-3" />} label={t("sessionTabs.documents.recent")} />
          {recent.map((document) => (
            <DocumentRow
              document={document}
              key={`recent:${document.path}`}
              onSelect={onSelect}
              selected={document.path === selectedPath}
            />
          ))}
          <Heading label={t("sessionTabs.documents.all")} />
        </>
      ) : null}

      {documents.map((document) => (
        <DocumentRow
          document={document}
          key={document.path}
          onSelect={onSelect}
          selected={document.path === selectedPath}
        />
      ))}
      {documents.length === 0 ? (
        <p className="px-2 py-1 text-xs text-muted-foreground">
          {t("sessionTabs.documents.noMatches")}
        </p>
      ) : null}

      {outline.length > 0 ? (
        <>
          <Heading label={t("sessionTabs.documents.outline")} />
          {outline.map((entry) => (
            <button
              className="w-full truncate rounded px-2 py-1 text-left text-xs text-muted-foreground hover:bg-muted"
              key={`${entry.anchor}`}
              onClick={() => onSelectHeading(entry)}
              // Indented by depth so the shape of the document is visible in the list. A flat
              // outline of twenty headings says nothing about which contains which.
              style={{ paddingLeft: `${0.5 + (entry.depth - 1) * 0.75}rem` }}
              type="button"
            >
              {entry.text}
            </button>
          ))}
        </>
      ) : null}
    </section>
  );
}

function Heading({ icon, label }: { icon?: React.ReactNode; label: string }) {
  return (
    <p className="flex items-center gap-1 px-2 pt-1 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">
      {icon}
      {label}
    </p>
  );
}

function DocumentRow({
  document,
  onSelect,
  selected,
}: {
  document: SessionDocument;
  onSelect: (document: SessionDocument) => void;
  selected: boolean;
}) {
  return (
    <button
      className={cn(
        "flex h-8 w-full items-center gap-2 rounded px-2 text-left text-sm hover:bg-muted",
        selected && "bg-muted text-primary",
      )}
      onClick={() => onSelect(document)}
      type="button"
    >
      <FileText className="h-4 w-4 shrink-0 text-primary" />
      <span className="truncate">{document.path}</span>
    </button>
  );
}
