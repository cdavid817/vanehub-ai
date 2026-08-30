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
      <div className="shrink-0 border-b border-border-subtle">{topBar}</div>
      <div className="flex min-h-0 flex-1">
        <div className="shrink-0 border-r border-border-subtle">{activityRail}</div>
        <div className="min-w-0 flex-1 overflow-hidden">{children}</div>
      </div>
    </div>
  );
}
