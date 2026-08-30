import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "../../lib/utils";

export interface FormSectionProps {
  title: string;
  description?: string;
  children: ReactNode;
  icon?: LucideIcon;
  /** e.g. a "reset this section" action, rendered next to the heading. */
  actions?: ReactNode;
  className?: string;
}

export function FormSection({ title, description, children, icon: Icon, actions, className }: FormSectionProps) {
  return (
    <section className={cn("border-b border-border-subtle pb-5 pt-5 first:pt-0 last:border-b-0 last:pb-0", className)}>
      <div className="mb-4 flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          {Icon ? (
            <span className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-nav-active-soft text-primary">
              <Icon aria-hidden="true" className="h-4 w-4" />
            </span>
          ) : null}
          <div className="min-w-0">
            <h3 className="text-sm font-semibold leading-5 text-foreground">{title}</h3>
            {description ? <p className="mt-0.5 text-xs leading-5 text-muted-foreground">{description}</p> : null}
          </div>
        </div>
        {actions ? <div className="shrink-0">{actions}</div> : null}
      </div>
      {children}
    </section>
  );
}
