import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import {
  countCustomInstructionsCharacters,
  customInstructionsFieldCharacterLimit,
} from "../../../types/settings";
import type { InstructionMergeMode } from "../../../types/personalization";
import type { InstructionDraft, InstructionValues } from "./instruction-drafts";

/**
 * Mirrors the native estimator's `CHARACTERS_PER_TOKEN`.
 *
 * Coarse on purpose, in both places: this is a sense of size for a settings screen, and tokenizing
 * here would tie the number to whichever tokenizer one provider happens to use.
 */
const CHARACTERS_PER_TOKEN = 4;

const MERGE_MODES: InstructionMergeMode[] = ["inherit", "append", "replace", "disabled"];

export function approximateTokens(values: InstructionValues): number {
  const characters =
    countCustomInstructionsCharacters(values.aboutUser)
    + countCustomInstructionsCharacters(values.styleRules);
  return Math.ceil(characters / CHARACTERS_PER_TOKEN);
}

export function overLimitFields(values: InstructionValues): ("aboutUser" | "styleRules")[] {
  return (["aboutUser", "styleRules"] as const).filter(
    (field) => countCustomInstructionsCharacters(values[field]) > customInstructionsFieldCharacterLimit,
  );
}

function Field({
  disabled,
  field,
  onChange,
  value,
}: {
  disabled: boolean;
  field: "aboutUser" | "styleRules";
  onChange: (value: string) => void;
  value: string;
}) {
  const { t } = useTranslation();
  const count = countCustomInstructionsCharacters(value);
  const over = count > customInstructionsFieldCharacterLimit;

  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs font-medium" htmlFor={`personalization-${field}`}>
        {t(`personalization.editor.${field}`)}
      </label>
      <textarea
        aria-describedby={`personalization-${field}-count`}
        aria-invalid={over}
        className="ucd-input min-h-24 rounded-md p-2 text-sm"
        data-testid={`personalization-field-${field}`}
        disabled={disabled}
        id={`personalization-${field}`}
        onChange={(event) => onChange(event.target.value)}
        placeholder={t(`personalization.editor.${field}Placeholder`)}
        value={value}
      />
      <p
        className={`text-xs ${over ? "ucd-status-danger" : "text-muted-foreground"}`}
        data-testid={`personalization-count-${field}`}
        id={`personalization-${field}-count`}
      >
        {t("personalization.editor.characterCount", {
          count,
          limit: customInstructionsFieldCharacterLimit,
        })}
        {over ? ` — ${t("personalization.editor.overLimit")}` : ""}
      </p>
    </div>
  );
}

/**
 * Explicit Save and Discard, never a save on blur.
 *
 * A blur-save writes on every focus change, so a half-finished sentence and a stray Tab both reach
 * the store, and there is no moment at which the user has said the text is ready. It also has
 * nowhere to put a validation failure: the field is already saved by the time anyone looks.
 */
/**
 * Whether a key event is the IME committing a candidate rather than the user pressing a key.
 *
 * Without this, confirming a Japanese or Chinese candidate with Enter -- or typing `s` while a
 * modifier is held by the input method -- reaches the shortcut, and the half-composed text is
 * saved. `keyCode === 229` is the legacy signal browsers still send when `isComposing` is absent.
 */
function isComposing(event: React.KeyboardEvent): boolean {
  return event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229;
}

export function InstructionEditor({
  draft,
  onDiscard,
  onEdit,
  onSave,
}: {
  draft: InstructionDraft;
  onDiscard: () => void;
  onEdit: (patch: Partial<InstructionValues>) => void;
  onSave: () => void;
}) {
  const { t } = useTranslation();
  const overLimit = overLimitFields(draft.values);
  const dirty =
    draft.values.aboutUser !== draft.baseline.aboutUser
    || draft.values.styleRules !== draft.baseline.styleRules
    || draft.values.instructionMergeMode !== draft.baseline.instructionMergeMode;
  const blocked = !dirty || draft.saving || draft.conflict !== null || overLimit.length > 0;

  function onKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (isComposing(event)) return;
    const isSaveChord = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s";
    if (!isSaveChord) return;
    event.preventDefault();
    if (!blocked) onSave();
  }

  return (
    <div
      className="flex flex-col gap-4"
      data-testid="personalization-instruction-editor"
      onKeyDown={onKeyDown}
    >
      <label className="flex max-w-xs flex-col gap-1 text-xs font-medium">
        {t("personalization.editor.mergeMode")}
        <select
          className="ucd-input h-9 rounded-md px-2 text-sm"
          data-testid="personalization-merge-mode"
          disabled={draft.saving}
          onChange={(event) =>
            onEdit({ instructionMergeMode: event.target.value as InstructionMergeMode })
          }
          value={draft.values.instructionMergeMode}
        >
          {MERGE_MODES.map((mode) => (
            <option key={mode} value={mode}>
              {t(`personalization.editor.merge.${mode}`)}
            </option>
          ))}
        </select>
      </label>

      <Field
        disabled={draft.saving}
        field="aboutUser"
        onChange={(value) => onEdit({ aboutUser: value })}
        value={draft.values.aboutUser}
      />
      <Field
        disabled={draft.saving}
        field="styleRules"
        onChange={(value) => onEdit({ styleRules: value })}
        value={draft.values.styleRules}
      />

      <div className="flex flex-wrap items-center gap-3">
        <Button data-testid="personalization-save" disabled={blocked} onClick={onSave}>
          {draft.saving ? t("personalization.editor.saving") : t("personalization.editor.save")}
        </Button>
        <Button
          data-testid="personalization-discard"
          disabled={!dirty || draft.saving}
          onClick={onDiscard}
          variant="outline"
        >
          {t("personalization.editor.discard")}
        </Button>
        <span aria-live="polite" className="text-xs text-muted-foreground">
          {dirty ? (
            <span data-testid="personalization-dirty">{t("personalization.editor.unsaved")}</span>
          ) : null}
        </span>
        <span className="ml-auto text-xs text-muted-foreground" data-testid="personalization-tokens">
          {t("personalization.editor.approximateTokens", { count: approximateTokens(draft.values) })}
        </span>
      </div>

      {draft.error ? (
        <p className="text-sm ucd-status-danger" data-testid="personalization-save-error" role="alert">
          {t("personalization.editor.saveFailed")}
        </p>
      ) : null}
    </div>
  );
}
