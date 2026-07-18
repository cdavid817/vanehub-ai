import { Braces, Link2, ToggleRight, Workflow } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { PromptHookListResult } from "../../../types/prompt-hook";
import { StatCard } from "../page-parts";

export function PromptHookStatsCards({ stats }: { stats: PromptHookListResult["stats"] }) {
  const { t } = useTranslation();

  return (
    <div className="grid gap-3 md:grid-cols-4">
      <StatCard icon={Workflow} label={t("promptHooks.stats.total")} value={String(stats.total)} hint={t("promptHooks.stats.totalHint")} />
      <StatCard icon={ToggleRight} label={t("promptHooks.stats.enabled")} value={String(stats.enabled)} hint={t("promptHooks.stats.enabledHint")} />
      <StatCard icon={Braces} label={t("promptHooks.stats.builtin")} value={String(stats.builtin)} hint={t("promptHooks.stats.builtinHint")} />
      <StatCard icon={Link2} label={t("promptHooks.stats.user")} value={String(stats.user)} hint={t("promptHooks.stats.userHint")} />
    </div>
  );
}
