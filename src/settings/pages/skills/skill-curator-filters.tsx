import { useTranslation } from "react-i18next";
import type { CuratorCandidateState, CuratorRisk, CuratorRoute } from "../../../types/skill-curator";

export interface CuratorFilters {
  state: "all" | CuratorCandidateState;
  route: "all" | CuratorRoute;
  risk: "all" | CuratorRisk;
  skillId: string;
  age: "all" | "day" | "week" | "month";
  readiness: "all" | "ready" | "not_ready";
  stale: "all" | "stale" | "current";
  notification: "all" | "pending" | "settled";
}

export const defaultCuratorFilters: CuratorFilters = {
  state: "all",
  route: "all",
  risk: "all",
  skillId: "",
  age: "all",
  readiness: "all",
  stale: "all",
  notification: "all",
};

export function SkillCuratorFilters({
  filters,
  onChange,
}: {
  filters: CuratorFilters;
  onChange: (filters: CuratorFilters) => void;
}) {
  const { t } = useTranslation();
  return <fieldset className="grid gap-2 rounded-xl border border-border bg-background/80 p-3 sm:grid-cols-2 xl:grid-cols-4">
    <legend className="sr-only">{t("skills.curator.filters")}</legend>
    <SelectFilter label={t("skills.curator.filter.state")} onChange={(state) => onChange({ ...filters, state: state as CuratorFilters["state"] })} options={states} value={filters.state} />
    <SelectFilter label={t("skills.curator.filter.route")} onChange={(route) => onChange({ ...filters, route: route as CuratorFilters["route"] })} options={routes} value={filters.route} />
    <SelectFilter label={t("skills.curator.filter.risk")} onChange={(risk) => onChange({ ...filters, risk: risk as CuratorFilters["risk"] })} options={risks} value={filters.risk} />
    <label className="text-xs text-muted-foreground"><span>{t("skills.curator.filter.skill")}</span><input className={inputClass} onChange={(event) => onChange({ ...filters, skillId: event.target.value })} placeholder={t("skills.curator.filter.skillPlaceholder")} value={filters.skillId} /></label>
    <SelectFilter label={t("skills.curator.filter.age")} onChange={(age) => onChange({ ...filters, age: age as CuratorFilters["age"] })} options={["day", "week", "month"]} value={filters.age} />
    <SelectFilter label={t("skills.curator.filter.readiness")} onChange={(readiness) => onChange({ ...filters, readiness: readiness as CuratorFilters["readiness"] })} options={["ready", "not_ready"]} value={filters.readiness} />
    <SelectFilter label={t("skills.curator.filter.staleness")} onChange={(stale) => onChange({ ...filters, stale: stale as CuratorFilters["stale"] })} options={["stale", "current"]} value={filters.stale} />
    <SelectFilter label={t("skills.curator.filter.notification")} onChange={(notification) => onChange({ ...filters, notification: notification as CuratorFilters["notification"] })} options={["pending", "settled"]} value={filters.notification} />
  </fieldset>;
}

const states: CuratorCandidateState[] = ["pending", "awaiting_draft", "ready_for_review", "deferred", "rejected", "applying", "applied", "apply_failed", "superseded"];
const routes: CuratorRoute[] = ["advance", "needs_human_review"];
const risks: CuratorRisk[] = ["low", "medium", "high"];
const inputClass = "mt-1 h-9 w-full rounded-md border border-border bg-background px-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring";

function SelectFilter({ label, onChange, options, value }: { label: string; onChange: (value: string) => void; options: string[]; value: string }) {
  const { t } = useTranslation();
  return <label className="text-xs text-muted-foreground"><span>{label}</span><select className={inputClass} onChange={(event) => onChange(event.target.value)} value={value}>
    <option value="all">{t("skills.curator.filter.all")}</option>
    {options.map((option) => <option key={option} value={option}>{t(`skills.curator.value.${option}`)}</option>)}
  </select></label>;
}
