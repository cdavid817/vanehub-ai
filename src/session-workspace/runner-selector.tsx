import { RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentRunnerDescriptor, AgentRunnerSelection } from "../types/agent-runner";
import { sameSelection } from "./use-runner-selection";

export function RunnerSelector({
  descriptors, disabled, error, loading, onChange, onRetry, value,
}: {
  descriptors: AgentRunnerDescriptor[];
  disabled: boolean;
  error: boolean;
  loading: boolean;
  onChange: (selection: AgentRunnerSelection) => void;
  onRetry: () => void;
  value: AgentRunnerSelection;
}) {
  const { t } = useTranslation();
  const selectedIndex = descriptors.findIndex((descriptor) => sameSelection(descriptor.selection, value));
  return <div className="flex min-w-0 items-center gap-2 px-3 pt-2" data-testid="runner-selector">
    <label className="shrink-0 text-[11px] font-medium text-muted-foreground" htmlFor="agent-runner-select">
      {t("runner.selector.label")}
    </label>
    <select
      aria-describedby="agent-runner-status"
      className="h-7 min-w-0 max-w-64 rounded-md border border-input bg-background px-2 text-xs"
      disabled={disabled || loading || error}
      id="agent-runner-select"
      onChange={(event) => onChange(descriptors[Number(event.target.value)]?.selection ?? { kind: "local" })}
      value={selectedIndex < 0 ? "" : String(selectedIndex)}
    >
      {loading ? <option value="">{t("runner.selector.loading")}</option> : null}
      {!loading && error ? <option value="">{t("runner.selector.error")}</option> : null}
      {descriptors.map((descriptor, index) => <option disabled={!descriptor.available} key={`${descriptor.selection.kind}:${descriptor.selection.targetId ?? ""}:${descriptor.selection.targetRevision ?? ""}`} value={index}>
        {descriptor.label}{descriptor.hostLabel ? ` · ${descriptor.hostLabel}` : ""}{descriptor.available ? "" : ` · ${t(`runner.unavailable.${descriptor.unavailableReason ?? "unknown"}`)}`}
      </option>)}
    </select>
    <span className="max-w-52 truncate text-[11px] text-muted-foreground" id="agent-runner-status">
      {error ? t("runner.selector.errorHint") : t("runner.selector.simulationHint", { mode: descriptors.find((item) => sameSelection(item.selection, value))?.simulated ? t("runner.simulated") : t("runner.native") })}
    </span>
    {error ? <button aria-label={t("runner.selector.retry")} className="grid h-7 w-7 shrink-0 place-items-center rounded-md border border-input" onClick={onRetry} type="button"><RefreshCw aria-hidden="true" className="h-3.5 w-3.5" /></button> : null}
  </div>;
}
