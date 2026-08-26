import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { formatAppDateTime } from "../../../i18n/format";
import type { LspNegotiatedCapabilities, LspProcessState, LspServerStatus } from "../../../types/lsp";
import { StatusPill } from "../page-parts";

function stateTone(state: LspProcessState): "success" | "warning" | "danger" | "muted" {
  if (state === "ready") return "success";
  if (state === "failed") return "danger";
  if (state === "absent") return "muted";
  return "warning";
}

function CapabilitySummary({ capabilities }: { capabilities: LspNegotiatedCapabilities }) {
  const { t } = useTranslation();
  const positionEncoding = capabilities.positionEncoding === "utf16" ? "UTF-16" : "UTF-8";

  return (
    <section className="mt-4 border-t border-border/70 pt-4" aria-label={t("lspSettings.runtime.capabilities")}>
      <h5 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {t("lspSettings.runtime.capabilities")}
      </h5>
      <dl className="mt-3 grid gap-2 text-xs sm:grid-cols-2">
        <div className="flex items-center justify-between gap-2 rounded-md bg-background/70 p-2">
          <dt>{t("lspSettings.capability.positionEncoding")}</dt>
          <dd><code>{positionEncoding}</code></dd>
        </div>
        <div className="flex items-center justify-between gap-2 rounded-md bg-background/70 p-2">
          <dt>{t("lspSettings.capability.documentSync")}</dt>
          <dd><code>{capabilities.documentSync}</code></dd>
        </div>
        {capabilities.methods.map(({ method, supported }) => (
          <div className="flex items-center justify-between gap-2 rounded-md bg-background/70 p-2" key={method}>
            {/* Falls back to the raw identifier rather than rendering the missing key, so a method
                added to the backend does not blank out its row in every stale locale. */}
            <dt>{t(`lspSettings.capability.${method}`, { defaultValue: method })}</dt>
            <dd>
              <Badge tone={supported ? "success" : "muted"}>
                {t(supported ? "lspSettings.capability.enabled" : "lspSettings.capability.disabled")}
              </Badge>
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

function RuntimeMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border/70 bg-background/70 p-3">
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="mt-1 break-all text-sm font-medium">{value}</dd>
    </div>
  );
}

export function LspRuntimeStatusCard({ status }: { status: LspServerStatus }) {
  const { t, i18n } = useTranslation();
  const languageName = t(`lspSettings.language.${status.language}`);
  const lastResponse = status.lastResponseAt
    ? formatAppDateTime(status.lastResponseAt, i18n.language, {
      dateStyle: "short",
      timeStyle: "medium",
    })
    : t("lspSettings.runtime.never");

  return (
    <article
      aria-label={`${languageName} ${status.server}`}
      className="rounded-lg border border-border bg-muted/15 p-4"
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h4 className="text-sm font-semibold">{languageName}</h4>
          <code className="mt-1 block text-xs text-muted-foreground">{status.server}</code>
        </div>
        <StatusPill status={t(`lspSettings.state.${status.state}`)} tone={stateTone(status.state)} />
      </div>

      {status.reasonCode ? (
        <p className="mt-3 rounded-md border p-2 text-xs ucd-status-warning">
          {t(`lspSettings.reason.${status.reasonCode}`)}
        </p>
      ) : null}

      <dl className="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <RuntimeMetric label={t("lspSettings.runtime.relativeProjectRoot")} value={status.relativeProjectRoot} />
        <RuntimeMetric label={t("lspSettings.runtime.restartCount")} value={String(status.restartCount)} />
        <RuntimeMetric label={t("lspSettings.runtime.lastResponse")} value={lastResponse} />
        <RuntimeMetric label={t("lspSettings.runtime.diagnostics")} value={String(status.diagnosticCount)} />
      </dl>

      {status.negotiatedCapabilities ? (
        <CapabilitySummary capabilities={status.negotiatedCapabilities} />
      ) : null}
    </article>
  );
}
