import { useTranslation } from "react-i18next";
import type { SessionPersonalizationMode } from "../types/personalization";

const MODES: SessionPersonalizationMode[] = ["standard", "project-only", "temporary"];

/**
 * How much personalization the new session may use, chosen beside the workspace it applies to.
 *
 * Project-only needs a workspace to be "only" about, so without one it is offered as disabled with
 * a reason rather than hidden. Hiding it would leave a user who was told the mode exists unable to
 * find it, and silently falling back to standard would give them a session that reads memories they
 * meant to exclude.
 */
export function SessionPersonalizationModeSelector({
  hasWorkspace,
  mode,
  onChange,
}: {
  hasWorkspace: boolean;
  mode: SessionPersonalizationMode;
  onChange: (mode: SessionPersonalizationMode) => void;
}) {
  const { t } = useTranslation();
  const blockedId = "session-personalization-mode-blocked";
  const projectOnlyBlocked = !hasWorkspace;

  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs font-medium" htmlFor="session-personalization-mode">
        {t("createSession.personalization.label")}
      </label>
      <select
        aria-describedby={projectOnlyBlocked ? blockedId : undefined}
        className="ucd-input h-9 rounded-md px-2 text-sm"
        data-testid="session-personalization-mode"
        id="session-personalization-mode"
        onChange={(event) => onChange(event.target.value as SessionPersonalizationMode)}
        value={mode}
      >
        {MODES.map((candidate) => (
          <option
            disabled={candidate === "project-only" && projectOnlyBlocked}
            key={candidate}
            value={candidate}
          >
            {t(`personalization.preview.modeValue.${candidate}`)}
          </option>
        ))}
      </select>
      <p className="text-xs text-muted-foreground" data-testid="session-personalization-mode-help">
        {t(`createSession.personalization.help.${mode}`)}
      </p>
      {projectOnlyBlocked ? (
        // Described by the select rather than only shown next to it, so a screen reader reaching
        // the disabled option is told why it cannot be chosen.
        <p className="text-xs ucd-status-warning" id={blockedId} data-testid="session-personalization-mode-blocked">
          {t("createSession.personalization.needsWorkspace")}
        </p>
      ) : null}
    </div>
  );
}

/**
 * Keeps the selection legal when the workspace goes away.
 *
 * A session created project-only without a workspace is refused by the store, so the choice is
 * corrected here instead of failing at submit with an error about a control the user cannot see.
 */
export function modeForWorkspace(
  mode: SessionPersonalizationMode,
  hasWorkspace: boolean,
): SessionPersonalizationMode {
  return mode === "project-only" && !hasWorkspace ? "standard" : mode;
}
