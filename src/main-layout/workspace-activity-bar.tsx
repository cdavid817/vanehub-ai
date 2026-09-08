import type { LucideIcon } from "lucide-react";
import { cn } from "../lib/utils";
import { formatActivityShortcut, shortcutKeyDescriptor } from "./workspace-shortcuts";
import type { WorkspaceDestination } from "./workspace-route";

/**
 * One activity entry, described rather than wired: the bar renders whatever it is given and knows
 * nothing about which surface an entry opens, so adding or retiring an entry is a configuration
 * change in the shell, not a new prop here.
 */
export interface ActivityItem {
  id: WorkspaceDestination | "settings";
  icon: LucideIcon;
  /** Accessible name and tooltip base; the tooltip appends the shortcut. */
  label: string;
  /** Chord in `Mod+N` form; `Mod` is Command on macOS and Control elsewhere. */
  shortcut: string;
  /** Rendered only when greater than zero. */
  badge?: number;
  onSelect: () => void;
  ariaControls?: string;
  active?: boolean;
  /** Set only by an entry that also toggles a region, such as Sessions and its sidebar. */
  expanded?: boolean;
  testId?: string;
}

interface WorkspaceActivityBarProps {
  items: ActivityItem[];
  utilityItems: ActivityItem[];
  /** Accessible name of the navigation landmark. */
  label: string;
}

const activityButtonClass =
  "ucd-interactive relative flex h-10 w-10 items-center justify-center rounded-md border border-transparent text-muted-foreground outline-hidden focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background";

function ActivityButton({ item }: { item: ActivityItem }) {
  const Icon = item.icon;
  const badge = item.badge ?? 0;
  return (
    <button
      aria-controls={item.ariaControls}
      aria-expanded={item.expanded}
      aria-keyshortcuts={shortcutKeyDescriptor(item.shortcut)}
      aria-label={item.label}
      className={cn(activityButtonClass, item.active && "border-primary bg-[hsl(var(--nav-active-soft))] text-primary")}
      data-activity-item={item.id}
      data-testid={item.testId}
      onClick={item.onSelect}
      title={`${item.label} · ${formatActivityShortcut(item.shortcut)}`}
      type="button"
    >
      <Icon aria-hidden="true" className="h-5 w-5" />
      {badge > 0 ? (
        <span
          className="absolute -right-0.5 -top-0.5 min-w-4 rounded-full bg-primary px-1 text-center text-[9px] leading-4 text-primary-foreground"
          data-testid={`activity-bar-badge-${item.id}`}
        >
          {badge > 99 ? "99+" : badge}
        </span>
      ) : null}
    </button>
  );
}

/** Every entry is icon-only, so each one carries a localized accessible name and tooltip. */
export function WorkspaceActivityBar({ items, label, utilityItems }: WorkspaceActivityBarProps) {
  return (
    <nav aria-label={label} className="ucd-activity-bar flex w-12 shrink-0 flex-col items-center px-1 py-2">
      <div className="flex flex-col items-center gap-1" data-activity-group="primary">
        {items.map((item) => <ActivityButton item={item} key={item.id} />)}
      </div>
      <div className="mt-auto flex flex-col items-center gap-1" data-activity-group="utility">
        {utilityItems.map((item) => <ActivityButton item={item} key={item.id} />)}
      </div>
    </nav>
  );
}
