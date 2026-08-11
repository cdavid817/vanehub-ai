import { useState } from "react";
import { CircleAlert, LoaderCircle, ShieldAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../components/ui/application-dialog";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import type { SessionRecoverySummary } from "../services/agent-service";
import type { Session } from "../types/agent";

export function SessionRecoveryNotice({
  acknowledging,
  onAcknowledge,
  session,
  summary,
}: {
  acknowledging: boolean;
  onAcknowledge: () => Promise<void>;
  session: Session | null;
  summary: SessionRecoverySummary | null;
}) {
  const { t } = useTranslation();
  const [confirming, setConfirming] = useState(false);
  if (!session || session.recoveryStatus === "clean") return null;

  const actionRequired = session.recoveryStatus === "action_required";
  const quarantined = session.recoveryStatus === "quarantined";
  const Icon = session.recoveryStatus === "reconciling"
    ? LoaderCircle
    : quarantined ? ShieldAlert : CircleAlert;
  const tone = quarantined ? "danger" : "warning";

  return (
    <>
      <section
        aria-live={actionRequired || quarantined ? "assertive" : "polite"}
        className="rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3 shadow-xs"
        data-testid="session-recovery-notice"
        role={actionRequired || quarantined ? "alert" : "status"}
      >
        <div className="flex items-start gap-3">
          <span className="mt-0.5 grid h-8 w-8 shrink-0 place-items-center rounded-full bg-muted">
            <Icon
              aria-hidden="true"
              className={session.recoveryStatus === "reconciling" ? "animate-spin text-primary motion-reduce:animate-none" : "text-foreground"}
            />
          </span>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-sm font-semibold">{t(`recovery.${session.recoveryStatus}.title`)}</h3>
              <Badge tone={tone}>{t(`recovery.${session.recoveryStatus}.badge`)}</Badge>
            </div>
            <p className="mt-1 text-sm leading-5 text-muted-foreground">
              {t(`recovery.${session.recoveryStatus}.description`)}
            </p>
            {summary?.latestReport ? (
              <p className="mt-1 text-xs text-muted-foreground">
                {t("recovery.reportReference", {
                  revision: summary.latestReport.recoveryRevision,
                  count: summary.latestReport.reasonCodes.length,
                })}
              </p>
            ) : null}
          </div>
          {actionRequired ? (
            <Button onClick={() => setConfirming(true)} size="sm" type="button" variant="outline">
              {t("recovery.acknowledge.open")}
            </Button>
          ) : null}
        </div>
      </section>
      {confirming ? (
        <ApplicationDialog
          closeDisabled={acknowledging}
          description={t("recovery.acknowledge.description")}
          maxWidth="max-w-lg"
          onClose={() => setConfirming(false)}
          title={t("recovery.acknowledge.title")}
        >
          <div className="space-y-4">
            <div className="rounded-lg border border-border bg-muted/40 p-3 text-sm leading-6">
              <p>{t("recovery.acknowledge.noRetry")}</p>
              <p className="mt-2 font-medium">{t("recovery.acknowledge.uncertainEffect")}</p>
            </div>
            <div className="flex justify-end gap-2">
              <Button disabled={acknowledging} onClick={() => setConfirming(false)} type="button" variant="ghost">
                {t("recovery.acknowledge.cancel")}
              </Button>
              <Button
                data-dialog-autofocus
                disabled={acknowledging}
                onClick={() => {
                  void onAcknowledge()
                    .then(() => setConfirming(false))
                    .catch(() => undefined);
                }}
                type="button"
              >
                {acknowledging ? t("recovery.acknowledge.pending") : t("recovery.acknowledge.confirm")}
              </Button>
            </div>
          </div>
        </ApplicationDialog>
      ) : null}
    </>
  );
}
