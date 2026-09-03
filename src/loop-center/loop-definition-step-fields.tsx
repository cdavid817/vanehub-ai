import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { LoopLimits } from "../types/loop";
import type { LoopDefinitionDraft } from "./loop-definition-form";

/** Shared by every step of `loop-definition-dialog.tsx`'s four-step flow (task 17.5's file split):
 *  each step owns a slice of the same draft and reports edits back up through this one setter. */
export interface StepProps {
  draft: LoopDefinitionDraft;
  setDraft: (draft: LoopDefinitionDraft) => void;
}

export const inputClass = "ucd-input h-9 w-full rounded px-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export function Field({ children, className = "", label }: { children: ReactNode; className?: string; label: string }) {
  const { t } = useTranslation();
  return <label className={`grid gap-1.5 ${className}`}><span className="text-xs font-medium text-muted-foreground">{t(label)}</span>{children}</label>;
}

export function NumberField({ draft, field, max, setDraft }: StepProps & { field: keyof LoopLimits; max?: number }) {
  return <Field label={`loops.editor.field.${field}`}><input className={inputClass} max={max} min={1} type="number" value={draft.limits[field]} onChange={(event) => setDraft({ ...draft, limits: { ...draft.limits, [field]: Number(event.target.value) } })} /></Field>;
}
