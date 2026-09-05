import { useEffect, useRef, useState, type FormEvent } from "react";
import { BookmarkPlus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { MissionControlSavedView } from "./mission-control-saved-views";

export interface MissionControlSavedViewMenuProps {
  savedViews: MissionControlSavedView[];
  onApply: (view: MissionControlSavedView) => void;
  onDelete: (id: string) => void;
  onSave: (name: string) => void;
}

/**
 * 16.6: a direct port of `WorkBoardSavedViewMenu`'s own composition (14.5) -- see that file's own
 * doc comment for why this is not built on `ActionMenu`: each saved-view row needs two independent
 * actions (Apply by clicking the row, Delete via its own trailing button) plus an inline
 * "save current as..." mini-form, none of which fit ActionMenu's one-action-per-item,
 * closes-on-select model.
 */
export function MissionControlSavedViewMenu({ savedViews, onApply, onDelete, onSave }: MissionControlSavedViewMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const containerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  function close() {
    setOpen(false);
    triggerRef.current?.focus();
  }

  useEffect(() => {
    if (!open) return;
    function handlePointerDown(event: PointerEvent) {
      if (!containerRef.current?.contains(event.target as Node)) setOpen(false);
    }
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, [open]);

  function submitSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = name.trim();
    if (!trimmed) return;
    onSave(trimmed);
    setName("");
  }

  return (
    <div className="relative inline-block" ref={containerRef}>
      <button
        aria-expanded={open}
        aria-haspopup="true"
        className="ucd-focus-ring inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-md border border-border px-2.5 py-1.5 text-sm hover:bg-accent"
        onClick={() => setOpen((value) => !value)}
        ref={triggerRef}
        type="button"
      >
        <BookmarkPlus aria-hidden="true" className="h-3.5 w-3.5" />
        {t("missionControl.savedViews.trigger")}
      </button>
      {open ? (
        <div
          className="ucd-raised absolute right-0 z-40 mt-1 min-w-56 rounded-md border border-border p-2 shadow-lg"
          onKeyDown={(event) => { if (event.key === "Escape") close(); }}
        >
          {savedViews.length === 0 ? <p className="px-1 py-1 text-xs text-muted-foreground">{t("missionControl.savedViews.empty")}</p> : null}
          <ul className="grid gap-0.5">
            {savedViews.map((view) => (
              <li className="flex items-center gap-1" key={view.id}>
                <button
                  className="ucd-focus-ring min-w-0 flex-1 truncate rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent"
                  onClick={() => { onApply(view); close(); }}
                  type="button"
                >
                  {view.name}
                </button>
                <Button
                  aria-label={t("missionControl.savedViews.delete", { name: view.name })}
                  className="min-h-11 min-w-11"
                  onClick={() => onDelete(view.id)}
                  size="icon"
                  type="button"
                  variant="ghost"
                >
                  <Trash2 aria-hidden="true" className="h-3.5 w-3.5" />
                </Button>
              </li>
            ))}
          </ul>
          <form className="mt-2 flex items-center gap-1 border-t border-border pt-2" onSubmit={submitSave}>
            <input
              aria-label={t("missionControl.savedViews.nameLabel")}
              className="ucd-input h-8 min-w-0 flex-1 rounded-md px-2 text-xs"
              onChange={(event) => setName(event.target.value)}
              placeholder={t("missionControl.savedViews.namePlaceholder")}
              value={name}
            />
            <Button disabled={!name.trim()} size="sm" type="submit">{t("missionControl.savedViews.save")}</Button>
          </form>
        </div>
      ) : null}
    </div>
  );
}
