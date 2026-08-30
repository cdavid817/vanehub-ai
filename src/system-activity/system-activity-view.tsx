import { useTranslation } from "react-i18next";
import { MeasuredVirtualList } from "../components/measured-virtual-list";
import { formatAppDateTime } from "../i18n/format";
import { formatActivityUnreadBadge } from "./activity-badge";
import type { ActivityNavigator } from "./activity-navigation";
import type { ActivitySeverity } from "./activity-contracts";
import { SystemActivityControls } from "./system-activity-controls";
import { SystemActivityTimelineItem } from "./system-activity-timeline-item";
import { useSystemActivity } from "./use-system-activity";

export interface SystemActivityViewProps {
  onNavigate?: ActivityNavigator;
}

const severityOptions: readonly ActivitySeverity[] = ["info", "warning", "error", "critical"];

/**
 * The read-only System Activity surface. Rendered instead of the interactive chat component, so
 * no composer, Agent lifecycle hook, seat, or provider runtime ever mounts here; every action is
 * navigation, filtering, read-state, preferences, rebuild, or export.
 */
export function SystemActivityView({ onNavigate = () => undefined }: SystemActivityViewProps) {
  const { t, i18n } = useTranslation();
  const model = useSystemActivity();
  const selected = model.sessions.find((session) => session.sessionId === model.selectedSessionId);
  const effectiveRead = model.readState
    ? model.readState.markUnreadSequence === null
      ? model.readState.highestReadSequence
      : Math.min(model.readState.highestReadSequence, model.readState.markUnreadSequence - 1)
    : 0;
  const laggingDomains = (model.health?.domains ?? []).filter(
    (domain) => domain.gap != null || domain.failureCode != null || Number(domain.pendingCount ?? 0) > 0,
  );

  if (model.loading && model.sessions.length === 0) {
    return (
      <div className="grid h-full place-items-center text-sm text-muted-foreground" data-testid="system-activity-loading">
        {t("systemActivity.view.loading")}
      </div>
    );
  }
  if (model.error && model.sessions.length === 0) {
    return (
      <div className="grid h-full place-items-center" data-testid="system-activity-error" role="alert">
        <div className="text-center">
          <p className="text-sm text-destructive">{t("systemActivity.view.error")}</p>
          <p className="mt-1 font-mono text-xs text-muted-foreground">{model.error}</p>
          <button className="mt-2 rounded-md border border-border px-3 py-1 text-xs" onClick={model.refresh} type="button">
            {t("systemActivity.view.retry")}
          </button>
        </div>
      </div>
    );
  }
  if (model.sessions.length === 0) {
    return (
      <div className="grid h-full place-items-center text-sm text-muted-foreground" data-testid="system-activity-empty">
        {t("systemActivity.view.empty")}
      </div>
    );
  }

  return (
    <div className="grid h-full min-h-0 w-full grid-cols-1 grid-rows-[auto_minmax(0,1fr)_auto] gap-3 xl:grid-cols-[14rem_minmax(0,1fr)_16rem] xl:grid-rows-1" data-testid="system-activity-view">
      <nav aria-label={t("systemActivity.view.sessions")} className="flex max-h-28 gap-1 overflow-auto xl:block xl:max-h-none xl:space-y-1">
        {model.sessions.filter((session) => session.visible).map((session) => (
          <button
            aria-current={session.sessionId === model.selectedSessionId ? "true" : undefined}
            className={`flex min-w-48 items-center gap-2 rounded-md border px-2 py-1.5 text-left text-xs xl:min-w-0 ${
              session.sessionId === model.selectedSessionId
                ? "border-primary bg-[hsl(var(--nav-active-soft))]"
                : "border-transparent hover:bg-muted"
            }`}
            data-testid="system-activity-session"
            key={session.sessionId}
            onClick={() => model.selectSession(session.sessionId)}
            type="button"
          >
            <span className="min-w-0 flex-1 truncate">
              {session.scopeKind === "global"
                ? t("systemActivity.view.globalSession")
                : session.safeDisplayIdentity ?? session.canonicalScopeId}
            </span>
            {session.attentionKind !== "none" ? (
              <span aria-label={t("systemActivity.view.attention")} className="h-2 w-2 shrink-0 rounded-full bg-destructive" />
            ) : null}
            {session.unreadCount > 0 ? (
              <span className="shrink-0 rounded-full bg-primary px-1.5 text-[10px] text-primary-foreground" data-testid="system-activity-unread-badge">
                {formatActivityUnreadBadge(session.unreadCount)}
              </span>
            ) : null}
          </button>
        ))}
      </nav>
      <section aria-label={t("systemActivity.view.timeline")} className="flex min-h-0 min-w-0 flex-1 flex-col gap-2">
        <div className="flex shrink-0 flex-wrap items-center gap-2">
          <input
            aria-label={t("systemActivity.view.search")}
            className="w-48 rounded-md border border-border bg-background px-2 py-1 text-xs"
            onChange={(event) => model.setSearchText(event.target.value)}
            placeholder={t("systemActivity.view.search")}
            value={model.searchText}
          />
          <select
            aria-label={t("systemActivity.view.severityFilter")}
            className="rounded-md border border-border bg-background px-2 py-1 text-xs"
            onChange={(event) =>
              model.setSeverityFilter(
                event.target.value === "" ? null : (event.target.value as ActivitySeverity),
              )
            }
            value={model.severityFilter ?? ""}
          >
            <option value="">{t("systemActivity.view.allSeverities")}</option>
            {severityOptions.map((severity) => (
              <option key={severity} value={severity}>
                {t(`systemActivity.severity.${severity}.title`)}
              </option>
            ))}
          </select>
          <button
            className="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
            data-testid="system-activity-mark-read"
            onClick={model.markReadThroughNewest}
            type="button"
          >
            {t("systemActivity.view.markRead")}
          </button>
        </div>
        {model.staleGeneration ? (
          <p className="shrink-0 rounded-md border border-amber-300 bg-amber-50 px-2 py-1 text-xs text-amber-900 dark:border-amber-700 dark:bg-amber-950/40 dark:text-amber-100" data-testid="system-activity-stale-banner" role="status">
            {t("systemActivity.view.staleGeneration")}
          </p>
        ) : null}
        {laggingDomains.length > 0 ? (
          <p className="shrink-0 rounded-md border border-border bg-muted px-2 py-1 text-xs text-muted-foreground" data-testid="system-activity-lag-banner" role="status">
            {t("systemActivity.view.lag", { count: laggingDomains.length })}
          </p>
        ) : null}
        {model.entries.length === 0 ? (
          <p className="text-sm text-muted-foreground" data-testid="system-activity-no-items">
            {t("systemActivity.view.noItems")}
          </p>
        ) : (
          <MeasuredVirtualList
            ariaLabel={t("systemActivity.view.timeline")}
            className="min-h-0 flex-1"
            estimateSize={() => 132}
            getItemKey={(entry) => entry.envelope.eventId}
            itemClassName="pb-2"
            items={model.entries}
            overscan={6}
            renderItem={(entry) => (
              <SystemActivityTimelineItem
                entry={entry}
                language={i18n.language}
                onNavigate={onNavigate}
                t={t}
                unread={entry.sequence > effectiveRead}
              />
            )}
            testId="system-activity-timeline"
          />
        )}
        {model.nextCursor ? (
          <button className="shrink-0 self-center rounded-md border border-border px-3 py-1 text-xs hover:bg-muted" onClick={model.loadMore} type="button">
            {t("systemActivity.view.loadMore")}
          </button>
        ) : null}
      </section>
      <aside aria-label={t("systemActivity.view.summary")} className="max-h-52 space-y-3 overflow-y-auto xl:max-h-none">
        {model.dashboard.length > 0 ? (
          <section aria-label={t("systemActivity.view.dashboard")} className="rounded-lg border border-border p-3" data-testid="system-activity-dashboard">
            <h3 className="text-xs font-semibold">{t("systemActivity.view.dashboard")}</h3>
            {model.dashboard.map((summary) => (
              <dl className="mt-2 text-xs text-muted-foreground" key={summary.materializationKind}>
                <dt className="font-mono">{summary.materializationKind}</dt>
                <dd>{t("systemActivity.view.updatedAt", { time: formatAppDateTime(summary.updatedAtMs, i18n.language, { dateStyle: "medium", timeStyle: "short" }) })}</dd>
              </dl>
            ))}
          </section>
        ) : null}
        {model.health ? (
          <section aria-label={t("systemActivity.view.health")} className="rounded-lg border border-border p-3 text-xs text-muted-foreground" data-testid="system-activity-health">
            <h3 className="font-semibold text-foreground">{t("systemActivity.view.health")}</h3>
            <p className="mt-1">
              {model.health.lastCompletedAtMs
                ? t("systemActivity.view.lastProjected", {
                    time: formatAppDateTime(model.health.lastCompletedAtMs, i18n.language, {
                      dateStyle: "medium",
                      timeStyle: "short",
                    }),
                  })
                : t("systemActivity.view.neverProjected")}
            </p>
          </section>
        ) : null}
        {selected ? <SystemActivityControls onChanged={model.refresh} session={selected} /> : null}
      </aside>
    </div>
  );
}
