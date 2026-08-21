import { Eye, Link2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { ManagedCliAgentId } from "../../../types/agent";
import type {
  PromptHook,
  PromptHookMutationInput,
  PromptHookVariableDefinition,
} from "../../../types/prompt-hook";
import { promptHookCategoryOrder } from "./prompt-hook-view-model";

export function PromptHookOverview({
  agents,
  busy,
  draft,
  hook,
  onBindingChange,
  onDraftChange,
  onEnabledChange,
}: {
  agents: { id: ManagedCliAgentId; displayName: string }[];
  busy: boolean;
  draft: PromptHookMutationInput;
  hook: PromptHook;
  onBindingChange: (agentId: ManagedCliAgentId, checked: boolean) => void;
  onDraftChange: (draft: PromptHookMutationInput) => void;
  onEnabledChange: (enabled: boolean) => void;
}) {
  const { t } = useTranslation();
  const editable = hook.source === "user";
  const set = <Key extends keyof PromptHookMutationInput>(key: Key, value: PromptHookMutationInput[Key]) => {
    onDraftChange({ ...draft, [key]: value });
  };
  return (
    <div className="space-y-4">
      {!editable ? <p className="rounded-md bg-muted p-3 text-sm text-muted-foreground">{t("promptHooks.detail.builtinReadonly")}</p> : null}
      <div className="grid gap-3 md:grid-cols-2">
        <Field disabled label="ID" value={draft.id} onChange={() => undefined} />
        <Field disabled={!editable} label={t("promptHooks.dialog.name")} value={draft.name} onChange={(value) => set("name", value)} />
        <Select disabled={!editable} label={t("promptHooks.dialog.category")} value={draft.category} onChange={(value) => set("category", value as PromptHookMutationInput["category"])}>
          {promptHookCategoryOrder.map((category) => <option key={category} value={category}>{t(`promptHooks.category.${category}`)}</option>)}
        </Select>
        <Select disabled={!editable} label={t("promptHooks.dialog.stage")} value={draft.stage} onChange={(value) => set("stage", value as PromptHookMutationInput["stage"])}>
          {(["session-init", "per-turn"] as const).map((stage) => <option key={stage} value={stage}>{t(`promptHooks.stage.${stage}`)}</option>)}
        </Select>
      </div>
      <Field disabled={!editable} label={t("promptHooks.dialog.description")} value={draft.description} onChange={(value) => set("description", value)} />
      <label className="block text-sm">
        {t("promptHooks.dialog.order")}
        <input
          className="mt-1 w-full rounded-md border border-border bg-background px-3 py-2 text-sm disabled:bg-muted"
          disabled={!editable}
          onChange={(event) => set("order", Number(event.target.value))}
          type="number"
          value={draft.order}
        />
      </label>
      <div className="flex flex-wrap gap-2">
        <Badge tone="muted">{t(`promptHooks.governance.${hook.governance.governanceTier}`)}</Badge>
        <Badge tone="muted">{hook.governance.safetyTier}</Badge>
        <Badge tone="muted">{hook.governance.transparencyTier}</Badge>
      </div>
      <label className="flex items-center gap-2 text-sm font-medium">
        <input
          checked={hook.enabled}
          className="h-4 w-4 accent-[hsl(var(--primary))]"
          disabled={!hook.disableable || busy}
          onChange={(event) => onEnabledChange(event.target.checked)}
          type="checkbox"
        />
        {t("promptHooks.enabled")}
      </label>
      <fieldset>
        <legend className="flex items-center gap-2 text-sm font-medium">
          <Link2 className="h-4 w-4 text-primary" aria-hidden="true" />
          {t("promptHooks.filters.agent")}
        </legend>
        <div className="mt-2 grid gap-2 sm:grid-cols-2">
          {agents.map((agent) => (
            <label className="flex min-w-0 items-center gap-2 rounded-md border border-border bg-[hsl(var(--panel-muted))] px-3 py-2 text-sm" key={agent.id}>
              <input
                checked={hook.cliBindings.includes(agent.id)}
                className="h-4 w-4 shrink-0 accent-[hsl(var(--primary))]"
                disabled={busy}
                onChange={(event) => onBindingChange(agent.id, event.target.checked)}
                type="checkbox"
              />
              <span className="truncate">{agent.displayName}</span>
            </label>
          ))}
        </div>
      </fieldset>
    </div>
  );
}

export function PromptHookContentEditor({
  draft,
  hook,
  variables,
  onChange,
  onPreview,
}: {
  draft: PromptHookMutationInput;
  hook: PromptHook;
  variables: PromptHookVariableDefinition[];
  onChange: (draft: PromptHookMutationInput) => void;
  onPreview: () => void;
}) {
  const { t } = useTranslation();
  if (hook.source === "builtin") {
    return (
      <div className="space-y-4">
        <p className="rounded-md bg-muted p-3 text-sm text-muted-foreground">{t("promptHooks.detail.builtinReadonly")}</p>
        <Button onClick={onPreview} variant="outline"><Eye aria-hidden="true" />{t("promptHooks.actions.preview")}</Button>
      </div>
    );
  }
  const insert = (token: string) => onChange({
    ...draft,
    templateBody: `${draft.templateBody}${draft.templateBody ? " " : ""}${token}`,
  });
  return (
    <div className="space-y-4">
      <div>
        <div className="text-sm font-medium">{t("promptHooks.lifecycle.variables")}</div>
        <div className="mt-2 grid gap-2 sm:grid-cols-2">
          {variables.map((variable) => (
            <button
              aria-label={variable.token}
              className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-2 text-left hover:bg-accent focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
              key={variable.name}
              onClick={() => insert(variable.token)}
              type="button"
            >
              <span className="block font-mono text-xs font-semibold">{variable.token}</span>
              <span className="mt-1 block text-xs text-muted-foreground">{t(variable.descriptionKey)}</span>
              <span className="mt-1 block text-xs">{t(variable.availabilityKey)} · <span className="font-mono">{variable.example}</span></span>
            </button>
          ))}
        </div>
      </div>
      <label className="block text-sm">
        {t("promptHooks.dialog.body")}
        <textarea
          className="mt-1 min-h-64 w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          onChange={(event) => onChange({ ...draft, templateBody: event.target.value })}
          value={draft.templateBody}
        />
      </label>
      <div className="flex justify-between gap-2">
        <p className="text-xs leading-5 text-muted-foreground">{t("promptHooks.lifecycle.variableSafety")}</p>
        <Button onClick={onPreview} size="sm" variant="outline"><Eye aria-hidden="true" />{t("promptHooks.actions.preview")}</Button>
      </div>
    </div>
  );
}

function Field({ label, value, disabled, onChange }: { label: string; value: string; disabled?: boolean; onChange: (value: string) => void }) {
  return (
    <label className="block text-sm">
      {label}
      <input className="mt-1 w-full rounded-md border border-border bg-background px-3 py-2 text-sm disabled:bg-muted" disabled={disabled} onChange={(event) => onChange(event.target.value)} value={value} />
    </label>
  );
}

function Select({ label, value, disabled, children, onChange }: { label: string; value: string; disabled?: boolean; children: React.ReactNode; onChange: (value: string) => void }) {
  return (
    <label className="block text-sm">
      {label}
      <select className="mt-1 w-full rounded-md border border-border bg-background px-3 py-2 text-sm disabled:bg-muted" disabled={disabled} onChange={(event) => onChange(event.target.value)} value={value}>{children}</select>
    </label>
  );
}
