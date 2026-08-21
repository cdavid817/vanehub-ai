import { CircleCheckBig, Workflow } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { PromptHookListResult } from "../../../types/prompt-hook";

export function PromptHookInventorySummary({
  stats,
  visible,
}: {
  stats: PromptHookListResult["stats"];
  visible: number;
}) {
  const { t } = useTranslation();
  return (
    <div className="ucd-panel flex flex-wrap items-center gap-2 rounded-lg px-3 py-2 text-sm" aria-live="polite">
      <Workflow className="h-4 w-4 text-primary" aria-hidden="true" />
      <span>{t("promptHooks.summary", { visible, ...stats })}</span>
      {visible === stats.total ? (
        <CircleCheckBig className="ml-auto h-4 w-4 text-muted-foreground" aria-hidden="true" />
      ) : null}
    </div>
  );
}
