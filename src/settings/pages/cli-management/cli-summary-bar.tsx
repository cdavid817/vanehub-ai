import { AlertTriangle, CheckCircle2, KeyRound, ShieldAlert, ArrowUpCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { CliSummaryBucket, CliSummaryCounts } from "./cli-management-presenters";

/**
 * The five counts, as one compact row of filter buttons.
 *
 * Each bucket is also the state filter, so the number and the way to act on it are the same
 * control -- a count you cannot click is a number you have to act on somewhere else.
 */

const BUCKETS: Array<{
  bucket: CliSummaryBucket;
  labelKey: string;
  icon: typeof CheckCircle2;
  tone: string;
}> = [
  { bucket: "ready", labelKey: "cli.summary.ready", icon: CheckCircle2, tone: "text-[hsl(var(--success))]" },
  { bucket: "needsLogin", labelKey: "cli.summary.needsLogin", icon: KeyRound, tone: "text-[hsl(var(--warning))]" },
  { bucket: "updates", labelKey: "cli.summary.updates", icon: ArrowUpCircle, tone: "text-primary" },
  { bucket: "conflicts", labelKey: "cli.summary.conflicts", icon: AlertTriangle, tone: "text-[hsl(var(--warning))]" },
  { bucket: "broken", labelKey: "cli.summary.broken", icon: ShieldAlert, tone: "text-[hsl(var(--danger))]" },
];

export function CliSummaryBar({
  counts,
  activeState,
  onSelectState,
}: {
  counts: CliSummaryCounts;
  activeState: string;
  onSelectState: (state: string) => void;
}) {
  const { t } = useTranslation();

  return (
    <div aria-label={t("cli.summary.title")} className="flex flex-wrap gap-2" role="group">
      {BUCKETS.map(({ bucket, labelKey, icon: Icon, tone }) => {
        const active = activeState === bucket;
        return (
          <button
            aria-pressed={active}
            className={`ucd-panel flex items-center gap-2 rounded-md px-3 py-2 text-left text-xs transition-colors focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring ${
              active ? "border-primary text-primary" : "hover:bg-accent"
            }`}
            key={bucket}
            type="button"
            // Clicking an active bucket clears the filter: the same control both applies and
            // removes it, so there is never a filter with no visible way back.
            onClick={() => onSelectState(active ? "all" : bucket)}
          >
            <Icon aria-hidden="true" className={`h-4 w-4 ${tone}`} />
            <span className="font-semibold tabular-nums">{counts[bucket]}</span>
            <span className="text-muted-foreground">{t(labelKey)}</span>
          </button>
        );
      })}
    </div>
  );
}
