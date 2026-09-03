import { useTranslation } from "react-i18next";
import { normalizeDisplayPath } from "../lib/session-path";
import { StatusBadge } from "../ui/status/StatusBadge";
import type { ExecutionTargetKind, ExecutionTargetOption } from "./execution-target-providers";

export interface ExecutionTargetOptionSummaryProps {
  kind: ExecutionTargetKind;
  option: ExecutionTargetOption;
}

/**
 * 15.6: target type, safe title, project, status, and stable identity, shared between the result
 * row a reader picks from and the confirm panel shown once more before the link is actually
 * created (execution-target-picker.tsx) -- one rendering of these fields, not two that could drift.
 */
export function ExecutionTargetOptionSummary({ kind, option }: ExecutionTargetOptionSummaryProps) {
  const { t } = useTranslation();
  return (
    <div className="grid min-w-0 gap-1 text-left">
      <div className="flex flex-wrap items-center gap-1.5">
        <span className="min-w-0 truncate text-sm font-medium">{option.title}</span>
        <span className="shrink-0 rounded border border-border px-1 py-0.5 text-[0.6875rem] text-muted-foreground">
          {t(`goals.target.${kind}`)}
        </span>
      </div>
      <div className="flex flex-wrap items-center gap-1.5">
        <StatusBadge label={t(option.statusKey)} tone={option.statusTone} />
        {option.projectPath ? (
          <span className="truncate text-xs text-muted-foreground" title={normalizeDisplayPath(option.projectPath)}>
            {normalizeDisplayPath(option.projectPath)}
          </span>
        ) : null}
      </div>
      <span className="truncate text-[0.6875rem] text-muted-foreground/70">{option.id}</span>
    </div>
  );
}
