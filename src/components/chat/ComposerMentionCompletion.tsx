import { useEffect, useRef } from "react";
import { FileText } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import type { FileSearchMatch } from "../../types/session-workspace";
import { SeatMentionCompletion, type SeatMentionOption } from "./SeatMentionCompletion";

export function ComposerMentionCompletion({
  activeIndex, fileSuggestions, listboxId, onSelectFile, onSelectParticipant, optionId, participantSuggestions,
}: {
  activeIndex: number | null;
  fileSuggestions: FileSearchMatch[];
  listboxId: string;
  onSelectFile: (candidate: FileSearchMatch) => void;
  onSelectParticipant: (mention: string) => void;
  optionId: (index: number) => string;
  participantSuggestions: SeatMentionOption[];
}) {
  const { t } = useTranslation();
  const panelRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const active = panelRef.current?.querySelector<HTMLElement>('[aria-selected="true"]');
    active?.scrollIntoView?.({ block: "nearest" });
  }, [activeIndex]);

  if (participantSuggestions.length === 0 && fileSuggestions.length === 0) return null;
  const fileOffset = participantSuggestions.length;
  return (
    <div
      aria-label={`${t("chat.completion.participant")} / ${t("chat.completion.file")}`}
      className="ucd-panel grid max-h-56 w-full gap-1 overflow-y-auto rounded-md p-1 text-xs shadow-lg"
      id={listboxId}
      ref={panelRef}
      role="listbox"
    >
      <SeatMentionCompletion activeIndex={activeIndex} onSelect={onSelectParticipant} optionId={optionId} options={participantSuggestions} />
      {fileSuggestions.length ? <p className="px-2 py-1 text-[11px] font-semibold uppercase text-muted-foreground">{t("chat.completion.file")}</p> : null}
      {fileSuggestions.map((candidate, index) => {
        const unifiedIndex = fileOffset + index;
        const active = unifiedIndex === activeIndex;
        return (
          <button
            aria-selected={active}
            className={cn("flex min-w-0 items-center gap-2 rounded px-2 py-1.5 text-left hover:bg-muted", active && "bg-muted ring-1 ring-inset ring-ring")}
            data-active={active ? "true" : undefined}
            id={optionId(unifiedIndex)}
            key={candidate.path}
            onClick={() => onSelectFile(candidate)}
            role="option"
            type="button"
          >
            <FileText className="h-3.5 w-3.5 shrink-0 text-primary" aria-hidden="true" />
            <span className="min-w-0 flex-1 truncate">{candidate.path}</span>
          </button>
        );
      })}
    </div>
  );
}
