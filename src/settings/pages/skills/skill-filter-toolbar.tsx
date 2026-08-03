import { RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { SkillInventoryFilters, SkillSort, SkillSourceFilter, SkillStatusFilter } from "../../../lib/skill-management";

export function SkillFilterToolbar({
  categories,
  filters,
  onChange,
  resultCount,
}: {
  categories: string[];
  filters: SkillInventoryFilters;
  onChange: (filters: SkillInventoryFilters) => void;
  resultCount: number;
}) {
  const { t } = useTranslation();
  const selectClass = "min-h-9 rounded-md border border-border bg-background px-2 text-sm";
  const change = (patch: Partial<SkillInventoryFilters>) => onChange({ ...filters, ...patch });
  return (
    <div className="ucd-panel grid gap-2 rounded-lg p-3 sm:grid-cols-2 xl:grid-cols-[repeat(4,auto)]">
      <span className="text-sm tabular-nums text-muted-foreground sm:col-span-2 xl:col-span-4">{t("skills.resultCount", { count: resultCount })}</span>
      <select aria-label={t("skills.filters.category")} className={selectClass} onChange={(event) => change({ category: event.target.value })} value={filters.category}>
        {categories.map((item) => <option key={item} value={item}>{item === "all" ? t("skills.filters.allCategories") : item}</option>)}
      </select>
      <select aria-label={t("skills.filters.source")} className={selectClass} onChange={(event) => change({ source: event.target.value as SkillSourceFilter })} value={filters.source}>
        <option value="all">{t("skills.filters.allSources")}</option><option value="builtin">{t("skills.source.builtin")}</option><option value="user">{t("skills.source.user")}</option><option value="imported">{t("skills.source.imported")}</option>
      </select>
      <select aria-label={t("skills.filters.status")} className={selectClass} onChange={(event) => change({ status: event.target.value as SkillStatusFilter })} value={filters.status}>
        <option value="all">{t("skills.filters.allStatuses")}</option><option value="enabled">{t("skills.enabled")}</option><option value="disabled">{t("basic.disabled")}</option>
      </select>
      <select aria-label={t("skills.filters.sort")} className={selectClass} onChange={(event) => change({ sort: event.target.value as SkillSort })} value={filters.sort}>
        <option value="name">{t("skills.filters.sortName")}</option><option value="updated">{t("skills.filters.sortUpdated")}</option><option value="source">{t("skills.filters.sortSource")}</option>
      </select>
      <Button className="sm:col-span-2 xl:col-span-4 xl:justify-self-end" onClick={() => onChange({ category: "all", query: "", sort: "name", source: "all", status: "all" })} variant="ghost"><RotateCcw />{t("skills.filters.clear")}</Button>
    </div>
  );
}
