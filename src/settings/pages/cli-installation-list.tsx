import { AlertTriangle, CheckCircle2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { normalizeDisplayPath } from "../../lib/session-path";
import type { CliInstallation } from "../../types/agent";

export function CliInstallationList({ installations }: { installations: CliInstallation[] }) {
  const { t } = useTranslation();

  if (installations.length === 0) {
    return <p className="text-xs text-muted-foreground">{t("cli.diagnostics.none")}</p>;
  }

  return (
    <ul className="grid gap-2">
      {installations.map((installation) => (
        <li className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-2" key={installation.path}>
          <div className="flex flex-wrap items-center gap-2 text-xs">
            {installation.runnable ? (
              <CheckCircle2 className="h-3.5 w-3.5 text-[hsl(var(--success))]" aria-hidden="true" />
            ) : (
              <AlertTriangle className="h-3.5 w-3.5 text-[hsl(var(--warning))]" aria-hidden="true" />
            )}
            <span className="rounded border border-border px-1.5 py-0.5 font-medium">
              {t(`cli.source.${installation.source}`)}
            </span>
            {installation.isActive ? (
              <span className="rounded border border-primary bg-[hsl(var(--nav-active-soft))] px-1.5 py-0.5 text-primary">
                {t("cli.diagnostics.active")}
              </span>
            ) : null}
            <span className="ml-auto font-mono text-muted-foreground">
              {installation.version ?? t("cli.versionUnknown")}
            </span>
          </div>
          <div className="mt-2 break-all font-mono text-[11px] text-muted-foreground">{normalizeDisplayPath(installation.path)}</div>
          {installation.error ? <div className="mt-1 text-[11px] text-[hsl(var(--warning))]">{installation.error}</div> : null}
        </li>
      ))}
    </ul>
  );
}
