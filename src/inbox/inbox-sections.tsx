import { AlertTriangle } from "lucide-react";
import { useEffect, useRef, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { RunCard } from "../mission-control/mission-control";
import type { ActivityNavigator } from "../system-activity/activity-navigation";
import { SystemActivityTimelineItem } from "../system-activity/system-activity-timeline-item";
import type { MissionControlAction, MissionControlRunSummary } from "../types/mission-control";
import type { InboxSections as InboxSectionData } from "./inbox-feed";
import type { InboxView } from "../main-layout/workspace-route";

interface InboxSectionsProps {
  sections: InboxSectionData;
  loading: boolean;
  /** The section the route names; it is scrolled into view when it changes. */
  focusedView: InboxView;
  onAct: (run: MissionControlRunSummary, action: MissionControlAction) => void;
  onInspect: (run: MissionControlRunSummary) => void;
  onNavigateActivity: ActivityNavigator;
}

function Section({ children, count, focused, id, title, urgent = false }: {
  children: ReactNode;
  count: number;
  focused: boolean;
  id: string;
  title: string;
  urgent?: boolean;
}) {
  const { t } = useTranslation();
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    if (focused) ref.current?.scrollIntoView?.({ block: "start" });
  }, [focused]);
  return (
    <section aria-labelledby={`${id}-title`} className="mb-4" data-inbox-section={id} data-inbox-focused={focused ? "true" : "false"} id={id} ref={ref}>
      <h2 className="mb-2 flex items-center gap-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground" id={`${id}-title`}>
        {urgent ? <AlertTriangle aria-hidden="true" className="h-3.5 w-3.5 text-warning" /> : null}
        <span>{title}</span>
        <span className="ml-1 rounded-full bg-muted px-1.5 text-[10px] tabular-nums text-muted-foreground" data-testid={`${id}-count`}>{count}</span>
      </h2>
      {count === 0 ? <p className="ucd-muted-panel rounded-md p-3 text-xs text-muted-foreground">{t("inbox.sectionEmpty")}</p> : <div className="grid gap-2">{children}</div>}
    </section>
  );
}

/**
 * Needs attention / Running / Recently finished. Run rows are Mission Control's own cards and
 * activity rows are System Activity's own timeline items, so each row keeps the navigation and
 * control actions its owning surface defines.
 */
export function InboxSections({ focusedView, loading, onAct, onInspect, onNavigateActivity, sections }: InboxSectionsProps) {
  const { i18n, t } = useTranslation();
  const attentionCount = sections.attention.runs.length + sections.attention.activity.length;
  const total = attentionCount + sections.running.length + sections.recent.length;
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-3" data-testid="inbox-sections">
      <Section count={attentionCount} focused={focusedView === "attention"} id="inbox-attention" title={t("inbox.section.attention")} urgent>
        {sections.attention.runs.map((run) => <RunCard key={run.runId} onAct={onAct} onInspect={onInspect} run={run} />)}
        {sections.attention.activity.map(({ entry, sessionId }) => (
          <SystemActivityTimelineItem entry={entry} key={`${sessionId}:${entry.envelope.eventId}`} language={i18n.language} onNavigate={onNavigateActivity} t={t} unread />
        ))}
      </Section>
      <Section count={sections.running.length} focused={focusedView === "running"} id="inbox-running" title={t("inbox.section.running")}>
        {sections.running.map((run) => <RunCard key={run.runId} onAct={onAct} onInspect={onInspect} run={run} />)}
      </Section>
      <Section count={sections.recent.length} focused={focusedView === "recent"} id="inbox-recent" title={t("inbox.section.recent")}>
        {sections.recent.map((run) => <RunCard key={run.runId} onAct={onAct} onInspect={onInspect} run={run} />)}
      </Section>
      {!loading && total === 0 ? <p className="p-8 text-center text-sm text-muted-foreground" data-testid="inbox-empty">{t("inbox.empty")}</p> : null}
    </div>
  );
}
