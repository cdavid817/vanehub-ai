import { Copy } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { normalizeDisplayPath } from "../../../lib/session-path";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";

/**
 * Every installation found, with what the backend concluded about each.
 *
 * Paths are truncated for layout and carried in full on `title` and the copy button, because a
 * truncated path is unusable for the thing a user actually does with one: paste it somewhere.
 */
export function CliInstallationsTab({ snapshot }: { snapshot: CliEnvironmentSnapshot }) {
  const { t } = useTranslation();

  if (snapshot.installations.length === 0) {
    return <p className="text-sm text-muted-foreground">{t("cli.diagnostics.none")}</p>;
  }

  return (
    <ul className="grid gap-3">
      {snapshot.installations.map((installation) => {
        const pathSelected = installation.id === snapshot.pathSelectedInstallationId;
        const recommended = installation.id === snapshot.recommendedInstallationId;
        // Shadowed: PATH reaches something else first, so this one never runs from a shell.
        const shadowed = !pathSelected
          && snapshot.pathSelectedInstallationId !== null
          && installation.pathPriority !== null;
        return (
          <li className="rounded-md border border-border p-3 text-xs" key={installation.id}>
            <div className="flex flex-wrap items-center gap-1.5">
              <Badge tone="muted">{t(`cli.source.${installation.sourceId ?? "unknown"}`)}</Badge>
              <span className="text-muted-foreground">
                {t(`cli.confidence.${installation.sourceConfidence}`)}
              </span>
              {pathSelected ? <Badge tone="success">{t("cli.pathSelectedBadge")}</Badge> : null}
              {recommended ? <Badge tone="default">{t("cli.recommendedBadge")}</Badge> : null}
              {shadowed ? <Badge tone="warning">{t("cli.shadowedBadge")}</Badge> : null}
              <span className="ml-auto font-mono">
                {installation.reportedVersion ?? t("cli.versionUnknown")}
              </span>
            </div>

            <div className="mt-2 flex items-start gap-2">
              <span className="min-w-0 flex-1 truncate font-mono" title={installation.executablePath}>
                {normalizeDisplayPath(installation.executablePath)}
              </span>
              <Button
                aria-label={t("cli.copyPath")}
                size="icon"
                title={t("cli.copyPath")}
                variant="ghost"
                onClick={() => void navigator.clipboard?.writeText(installation.executablePath)}
              >
                <Copy aria-hidden="true" className="h-3.5 w-3.5" />
              </Button>
            </div>

            <dl className="mt-2 grid gap-1 sm:grid-cols-2">
              {installation.canonicalPath ? (
                <div className="min-w-0 sm:col-span-2">
                  <dt className="text-muted-foreground">{t("cli.canonicalPath")}</dt>
                  <dd className="truncate font-mono" title={installation.canonicalPath}>
                    {normalizeDisplayPath(installation.canonicalPath)}
                  </dd>
                </div>
              ) : null}
              <div>
                <dt className="text-muted-foreground">{t("cli.pathPriority")}</dt>
                <dd>
                  {installation.pathPriority !== null
                    ? t("cli.diagnostics.pathPosition", { position: installation.pathPriority + 1 })
                    : t(`cli.origin.${installation.environmentOrigin}`)}
                </dd>
              </div>
              <div>
                <dt className="text-muted-foreground">{t("cli.axis.executable")}</dt>
                <dd>{t(`cli.executable.${installation.executableStatus}`)}</dd>
              </div>
            </dl>

            {installation.aliasPaths.length > 0 ? (
              <p className="mt-2 text-muted-foreground">
                {t("cli.diagnostics.aliases", { count: installation.aliasPaths.length })}
              </p>
            ) : null}
            {installation.targetMissing ? (
              <p className="mt-1 ucd-status-warning rounded p-1.5">
                {t("cli.diagnostics.targetMissing")}
              </p>
            ) : null}
          </li>
        );
      })}
    </ul>
  );
}
