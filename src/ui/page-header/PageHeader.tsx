import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ActionMenu, type ActionMenuItem } from "../actions/ActionMenu";
import { cn } from "../../lib/utils";

export interface PageHeaderProps {
  title: string;
  breadcrumb?: ReactNode;
  /** The nav-entry icon, repeated here so a page keeps its visual identity once it arrives at
   *  this shared header — every settings page's own local header already does this (task 12.18). */
  icon?: LucideIcon;
  /** Clamped so a long description cannot grow the header past a predictable height. */
  description?: string;
  statusSummary?: ReactNode;
  /** A single slot — the destination model calls for exactly one primary action per page. */
  primaryAction?: ReactNode;
  moreMenuItems?: ActionMenuItem[];
  className?: string;
}

export function PageHeader({ title, breadcrumb, icon: Icon, description, statusSummary, primaryAction, moreMenuItems, className }: PageHeaderProps) {
  const { t } = useTranslation();
  return (
    <div className={cn("grid gap-3 border-b border-border-subtle pb-5", className)}>
      {breadcrumb ? <div className="text-xs text-muted-foreground">{breadcrumb}</div> : null}
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          {Icon ? (
            <span className="mt-0.5 flex h-11 w-11 shrink-0 items-center justify-center rounded-md border border-primary/30 bg-[hsl(var(--nav-active-soft))] text-primary">
              <Icon className="h-4 w-4" aria-hidden="true" />
            </span>
          ) : null}
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="wrap-break-word text-xl font-semibold leading-tight tracking-tight">{title}</h1>
              {statusSummary}
            </div>
            {description ? <p className="mt-1.5 line-clamp-2 max-w-2xl text-sm leading-6 text-muted-foreground">{description}</p> : null}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {primaryAction}
          {moreMenuItems?.length ? <ActionMenu items={moreMenuItems} triggerLabel={t("workbenchUi.pageHeader.moreActions")} /> : null}
        </div>
      </div>
    </div>
  );
}
