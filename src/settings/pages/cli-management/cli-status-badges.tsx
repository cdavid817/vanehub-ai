import { AlertTriangle, CheckCircle2, CircleHelp, Clock, ShieldAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";

/**
 * The status axes, each rendered from the backend's own value.
 *
 * Every badge carries an icon as well as a tone, because colour alone is not a status cue for
 * anyone who cannot distinguish the tones.
 */

type Tone = "success" | "warning" | "danger" | "muted";

/** Icons paired with tones, so the meaning survives without colour. */
const TONE_ICONS = {
  success: CheckCircle2,
  warning: AlertTriangle,
  danger: ShieldAlert,
  muted: CircleHelp,
} as const;

// The values below are the backend's own `as_str` output. Spelling one differently here does not
// fail to compile -- it silently renders every value in the neutral tone forever.
function toneOfExecutable(value: string): Tone {
  if (value === "healthy") return "success";
  if (value === "broken" || value === "unsupported-architecture") return "danger";
  if (value === "timeout" || value === "permission-denied") return "warning";
  return "muted";
}

function toneOfAuthentication(value: string): Tone {
  if (value === "authenticated") return "success";
  if (value === "expired" || value === "required") return "warning";
  return "muted";
}

function toneOfCompatibility(value: string): Tone {
  if (value === "supported") return "success";
  if (value === "unsupported-version" || value === "unsupported-platform") return "danger";
  return "muted";
}

function toneOfUpdate(value: string): Tone {
  if (value === "up-to-date") return "success";
  if (value === "available") return "warning";
  return "muted";
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
  const Icon = TONE_ICONS[tone];
  return (
    <Badge className="gap-1" tone={tone} title={t(labelKey)}>
      <Icon aria-hidden="true" className="h-3 w-3" />
      {t(value)}
    </Badge>
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
        <Badge className="gap-1" tone="warning" title={t("cli.axis.freshness")}>
          <Clock aria-hidden="true" className="h-3 w-3" />
          {t("cli.freshness.stale")}
        </Badge>
      ) : null}
    </div>
  );
}
