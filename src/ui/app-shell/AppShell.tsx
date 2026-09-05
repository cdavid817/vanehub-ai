import type { ReactNode } from "react";
import { cn } from "../../lib/utils";

export interface AppShellProps {
  topBar: ReactNode;
  activityRail: ReactNode;
  /** The route outlet — AppShell only provides the frame, never imports a destination module. */
  children: ReactNode;
  className?: string;
}

/**
 * The outermost frame from design.md Decision 2: TopBar + ActivityRail + RouteOutlet. Takes no
 * feature-service dependency and imports no destination module — every slot is caller-supplied,
 * so `main-layout.tsx`'s domain-specific orchestration cannot leak back in here.
 */
export function AppShell({ topBar, activityRail, children, className }: AppShellProps) {
  return (
    <div className={cn("flex h-full min-h-0 flex-col", className)}>
      {/* `relative z-60`: a Sheet opened by `children` is `position: fixed` and would otherwise sit
          on top of this persistent chrome, since a fixed element compares z-index against the
          page's stacking contexts regardless of where it is nested in the DOM. Sheet's `left`/
          `right`/`bottom` placements stay at z-50, below this, by design — they are modals over
          the work surface, not a claim to replace this chrome. Sheet's `full` placement is the one
          exception (z-65, above this): design.md Decision 3's "辅助区域全屏 Sheet" means a true
          full-screen takeover, which this chrome should not visually interrupt.
          `NotificationToastViewport` sits above both at z-70, so a toast is never hidden by either. */}
      <div className="relative z-60 shrink-0 border-b border-border-subtle">{topBar}</div>
      <div className="flex min-h-0 flex-1">
        <div className="relative z-60 shrink-0 border-r border-border-subtle">{activityRail}</div>
        {/* `min-h-0`: without it this flex item's default `min-height: auto` refuses to shrink
            below its content's intrinsic height, so a tall enough `children` subtree grows this
            whole slot past the viewport instead of being capped here so its own internal scroll
            regions (e.g. the message list) take over. */}
        <div className="min-h-0 min-w-0 flex-1 overflow-hidden">{children}</div>
      </div>
    </div>
  );
}
