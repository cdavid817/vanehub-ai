import { useTranslation } from "react-i18next";
import { EmptyState } from "../ui/empty-state/EmptyState";

/**
 * design.md Decision 18 frames Projects & Workspaces as a read-only aggregation of existing
 * project/worktree truth — real content work scoped to task group 13 (Milestone 4), not this
 * shell. Honest placeholder rather than a "coming soon" card standing in for content: the nav
 * entry and route exist now so the five-domain structure is stable, and this is replaced with
 * real aggregated content in that milestone rather than left indefinitely.
 */
export function ProjectsDestination() {
  const { t } = useTranslation();
  return (
    <div className="flex h-full min-h-0 items-center justify-center">
      <EmptyState description={t("layout.projectsPlaceholder.description")} title={t("layout.projectsPlaceholder.title")} variant="unsupported" />
    </div>
  );
}
