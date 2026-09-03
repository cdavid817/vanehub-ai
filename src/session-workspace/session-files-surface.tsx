import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import { LazyFeature } from "../components/lazy-feature";
import { useTabList } from "../ui/runtime-panel/use-tab-list";

const loadFilesTab = () => import("./files-tab").then((module) => ({ default: module.FilesTab }));
const loadDocumentsTab = () => import("./documents-tab").then((module) => ({ default: module.DocumentsTab }));

export type FilesSurfaceView = "explorer" | "documents";
const filesSurfaceViews: FilesSurfaceView[] = ["explorer", "documents"];

/**
 * design.md Decision 7: Documents and Files merge into one Files primary surface with document
 * and explorer views, rather than two permanent primary tabs — both existing service paths
 * (`FilesTab`'s tree/preview/search, `DocumentsTab`'s outline/viewer) stay reachable unmodified,
 * just behind a subview switch instead of the top-level tab bar.
 *
 * Both views mount lazily on first visit and then stay mounted, matching every other surface's
 * retention: switching Explorer -> Documents and back must not lose a reader's search query or
 * scroll position, the same way switching away from Files and back never did.
 */
export function SessionFilesSurface({
  isVisible = true,
  onNavigateToShell,
  onOpenChanges,
  onShowEvidence,
  sessionId,
}: {
  isVisible?: boolean;
  onNavigateToShell?: () => void;
  onOpenChanges?: (path: string) => void;
  onShowEvidence?: (path: string) => void;
  sessionId: string | null;
}) {
  const { t } = useTranslation();
  const [view, setView] = useState<FilesSurfaceView>("explorer");
  const [everVisited, setEverVisited] = useState<Record<FilesSurfaceView, boolean>>({ explorer: true, documents: false });

  function activate(next: FilesSurfaceView) {
    setView(next);
    setEverVisited((current) => (current[next] ? current : { ...current, [next]: true }));
  }

  const viewTabs = useTabList(filesSurfaceViews.map((id) => ({ id })), view, (id) => activate(id as FilesSurfaceView));

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div
        aria-label={t("filesSurface.viewSwitcher")}
        className="ucd-segmented flex shrink-0 gap-1 rounded-md p-1"
        onKeyDown={viewTabs.handleKeyDown}
        role="tablist"
      >
        {filesSurfaceViews.map((id) => (
          <button
            aria-selected={view === id}
            className={cn(
              "h-7 rounded-md px-2.5 text-xs transition-colors focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring",
              view === id
                ? "bg-background font-semibold text-primary shadow-xs"
                : "text-muted-foreground hover:bg-muted hover:text-foreground",
            )}
            data-testid={`files-surface-view-${id}`}
            key={id}
            onClick={() => activate(id)}
            ref={viewTabs.registerTabRef(id)}
            role="tab"
            tabIndex={view === id ? 0 : -1}
            type="button"
          >
            {t(`filesSurface.view.${id}`)}
          </button>
        ))}
      </div>
      <div className="min-h-0 flex-1">
        {everVisited.explorer ? (
          <div className={cn("h-full min-h-0", view === "explorer" ? "block" : "hidden")}>
            <LazyFeature
              componentProps={{
                isVisible: isVisible && view === "explorer",
                onNavigateToShell,
                onShowEvidence,
                sessionId,
              }}
              loader={loadFilesTab}
            />
          </div>
        ) : null}
        {everVisited.documents ? (
          <div className={cn("h-full min-h-0", view === "documents" ? "block" : "hidden")}>
            <LazyFeature
              componentProps={{ isVisible: isVisible && view === "documents", onOpenChanges, sessionId }}
              loader={loadDocumentsTab}
            />
          </div>
        ) : null}
      </div>
    </div>
  );
}
