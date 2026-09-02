import { LazyFeature, type LazyFeatureLoader } from "../components/lazy-feature";

const loadProjects: LazyFeatureLoader<Record<string, never>> = () => import("../projects/projects")
  .then((module) => ({ default: module.Projects }));

/**
 * Task 13.1: real content replacing the former placeholder (design.md Decision 18). Zero-prop for
 * the same reason as Quality's own shell — `WorkbenchLocation`'s `projectId` (for a future
 * detail-panel selection, task 13.7) is not consumed here yet; no injectable initial-selection
 * prop exists on `Projects` because building one is that later task's own design decision to
 * make, not this shell's.
 */
export function ProjectsDestination() {
  return (
    <div className="h-full min-h-0 p-2">
      <LazyFeature className="h-full min-h-0" componentProps={{}} loader={loadProjects} />
    </div>
  );
}
