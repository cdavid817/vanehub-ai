import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "../../../components/ui/button";
import { formatAppDateTime } from "../../../i18n/format";
import { localMediaMessageKey } from "../../../session-workspace/local-media/local-media-errors";
import type { EngineStatus, LocalMediaEngine } from "../../../types/local-media";
import { SectionPanel, StatusPill } from "../page-parts";
import { ToggleField } from "./profile-fields";

type Tone = "success" | "warning" | "danger" | "muted";

const READINESS_TONE: Record<string, Tone> = {
  ready: "success",
  checking: "muted",
  disabled: "muted",
  unconfigured: "warning",
  restartRequired: "warning",
  unavailable: "danger",
};

/**
 * The card for one engine.
 *
 * Each card owns its own enable switch, readiness, and check button, and reads nothing from the
 * other two. That independence is the requirement, not a convenience: a machine with a working
 * PaddleOCR install and no microphone must keep OCR usable, so a failure has to be presentable
 * per engine or the page will end up gating all three on the weakest one.
 */
export function EngineCard({
  children,
  description,
  enabled,
  engine,
  icon,
  onProbe,
  onToggle,
  probeDisabledReasonKey,
  probing,
  status,
  title,
}: {
  children: ReactNode;
  description: string;
  enabled: boolean;
  engine: LocalMediaEngine;
  icon: LucideIcon;
  onProbe: () => void;
  onToggle: (value: boolean) => void;
  /** Set when checking would be meaningless right now -- unsaved edits, or a disabled engine. */
  probeDisabledReasonKey: string | null;
  probing: boolean;
  status: EngineStatus | undefined;
  title: string;
}) {
  const { t } = useTranslation();
  const readiness = status?.readiness;
  const state = probing ? "checking" : (readiness?.state ?? "disabled");

  return (
    <SectionPanel
      description={description}
      icon={icon}
      title={title}
      variant="settings"
    >
      <div className="grid gap-4 px-5 py-4 sm:px-6" data-testid={`local-media-card-${engine}`}>
        <div className="flex flex-wrap items-center gap-3">
          <StatusPill
            status={t(`localMedia.settings.readiness.${state}`)}
            tone={READINESS_TONE[state] ?? "muted"}
          />
          <ToggleField
            checked={enabled}
            label={t("localMedia.settings.enableEngine", { engine: title })}
            onChange={onToggle}
          />
          <Button
            data-testid={`local-media-probe-${engine}`}
            disabled={probing || probeDisabledReasonKey !== null}
            onClick={onProbe}
            size="sm"
            title={probeDisabledReasonKey ? t(probeDisabledReasonKey) : undefined}
            type="button"
            variant="outline"
          >
            {t(probing ? "localMedia.settings.checking" : "localMedia.settings.check")}
          </Button>
        </div>

        {readiness?.state === "unavailable" ? (
          <p className="ucd-status-danger rounded-lg px-3 py-2 text-xs" role="status">
            {t(localMediaMessageKey(readiness.code))}
          </p>
        ) : null}

        {status?.workerState === "quarantined" ? (
          <p className="ucd-status-warning rounded-lg px-3 py-2 text-xs">
            {t("localMedia.settings.quarantined")}
          </p>
        ) : null}

        <EngineMetadata status={status} />

        <div className="grid gap-4 sm:grid-cols-2">{children}</div>
      </div>
    </SectionPanel>
  );
}

/**
 * Version, model identity, and device as the probe reported them.
 *
 * These are host facts, not translated copy, and they are the only evidence the user has that a
 * "ready" badge refers to the install they think it does.
 */
function EngineMetadata({ status }: { status: EngineStatus | undefined }) {
  const { i18n, t } = useTranslation();
  const entries: Array<[string, string]> = [];
  if (status?.installedVersion) entries.push(["version", status.installedVersion]);
  if (status?.modelIdentity) entries.push(["model", status.modelIdentity]);
  if (status?.deviceSummary) entries.push(["device", status.deviceSummary]);
  if (status?.lastCheckedAt) {
    entries.push([
      "checkedAt",
      formatAppDateTime(status.lastCheckedAt, i18n.language, {
        dateStyle: "short",
        timeStyle: "short",
      }),
    ]);
  }
  if (entries.length === 0) return null;

  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-xs text-muted-foreground">
      {entries.map(([key, value]) => (
        <div className="contents" key={key}>
          <dt>{t(`localMedia.settings.meta.${key}`)}</dt>
          <dd className="truncate font-mono">{value}</dd>
        </div>
      ))}
    </dl>
  );
}
