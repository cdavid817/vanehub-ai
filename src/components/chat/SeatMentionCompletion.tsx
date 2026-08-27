import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import type { ModelFamily } from "../../services/model-family";

export interface SeatMentionOption {
  mention: string;
  roleName: string | null;
  agentName: string;
  modelFamily: ModelFamily;
  avatar: string;
}

/**
 * Offered only in a multi-seat session. It also states the line-leading rule, because a user who
 * types `@` mid-sentence and sees nothing happen has no way to discover why.
 */
export function SeatMentionCompletion({
  activeIndex,
  onSelect,
  optionId,
  options,
}: {
  activeIndex: number | null;
  onSelect: (mention: string) => void;
  optionId: (index: number) => string;
  options: SeatMentionOption[];
}) {
  const { t } = useTranslation();
  if (options.length === 0) return null;

  return (
    <div aria-label={t("chat.completion.participant")} className="grid gap-0.5 text-sm" role="group">
      <p className="px-2 py-1 text-[11px] font-semibold uppercase text-muted-foreground">{t("chat.completion.participant")}</p>
      {options.map((option, index) => (
        <button
          aria-selected={index === activeIndex}
          className={cn("ucd-interactive flex items-center gap-2 rounded px-2 py-1.5 text-left", index === activeIndex && "bg-muted ring-1 ring-inset ring-ring")}
          data-active={index === activeIndex ? "true" : undefined}
          id={optionId(index)}
          key={option.mention}
          onClick={() => onSelect(option.mention)}
          role="option"
          type="button"
        >
          <span aria-hidden="true">{option.avatar}</span>
          <span className="min-w-0 flex-1 truncate">
            <span className="font-medium">{option.roleName ?? option.agentName}</span>
            {option.roleName ? <span className="text-muted-foreground"> · {option.agentName}</span> : null}
          </span>
          <span className="shrink-0 text-[11px] text-muted-foreground">{option.modelFamily}</span>
        </button>
      ))}
      <p className="px-2 py-1 text-[11px] text-muted-foreground">{t("chat.mentionLineStartHint")}</p>
    </div>
  );
}
