import { AlertTriangle, CheckCircle2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { normalizeDisplayPath } from "../../lib/session-path";
import type { CliInstallation } from "../../types/cli-environment-snapshot";

export function CliInstallationList({ installations }: { installations: readonly CliInstallation[] }) {
  const { t } = useTranslation();

  if (installations.length === 0) {
    return <p className="text-xs text-muted-foreground">{t("cli.diagnostics.none")}</p>;
  }

  return (
    <ul className="grid gap-2">
      {installations.map((installation) => (
        <li className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-2" key={installation.id}>
          <div className="flex flex-wrap items-center gap-2 text-xs">
            {installation.executableStatus === "healthy" ? (
              <CheckCircle2 className="h-3.5 w-3.5 text-[hsl(var(--success))]" aria-hidden="true" />
            ) : (
              <AlertTriangle className="h-3.5 w-3.5 text-[hsl(var(--warning))]" aria-hidden="true" />
            )}
            <span className="rounded border border-border px-1.5 py-0.5 font-medium">
              {t(`cli.source.${installation.sourceId ?? "unknown"}`)}
            </span>
            {/* How sure the backend is about that source. An inferred guess is not a fact. */}
            <span className="text-[11px] text-muted-foreground">
              {t(`cli.confidence.${installation.sourceConfidence}`)}
            </span>
            {installation.pathPriority !== null ? (
              <span className="rounded border border-border px-1.5 py-0.5 text-muted-foreground">
                {t("cli.diagnostics.pathPosition", { position: installation.pathPriority + 1 })}
              </span>
            ) : null}
            <span className="ml-auto font-mono text-muted-foreground">
              {installation.reportedVersion ?? t("cli.versionUnknown")}
            </span>
          </div>
          <div className="mt-2 break-all font-mono text-[11px] text-muted-foreground">
            {normalizeDisplayPath(installation.executablePath)}
          </div>
          {/* Aliases folded into this entry, so three files on disk read as one installation. */}
          {installation.aliasPaths.length > 0 ? (
            <div className="mt-1 text-[11px] text-muted-foreground">
              {t("cli.diagnostics.aliases", { count: installation.aliasPaths.length })}
            </div>
          ) : null}
          {installation.targetMissing ? (
            <div className="mt-1 text-[11px] text-[hsl(var(--warning))]">{t("cli.diagnostics.targetMissing")}</div>
          ) : null}
        </li>
      ))}
    </ul>
  );
}
