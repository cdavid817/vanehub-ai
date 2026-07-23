import { ArrowDown, ArrowUp, Plus, Trash2 } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { createVerificationCommandDraft, validateVerificationCommand, type LoopVerificationCommandDraft } from "./loop-definition-form";

interface LoopVerificationCommandEditorProps {
  commands: LoopVerificationCommandDraft[];
  onChange: (commands: LoopVerificationCommandDraft[]) => void;
  showErrors: boolean;
}

export function LoopVerificationCommandEditor({ commands, onChange, showErrors }: LoopVerificationCommandEditorProps) {
  const { t } = useTranslation();
  const update = (index: number, command: LoopVerificationCommandDraft) => onChange(commands.map((current, position) => position === index ? command : current));
  const move = (index: number, offset: number) => {
    const target = index + offset;
    if (target < 0 || target >= commands.length) return;
    const next = [...commands];
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };

  return (
    <section className="grid gap-3" aria-labelledby="loop-verification-commands-title">
      <div className="flex items-center justify-between gap-3">
        <h3 className="text-xs font-semibold" id="loop-verification-commands-title">{t("loops.editor.commands.title")}</h3>
        <Button onClick={() => onChange([...commands, createVerificationCommandDraft(commands)])} size="sm" type="button" variant="outline"><Plus aria-hidden="true" />{t("loops.editor.commands.add")}</Button>
      </div>
      {commands.map((command, index) => (
        <CommandRow
          command={command}
          index={index}
          key={command.id}
          onMove={move}
          onRemove={() => onChange(commands.filter((_, position) => position !== index))}
          onUpdate={(next) => update(index, next)}
          showError={showErrors}
          total={commands.length}
        />
      ))}
      {commands.length === 0 ? <p className="rounded-md border border-dashed border-border px-3 py-6 text-center text-xs text-muted-foreground">{t("loops.editor.commands.empty")}</p> : null}
    </section>
  );
}

function CommandRow({ command, index, onMove, onRemove, onUpdate, showError, total }: {
  command: LoopVerificationCommandDraft;
  index: number;
  onMove: (index: number, offset: number) => void;
  onRemove: () => void;
  onUpdate: (command: LoopVerificationCommandDraft) => void;
  showError: boolean;
  total: number;
}) {
  const { t } = useTranslation();
  const issue = showError ? validateVerificationCommand(command) : null;
  return (
    <fieldset className="grid gap-3 rounded-md border border-border p-3">
      <legend className="sr-only">{t("loops.editor.commands.number", { number: index + 1 })}</legend>
      <div className="flex min-w-0 items-center gap-2">
        <span className="min-w-0 flex-1 truncate text-xs font-medium">{t("loops.editor.commands.number", { number: index + 1 })}</span>
        <CommandAction disabled={index === 0} label={t("loops.editor.commands.moveUp")} onClick={() => onMove(index, -1)}><ArrowUp aria-hidden="true" /></CommandAction>
        <CommandAction disabled={index === total - 1} label={t("loops.editor.commands.moveDown")} onClick={() => onMove(index, 1)}><ArrowDown aria-hidden="true" /></CommandAction>
        <CommandAction label={t("loops.editor.commands.remove")} onClick={onRemove}><Trash2 aria-hidden="true" /></CommandAction>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label="loops.editor.field.program"><input className={inputClass} value={command.program} onChange={(event) => onUpdate({ ...command, program: event.target.value })} /></Field>
        <Field label="loops.editor.field.workingDirectory"><input className={inputClass} value={command.workingDirectory} onChange={(event) => onUpdate({ ...command, workingDirectory: event.target.value })} /></Field>
        <Field label="loops.editor.field.arguments"><textarea className={`${inputClass} min-h-20 py-2`} value={command.arguments} onChange={(event) => onUpdate({ ...command, arguments: event.target.value })} /></Field>
        <div className="grid content-start gap-3">
          <Field label="loops.editor.field.commandTimeout"><input className={inputClass} min={1} type="number" value={command.timeoutSeconds} onChange={(event) => onUpdate({ ...command, timeoutSeconds: Number(event.target.value) })} /></Field>
          <label className="flex h-9 items-center gap-2 text-xs font-medium"><input checked={command.required} className="h-4 w-4 accent-primary" onChange={(event) => onUpdate({ ...command, required: event.target.checked })} type="checkbox" />{t("loops.editor.field.required")}</label>
        </div>
      </div>
      {issue ? <p className="text-xs text-destructive" role="alert">{t(`loops.editor.error.${issue}`)}</p> : null}
    </fieldset>
  );
}

function CommandAction({ children, disabled, label, onClick }: { children: ReactNode; disabled?: boolean; label: string; onClick: () => void }) {
  return <button aria-label={label} className="grid h-8 w-8 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-40" disabled={disabled} onClick={onClick} title={label} type="button">{children}</button>;
}

function Field({ children, label }: { children: ReactNode; label: string }) {
  const { t } = useTranslation();
  return <label className="grid gap-1.5"><span className="text-xs font-medium text-muted-foreground">{t(label)}</span>{children}</label>;
}

const inputClass = "ucd-input h-9 w-full rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring";
