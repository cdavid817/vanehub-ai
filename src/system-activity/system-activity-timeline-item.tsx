import type { TFunction } from "i18next";
import { formatAppDateTime } from "../i18n/format";
import type { SystemActivityTimelineEntry } from "../services/system-activity-service";
import type { ActivityEventCode, ActivitySeverity, ActivityStatus } from "./activity-contracts";
import type { ActivityNavigator } from "./activity-navigation";
import { openActivityNavigation } from "./activity-navigation";
import {
  activityEventPresentation,
  activitySeverityPresentation,
  activityStatusPresentation,
} from "./activity-presentation-registry";
import { SafeActivityPayload } from "./safe-activity-payload";

interface SystemActivityTimelineItemProps {
  entry: SystemActivityTimelineEntry;
  unread: boolean;
  t: TFunction;
  language: string;
  onNavigate: ActivityNavigator;
}

const severityTone: Record<ActivitySeverity, string> = {
  info: "text-muted-foreground",
  warning: "text-amber-600 dark:text-amber-400",
  error: "text-red-600 dark:text-red-400",
  critical: "text-red-700 dark:text-red-300",
};

/**
 * One bounded, read-only timeline entry rendered entirely from its envelope: localized title from
 * the registry, safe structured payload, and a navigation-only link. Nothing here can mutate
 * evolution state.
 */
export function SystemActivityTimelineItem({ entry, unread, t, language, onNavigate }: SystemActivityTimelineItemProps) {
  const { envelope } = entry;
  const presentation =
    activityEventPresentation[envelope.eventCode as ActivityEventCode] ?? null;
  const Icon = presentation?.icon ?? null;
  const title = presentation
    ? t(presentation.titleKey)
    : // Documented fallback: keep the safe code visible when the registry has no entry.
      envelope.eventCode;
  const statusKey = activityStatusPresentation[envelope.status as ActivityStatus];
  const severityKey = activitySeverityPresentation[envelope.severity as ActivitySeverity];
  return (
    <li
      className="rounded-lg border border-border bg-card p-3"
      data-sequence={entry.sequence}
      data-testid="system-activity-item"
      data-unread={unread ? "true" : "false"}
    >
      <div className="flex items-center gap-2">
        {Icon ? <Icon aria-hidden="true" className={`h-4 w-4 shrink-0 ${severityTone[envelope.severity]}`} /> : null}
        <span className="min-w-0 flex-1 truncate text-sm font-medium">{title}</span>
        {unread ? (
          <span aria-label={t("systemActivity.view.unreadItem")} className="h-2 w-2 shrink-0 rounded-full bg-primary" />
        ) : null}
        <span className={`shrink-0 text-xs ${severityTone[envelope.severity]}`}>
          {severityKey ? t(severityKey.titleKey) : envelope.severity}
        </span>
      </div>
      <p className="mt-1 truncate text-xs text-muted-foreground">
        {statusKey ? t(statusKey.titleKey) : envelope.status}
        {" · "}
        {formatAppDateTime(envelope.committedAtMs, language, { dateStyle: "medium", timeStyle: "short" })}
        {" · "}
        <span className="font-mono">{envelope.eventCode}</span>
      </p>
      {envelope.payload ? (
        <div className="mt-2">
          <SafeActivityPayload
            eventCode={envelope.eventCode}
            occurredAtMs={envelope.occurredAtMs}
            onNavigate={onNavigate}
            payload={envelope.payload}
            severity={envelope.severity}
            translate={(key, values) => t(key, values ?? {})}
          />
        </div>
      ) : null}
      {envelope.navigation ? (
        <button
          className="mt-2 text-xs text-primary underline-offset-2 hover:underline"
          onClick={() => openActivityNavigation(envelope.navigation, onNavigate)}
          type="button"
        >
          {t(`systemActivity.navigation.${envelope.navigation.kind}`, {
            id: envelope.navigation.stableId,
          })}
        </button>
      ) : null}
    </li>
  );
}
