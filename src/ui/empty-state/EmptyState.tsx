import type { ReactNode } from "react";
import { Ban, Inbox, Lock, SearchX, Sparkles, Unplug, type LucideIcon } from "lucide-react";
import { cn } from "../../lib/utils";

export type EmptyStateVariant = "first-run" | "no-data" | "no-filter-match" | "unsupported" | "unavailable" | "restricted";

const DEFAULT_ICON: Record<EmptyStateVariant, LucideIcon> = {
  "first-run": Sparkles,
  "no-data": Inbox,
  "no-filter-match": SearchX,
  unsupported: Ban,
  unavailable: Unplug,
  restricted: Lock,
};

export interface EmptyStateProps {
  variant: EmptyStateVariant;
  title: string;
  description?: string;
  icon?: LucideIcon;
  action?: ReactNode;
  className?: string;
}

/**
 * Structural shell only — title/description are caller-supplied and already localized, since
 * "no sessions" and "no runs" need domain-specific copy that this primitive cannot own.
 */
export function EmptyState({ variant, title, description, icon, action, className }: EmptyStateProps) {
  const Icon = icon ?? DEFAULT_ICON[variant];
  return (
    <div
      className={cn("flex min-h-40 flex-col items-center justify-center gap-2 p-6 text-center", className)}
      data-empty-state-variant={variant}
    >
      <Icon aria-hidden="true" className="h-8 w-8 text-muted-foreground" />
      <p className="text-sm font-medium">{title}</p>
      {description ? <p className="max-w-sm text-sm text-muted-foreground">{description}</p> : null}
      {action ? <div className="mt-2">{action}</div> : null}
    </div>
  );
}
