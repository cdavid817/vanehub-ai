import { useTranslation } from "react-i18next";
import { Link2, Puzzle, ToggleRight } from "lucide-react";
import type { SkillStats } from "../../../types/skill";
import { StatCard } from "../page-parts";

export function SkillStatsCards({ stats }: { stats: SkillStats }) {
  const { t } = useTranslation();

  return (
    <div className="grid gap-4 md:grid-cols-3">
      <StatCard icon={Puzzle} label={t("skills.stats.total")} value={String(stats.total)} hint={t("skills.stats.totalHint")} />
      <StatCard icon={ToggleRight} label={t("skills.stats.enabled")} value={String(stats.enabled)} hint={t("skills.stats.enabledHint")} />
      <StatCard icon={Link2} label={t("skills.stats.mounted")} value={String(stats.mounted)} hint={t("skills.stats.mountedHint")} />
    </div>
  );
}
