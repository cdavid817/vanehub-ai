import { AlertTriangle, CalendarDays } from "lucide-react";
import { useTranslation } from "react-i18next";
import { SectionPanel } from "../page-parts";
import { formatGeneratedAt } from "./usage-format";

interface UsageAccountingNoteProps {
  generatedAt?: string;
  language: string;
}

export function UsageAccountingNote({ generatedAt, language }: UsageAccountingNoteProps) {
  const { t } = useTranslation();
  const formatted = formatGeneratedAt(generatedAt, language);
  return (
    <SectionPanel description={t("usage.accounting.description")} title={t("usage.accounting.title")}>
      <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
        <div className="flex gap-3 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
          <div className="space-y-2 text-sm leading-6 text-muted-foreground">
            <p>{t("usage.accounting.reported")}</p>
            <p>{t("usage.accounting.estimated")}</p>
            <p>{t("usage.accounting.limitations")}</p>
          </div>
        </div>
        <div className="flex items-center gap-2 rounded-md border border-border bg-[hsl(var(--panel-muted))] px-4 py-3 text-sm">
          <CalendarDays className="h-4 w-4 text-primary" aria-hidden="true" />
          <span>{formatted ? t("usage.generatedAt", { value: formatted }) : t("usage.loading")}</span>
        </div>
      </div>
    </SectionPanel>
  );
}
