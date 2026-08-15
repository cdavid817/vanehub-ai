import { useTranslation } from "react-i18next";
import type { SlashCommand } from "../../services/slash-commands/types";

export function SlashCommandCompletion({
  onSelect,
  options,
}: {
  onSelect: (name: string) => void;
  options: SlashCommand[];
}) {
  const { t } = useTranslation();
  if (options.length === 0) return null;

  return (
    <div aria-label={t("slash.completion.title")} className="grid gap-0.5 text-sm" role="group">
      <p className="px-2 py-1 text-[11px] font-semibold uppercase text-muted-foreground">{t("slash.completion.title")}</p>
      {options.map((option) => (
        <button
          className="ucd-interactive flex items-center gap-2 rounded px-2 py-1.5 text-left"
          key={option.name}
          onClick={() => onSelect(option.name)}
          type="button"
        >
          <span className="shrink-0 font-medium">
            {option.argumentHint ? `/${option.name} ${option.argumentHint}` : `/${option.name}`}
          </span>
          <span className="min-w-0 flex-1 truncate text-muted-foreground">
            {t(`slash.command.${option.name}.description`)}
          </span>
        </button>
      ))}
    </div>
  );
}
