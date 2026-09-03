import { useEffect, useRef, useState, type FormEvent } from "react";
import { Gauge } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { WorkItemStage } from "../types/work-board";
import { workItemStages } from "../types/work-board";
import type { WorkBoardWipLimits } from "./work-board-wip-limits";

export interface WorkBoardWipLimitMenuProps {
  limits: WorkBoardWipLimits;
  onSave: (limits: WorkBoardWipLimits) => void;
}

/** Parses a `<input type="number">`'s string value into "no limit" (undefined) or a positive
 *  integer -- matches `isOverWipLimit`'s own "0/blank means no limit" reading. */
function parseLimitInput(raw: string): number | undefined {
  const value = Number.parseInt(raw, 10);
  return Number.isFinite(value) && value > 0 ? value : undefined;
}

/**
 * 14.14: a client-local, non-persisted-to-any-service popover for optional per-stage WIP limits,
 * modeled on `WorkBoardSavedViewMenu`'s own trigger+popover+inline-form shape (same reasons that
 * file gives for not building on `ActionMenu`: this needs several independent numeric fields plus
 * one save action, not a list of one-action-per-row menu items).
 */
export function WorkBoardWipLimitMenu({ limits, onSave }: WorkBoardWipLimitMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<WorkBoardWipLimits>(limits);
  const containerRef = useRef<HTMLDivElement>(null);

  // Re-seeds the draft from the last-saved limits every time the popover opens, so a reader who
  // opens, changes nothing, and closes never accidentally diverges from what is actually stored.
  useEffect(() => { if (open) setDraft(limits); }, [open, limits]);

  useEffect(() => {
    if (!open) return;
    function handlePointerDown(event: PointerEvent) {
      if (!containerRef.current?.contains(event.target as Node)) setOpen(false);
    }
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, [open]);

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onSave(draft);
    setOpen(false);
  }

  function setStageLimit(stage: WorkItemStage, raw: string) {
    setDraft((current) => ({ ...current, [stage]: parseLimitInput(raw) }));
  }

  return (
    <div className="relative inline-block" ref={containerRef}>
      <button
        aria-expanded={open}
        aria-haspopup="true"
        className="ucd-focus-ring inline-flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-sm hover:bg-accent"
        onClick={() => setOpen((value) => !value)}
        type="button"
      >
        <Gauge aria-hidden="true" className="h-3.5 w-3.5" />
        {t("todoBoard.wip.menuTrigger")}
      </button>
      {open ? (
        <form
          className="ucd-raised absolute right-0 z-40 mt-1 grid min-w-64 gap-2 rounded-md border border-border p-3 shadow-lg"
          onKeyDown={(event) => { if (event.key === "Escape") setOpen(false); }}
          onSubmit={submit}
        >
          <p className="text-xs text-muted-foreground">{t("todoBoard.wip.menuDescription")}</p>
          {workItemStages.map((stage) => (
            <label className="flex items-center justify-between gap-2 text-xs" key={stage}>
              <span>{t(`todoBoard.stage.${stage}`)}</span>
              <input
                aria-label={t("todoBoard.wip.limitLabel", { stage: t(`todoBoard.stage.${stage}`) })}
                className="ucd-input h-8 w-20 rounded-md px-2 text-right text-xs"
                min={1}
                onChange={(event) => setStageLimit(stage, event.target.value)}
                placeholder={t("todoBoard.wip.noLimit")}
                type="number"
                value={draft[stage] ?? ""}
              />
            </label>
          ))}
          <Button className="mt-1 justify-self-end" size="sm" type="submit">{t("todoBoard.save")}</Button>
        </form>
      ) : null}
    </div>
  );
}
