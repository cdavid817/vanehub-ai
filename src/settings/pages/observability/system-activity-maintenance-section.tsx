import { Activity } from "lucide-react";
import { useTranslation } from "react-i18next";
import { SystemActivityControls } from "../../../system-activity/system-activity-controls";
import { SystemActivityHealthPanel } from "../../../system-activity/system-activity-health-panel";
import { useSystemActivity } from "../../../system-activity/use-system-activity";
import { SectionPanel } from "../page-parts";

/**
 * The System Activity export, rebuild, preferences, and projection health controls, hosted on the
 * observability page. The controls and the data hook are the System Activity surface's own; this
 * section only chooses which system session they act on.
 */
export function SystemActivityMaintenanceSection() {
  const { i18n, t } = useTranslation();
  const model = useSystemActivity();
  // Every session, hidden ones included: a session hidden from the timeline can only be shown
  // again through its preferences, and those live behind this picker.
  const sessions = model.sessions;
  const selected = sessions.find((session) => session.sessionId === model.selectedSessionId) ?? null;

  return (
    <SectionPanel description={t("observability.systemActivity.description")} icon={Activity} title={t("observability.systemActivity.title")}>
      <div className="grid gap-4" data-testid="observability-system-activity">
        {sessions.length === 0 ? (
          <p className="text-sm text-muted-foreground" data-testid="observability-system-activity-empty">{t("observability.systemActivity.empty")}</p>
        ) : (
          <label className="grid gap-1.5 text-sm">
            <span className="font-medium text-muted-foreground">{t("observability.systemActivity.session")}</span>
            <select
              aria-label={t("observability.systemActivity.session")}
              className="ucd-input h-9 rounded px-3 text-sm"
              onChange={(event) => model.selectSession(event.target.value)}
              value={model.selectedSessionId ?? ""}
            >
              {sessions.map((session) => (
                <option key={session.sessionId} value={session.sessionId}>
                  {session.scopeKind === "global" ? t("systemActivity.view.globalSession") : session.safeDisplayIdentity ?? session.canonicalScopeId}
                  {session.visible ? "" : ` ${t("observability.systemActivity.hiddenSuffix")}`}
                </option>
              ))}
            </select>
          </label>
        )}
        <div className="grid gap-4 xl:grid-cols-2">
          {model.health ? <SystemActivityHealthPanel health={model.health} language={i18n.language} /> : null}
          {selected ? <SystemActivityControls onChanged={model.refresh} session={selected} /> : null}
        </div>
      </div>
    </SectionPanel>
  );
}
