import { forwardRef, type ReactNode } from "react";
import { CircleDot, ListRestart, Pencil, Plus, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { LoopDefinition, LoopRun } from "../types/loop";

interface LoopNavigationProps {
  className?: string;
  id?: string;
  definitions: LoopDefinition[];
  loading: boolean;
  onDefinitionChange: (id: string) => void;
  onCreateDefinition: () => void;
  onEditDefinition: () => void;
  onRunChange: (id: string) => void;
  runs: LoopRun[];
  selectedDefinitionId: string | null;
  selectedRunId: string | null;
  onClose?: () => void;
}

export const LoopNavigation = forwardRef<HTMLElement, LoopNavigationProps>(function LoopNavigation(props, ref) {
  const { i18n, t } = useTranslation();
  return (
    <aside aria-label={t("loops.navigation.open")} className={cn("flex min-h-0 min-w-0 flex-col overflow-hidden bg-[hsl(var(--panel-glass))]", props.className)} id={props.id} ref={ref} tabIndex={-1}>
      <header className="flex h-12 shrink-0 items-center gap-2 border-b border-border/70 px-3">
        <ListRestart aria-hidden="true" className="h-4 w-4 text-primary" />
        <h1 className="truncate text-sm font-semibold">{t("loops.title")}</h1>
        <div className="ml-auto flex items-center gap-1">
          <NavigationAction label={t("loops.definitions.create")} onClick={props.onCreateDefinition}><Plus aria-hidden="true" /></NavigationAction>
          <NavigationAction disabled={!props.selectedDefinitionId} label={t("loops.definitions.edit")} onClick={props.onEditDefinition}><Pencil aria-hidden="true" /></NavigationAction>
        </div>
        {props.onClose ? (
          <button aria-label={t("loops.navigation.close")} className="grid h-8 w-8 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring min-[1024px]:hidden" onClick={props.onClose} title={t("loops.navigation.close")} type="button">
            <X aria-hidden="true" className="h-4 w-4" />
          </button>
        ) : null}
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        <SectionLabel value={t("loops.definitions.title")} />
        <div className="grid gap-1">
          {props.definitions.map((definition) => (
            <button
              className={cn("ucd-list-row min-w-0 rounded-md px-2.5 py-2 text-left", props.selectedDefinitionId === definition.id && "border-primary bg-[hsl(var(--nav-active-soft))]")}
              key={definition.id}
              onClick={() => props.onDefinitionChange(definition.id)}
              type="button"
            >
              <span className="block truncate text-sm font-medium">{definition.name}</span>
              <span className="block truncate text-xs text-muted-foreground">{definition.baseBranch}</span>
            </button>
          ))}
        </div>
        <SectionLabel value={t("loops.runs.title")} />
        <div className="grid gap-1">
          {props.runs.map((run) => (
            <button
              className={cn("ucd-list-row flex min-w-0 items-center gap-2 rounded-md px-2.5 py-2 text-left", props.selectedRunId === run.id && "border-primary bg-[hsl(var(--nav-active-soft))]")}
              key={run.id}
              onClick={() => props.onRunChange(run.id)}
              type="button"
            >
              <CircleDot aria-hidden="true" className={cn("h-3.5 w-3.5 shrink-0", statusTone(run.status))} />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-xs font-medium">{t(`loops.status.${run.status}`)}</span>
                <span className="block truncate text-[11px] text-muted-foreground">{new Date(run.createdAt).toLocaleString(i18n.resolvedLanguage)}</span>
              </span>
            </button>
          ))}
          {props.loading ? <p className="px-2 py-3 text-xs text-muted-foreground">{t("loops.states.loading")}</p> : null}
        </div>
      </div>
    </aside>
  );
});

function NavigationAction({ children, disabled, label, onClick }: { children: ReactNode; disabled?: boolean; label: string; onClick: () => void }) {
  return <button aria-label={label} className="grid h-8 w-8 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-40" disabled={disabled} onClick={onClick} title={label} type="button">{children}</button>;
}

function SectionLabel({ value }: { value: string }) {
  return <h2 className="mb-1 mt-3 px-2 text-[11px] font-semibold uppercase text-muted-foreground first:mt-1">{value}</h2>;
}

function statusTone(status: LoopRun["status"]) {
  if (status === "succeeded") return "text-success";
  if (status === "failed" || status === "cancelled") return "text-destructive";
  if (status === "paused" || status === "awaiting-acceptance") return "text-warning";
  return "text-primary";
}
