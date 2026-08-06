import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import { validateExpertRoleInput } from "../../../services/expert-role-runtime";
import type { ExpertRole, SaveExpertRoleInput } from "../../../types/expert-role";

const avatarChoices = ["🏛", "🔧", "🔍", "🎨", "📐", "🧪", "📝", "🛡"];
const colorChoices = ["#9B7EBD", "#5B8C5A", "#C77D3A", "#5B9BD5", "#B5546E", "#4F9D8E"];

function draftFrom(role: ExpertRole | null): SaveExpertRoleInput {
  return {
    id: role && role.origin === "user" ? role.id : null,
    displayName: role?.displayName ?? "",
    avatar: role?.avatar ?? avatarChoices[0],
    color: role?.color ?? colorChoices[0],
    responsibility: role?.responsibility ?? "",
    instruction: role?.instruction ?? "",
    skillIds: role?.skillIds ?? [],
    reviewPolicy: role?.reviewPolicy ?? { peerReviewer: false, requireDifferentFamily: false },
    preferredProviders: role?.preferredProviders ?? [],
  };
}

export function ExpertRoleForm({
  onCancel,
  onSubmit,
  role,
  submitting,
}: {
  onCancel: () => void;
  onSubmit: (input: SaveExpertRoleInput) => void;
  /** A built-in role arrives here only when copying, so its id is dropped and a user role is created. */
  role: ExpertRole | null;
  submitting: boolean;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<SaveExpertRoleInput>(() => draftFrom(role));
  const [errors, setErrors] = useState<string[]>([]);

  function submit() {
    const found = validateExpertRoleInput(draft);
    setErrors(found);
    if (found.length === 0) onSubmit(draft);
  }

  return (
    <div className="grid content-start gap-3 rounded-lg border border-border p-3">
      <label className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("expertRoles.field.displayName")}</span>
        <input
          className="ucd-input h-9 rounded px-2 text-sm"
          onChange={(event) => setDraft({ ...draft, displayName: event.target.value })}
          value={draft.displayName}
        />
      </label>

      <div className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("expertRoles.field.avatar")}</span>
        <div className="flex flex-wrap gap-1.5">
          {avatarChoices.map((avatar) => (
            <button
              aria-pressed={draft.avatar === avatar}
              className={`ucd-interactive h-9 w-9 rounded-md border text-base ${draft.avatar === avatar ? "border-primary" : "border-border"}`}
              key={avatar}
              onClick={() => setDraft({ ...draft, avatar })}
              type="button"
            >
              {avatar}
            </button>
          ))}
        </div>
      </div>

      <div className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("expertRoles.field.color")}</span>
        <div className="flex flex-wrap gap-1.5">
          {colorChoices.map((color) => (
            <button
              aria-label={color}
              aria-pressed={draft.color === color}
              className={`ucd-interactive h-9 w-9 rounded-md border-2 ${draft.color === color ? "border-primary" : "border-transparent"}`}
              key={color}
              onClick={() => setDraft({ ...draft, color })}
              style={{ backgroundColor: color }}
              type="button"
            />
          ))}
        </div>
      </div>

      <label className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("expertRoles.field.responsibility")}</span>
        <input
          className="ucd-input h-9 rounded px-2 text-sm"
          onChange={(event) => setDraft({ ...draft, responsibility: event.target.value })}
          value={draft.responsibility}
        />
        <span className="text-[11px] text-muted-foreground">{t("expertRoles.field.responsibilityHint")}</span>
      </label>

      <label className="grid gap-1">
        <span className="text-xs font-medium text-muted-foreground">{t("expertRoles.field.instruction")}</span>
        <textarea
          className="ucd-input min-h-32 rounded p-2 text-sm"
          onChange={(event) => setDraft({ ...draft, instruction: event.target.value })}
          value={draft.instruction}
        />
        <span className="text-[11px] text-muted-foreground">{t("expertRoles.field.instructionHint")}</span>
      </label>

      <label className="flex items-center gap-2 text-sm">
        <input
          checked={draft.reviewPolicy.peerReviewer}
          onChange={(event) =>
            setDraft({
              ...draft,
              reviewPolicy: {
                peerReviewer: event.target.checked,
                // Requiring a different family is meaningless without review eligibility.
                requireDifferentFamily: event.target.checked && draft.reviewPolicy.requireDifferentFamily,
              },
            })
          }
          type="checkbox"
        />
        {t("expertRoles.field.peerReviewer")}
      </label>

      <label className="grid gap-1">
        <span className="flex items-center gap-2 text-sm">
          <input
            checked={draft.reviewPolicy.requireDifferentFamily}
            disabled={!draft.reviewPolicy.peerReviewer}
            onChange={(event) =>
              setDraft({
                ...draft,
                reviewPolicy: { ...draft.reviewPolicy, requireDifferentFamily: event.target.checked },
              })
            }
            type="checkbox"
          />
          {t("expertRoles.field.requireDifferentFamily")}
        </span>
        <span className="text-[11px] text-muted-foreground">{t("expertRoles.field.requireDifferentFamilyHint")}</span>
      </label>

      {errors.length > 0 ? (
        <ul className="grid gap-0.5 text-xs text-destructive">
          {errors.map((error) => <li key={error}>{error}</li>)}
        </ul>
      ) : null}

      <div className="flex justify-end gap-2">
        <Button className="h-8 px-3 text-xs" onClick={onCancel} type="button" variant="outline">
          {t("expertRoles.cancel")}
        </Button>
        <Button className="h-8 px-3 text-xs" disabled={submitting} onClick={submit} type="button">
          {t("expertRoles.save")}
        </Button>
      </div>
    </div>
  );
}
