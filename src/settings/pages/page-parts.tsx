import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, type LucideIcon } from "lucide-react";
import { Badge } from "../../components/ui/badge";
import { cn } from "../../lib/utils";

export function PageHeader({
  title,
  description,
  actions,
  icon: Icon,
}: {
  title: string;
  description: string;
  actions?: ReactNode;
  icon?: LucideIcon;
}) {
  const { t } = useTranslation();

  return (
    <div className="mb-6 grid gap-4 border-b border-border pb-6 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-start">
      <div className="flex min-w-0 items-start gap-4">
        {Icon ? (
          <span className="mt-0.5 flex h-11 w-11 shrink-0 items-center justify-center rounded-md border border-primary/30 bg-[hsl(var(--nav-active-soft))] text-primary">
            <Icon className="h-4 w-4" aria-hidden="true" />
          </span>
        ) : null}
        <div className="min-w-0">
          <div className="mb-1 text-xs font-medium text-muted-foreground">{t("app.settings.breadcrumb")}</div>
          <h2 className="wrap-break-word text-2xl font-semibold leading-tight tracking-tight">{title}</h2>
          <p className="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">{description}</p>
        </div>
      </div>
      {actions ? <div className="flex flex-wrap gap-3 lg:justify-end">{actions}</div> : null}
    </div>
  );
}

export function SectionPanel({
  title,
  description,
  children,
  className,
  icon: Icon,
  variant = "card",
}: {
  title: string;
  description?: string;
  children: ReactNode;
  className?: string;
  icon?: LucideIcon;
  variant?: "card" | "settings";
}) {
  if (variant === "settings") {
    return (
      <section className={cn("overflow-hidden rounded-xl border border-border bg-background", className)}>
        <div className="flex items-start gap-3 border-b border-border bg-muted/20 px-5 py-4 sm:px-6">
          {Icon ? (
            <span className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-[hsl(var(--nav-active-soft))] text-primary">
              <Icon className="h-4 w-4" aria-hidden="true" />
            </span>
          ) : null}
          <div className="min-w-0">
            <h3 className="wrap-break-word text-sm font-semibold leading-5">{title}</h3>
            {description ? <p className="mt-0.5 max-w-3xl text-xs leading-5 text-muted-foreground">{description}</p> : null}
          </div>
        </div>
        {children}
      </section>
    );
  }

  return (
    <section className={cn("rounded-lg border border-border bg-background p-5 shadow-xs sm:p-6", className)}>
      <div className="mb-5 flex gap-4 border-b border-border/70 pb-4">
        {Icon ? (
          <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border bg-[hsl(var(--panel-muted))] text-primary">
            <Icon className="h-4 w-4" aria-hidden="true" />
          </span>
        ) : null}
        <div className="min-w-0">
          <h3 className="wrap-break-word text-base font-semibold leading-6 tracking-tight">{title}</h3>
          {description ? <p className="mt-1 max-w-3xl text-sm leading-6 text-muted-foreground">{description}</p> : null}
        </div>
      </div>
      {children}
    </section>
  );
}

export function SettingsRow({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <div className="grid min-h-18 gap-3 border-b border-border/70 px-5 py-4 last:border-b-0 sm:grid-cols-[minmax(0,1fr)_minmax(180px,auto)] sm:items-center sm:px-6">
      <div className="min-w-0">
        <div className="text-sm font-medium leading-5 text-foreground">{title}</div>
        {description ? <div className="mt-0.5 text-xs leading-5 text-muted-foreground">{description}</div> : null}
      </div>
      <div className="min-w-0 sm:justify-self-end">{children}</div>
    </div>
  );
}

export function SettingsDisclosure({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <details className="group overflow-hidden rounded-xl border border-border bg-background">
      <summary className="flex cursor-pointer list-none items-center justify-between gap-4 px-5 py-4 hover:bg-muted/30 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring sm:px-6">
        <span className="min-w-0">
          <span className="block text-sm font-semibold leading-5 text-foreground">{title}</span>
          <span className="mt-0.5 block text-xs leading-5 text-muted-foreground">{description}</span>
        </span>
        <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground transition-transform group-open:rotate-180" aria-hidden="true" />
      </summary>
      <div className="grid gap-5 border-t border-border bg-muted/10 p-4 sm:p-5">{children}</div>
    </details>
  );
}

export function StatCard({ label, value, hint, icon: Icon }: { label: string; value: string; hint: string; icon?: LucideIcon }) {
  return (
    <div className="ucd-panel ucd-interactive rounded-lg p-5">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-2xl font-semibold tracking-tight text-primary">{value}</div>
          <div className="mt-1 text-sm font-medium">{label}</div>
        </div>
        {Icon ? (
          <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-border bg-[hsl(var(--panel-muted))] text-primary">
            <Icon className="h-4 w-4" aria-hidden="true" />
          </span>
        ) : null}
      </div>
      <div className="mt-3 text-xs leading-5 text-muted-foreground">{hint}</div>
    </div>
  );
}

export function StatusPill({ status }: { status: string }) {
  const tone = status.includes("Disabled") || status.includes("Error") ? "danger" : status.includes("Update") ? "warning" : "success";
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-sm border px-2 py-0.5 text-xs font-medium",
        tone === "danger" && "ucd-status-danger",
        tone === "warning" && "ucd-status-warning",
        tone === "success" && "ucd-status-success",
      )}
    >
      {status}
    </span>
  );
}

export function TagList({ tags }: { tags: string[] }) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {tags.map((tag) => (
        <Badge key={tag} tone="muted">
          {tag}
        </Badge>
      ))}
    </div>
  );
}
