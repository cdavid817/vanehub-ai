import { AlertTriangle, CheckCircle2, CircleHelp, Clock, ShieldAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { StatusBadge } from "../../../ui/status/StatusBadge";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";

/**
 * The status axes, each rendered from the backend's own value through the shared `StatusBadge`
 * (task 12.18) -- every badge carries an icon as well as a tone, because colour alone is not a
 * status cue for anyone who cannot distinguish the tones.
 */

type Tone = "success" | "warning" | "danger" | "neutral";

/** Icons paired with tones, so the meaning survives without colour. */
const TONE_ICONS: Record<Tone, typeof CheckCircle2> = {
  success: CheckCircle2,
  warning: AlertTriangle,
  danger: ShieldAlert,
  neutral: CircleHelp,
};

// The values below are the backend's own `as_str` output. Spelling one differently here does not
// fail to compile -- it silently renders every value in the neutral tone forever.
function toneOfExecutable(value: string): Tone {
  if (value === "healthy") return "success";
  if (value === "broken" || value === "unsupported-architecture") return "danger";
  if (value === "timeout" || value === "permission-denied") return "warning";
  return "neutral";
}

function toneOfAuthentication(value: string): Tone {
  if (value === "authenticated") return "success";
  if (value === "expired" || value === "required") return "warning";
  return "neutral";
}

function toneOfCompatibility(value: string): Tone {
  if (value === "supported") return "success";
  if (value === "unsupported-version" || value === "unsupported-platform") return "danger";
  return "neutral";
}

function toneOfUpdate(value: string): Tone {
  if (value === "up-to-date") return "success";
  if (value === "available") return "warning";
  return "neutral";
}

export function CliStatusBadge({
  labelKey,
  value,
  tone,
}: {
  labelKey: string;
  value: string;
  tone: Tone;
}) {
  const { t } = useTranslation();
  return (
    <StatusBadge
      description={t(labelKey)}
      icon={TONE_ICONS[tone]}
      label={t(value)}
      tone={tone}
    />
  );
}

/** The compact row a collapsed card shows. Freshness is included because stale data is a status. */
export function CliStatusBadges({ snapshot }: { snapshot: CliEnvironmentSnapshot }) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <CliStatusBadge
        labelKey="cli.axis.executable"
        tone={toneOfExecutable(snapshot.executable)}
        value={`cli.executable.${snapshot.executable}`}
      />
      <CliStatusBadge
        labelKey="cli.axis.authentication"
        tone={toneOfAuthentication(snapshot.authentication)}
        value={`cli.authentication.${snapshot.authentication}`}
      />
      <CliStatusBadge
        labelKey="cli.axis.compatibility"
        tone={toneOfCompatibility(snapshot.compatibility)}
        value={`cli.compatibility.${snapshot.compatibility}`}
      />
      <CliStatusBadge
        labelKey="cli.axis.update"
        tone={toneOfUpdate(snapshot.update)}
        value={`cli.update.${snapshot.update}`}
      />
      {snapshot.freshness === "stale" ? (
        <StatusBadge
          description={t("cli.axis.freshness")}
          icon={Clock}
          label={t("cli.freshness.stale")}
          tone="warning"
        />
      ) : null}
    </div>
  );
}
