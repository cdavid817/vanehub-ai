import { Stethoscope } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";

/**
 * What each probe concluded, and what it could not conclude.
 *
 * `unknown` is shown as `unknown`, never as a failure. "No documented non-interactive command
 * exists for this tool" and "the command failed" are different facts, and reporting the first as
 * the second is what made a working CLI look broken.
 */
const PROBES: Array<[labelKey: string, field: keyof CliEnvironmentSnapshot, prefix: string]> = [
  ["cli.probe.version", "executable", "cli.executable"],
  ["cli.probe.doctor", "readiness", "cli.readiness"],
  ["cli.probe.authentication", "authentication", "cli.authentication"],
  ["cli.probe.compatibility", "compatibility", "cli.compatibility"],
];

// Spelled exactly as the backend's `as_str` emits them. A value spelled differently here still
// compiles and still renders -- it just silently sits in the neutral tone forever.
const GOOD = ["healthy", "authenticated", "supported", "ready"];
const BAD = ["broken", "unsupported-version", "unsupported-platform", "unsupported-architecture"];
const WARN = ["timeout", "permission-denied", "required", "expired", "misconfigured", "needs-auth", "missing-dependency"];

function toneOf(value: string): "success" | "warning" | "danger" | "muted" {
  if (GOOD.includes(value)) return "success";
  if (BAD.includes(value)) return "danger";
  if (WARN.includes(value)) return "warning";
  return "muted";
}

export function CliDiagnosticsTab({
  snapshot,
  running,
  onRerun,
}: {
  snapshot: CliEnvironmentSnapshot;
  running: boolean;
  onRerun: () => void;
}) {
  const { t } = useTranslation();

  return (
    <div className="space-y-4 text-sm">
      <dl className="grid gap-3 sm:grid-cols-2">
        {PROBES.map(([labelKey, field, prefix]) => {
          const value = String(snapshot[field]);
          return (
            <div key={field}>
              <dt className="text-xs text-muted-foreground">{t(labelKey)}</dt>
              <dd className="mt-0.5">
                <Badge tone={toneOf(value)}>{t(`${prefix}.${value}`)}</Badge>
              </dd>
            </div>
          );
        })}
      </dl>

      {snapshot.conflicts.length > 0 ? (
        <section>
          <h4 className="text-xs font-semibold text-muted-foreground">{t("cli.diagnostics.conflicts")}</h4>
          <ul className="mt-2 grid gap-2">
            {snapshot.conflicts.map((conflict) => (
              <li className="rounded-md border p-2 text-xs ucd-status-warning" key={conflict.kind}>
                <div className="flex items-center gap-2">
                  <Badge tone={conflict.blocksMutation ? "warning" : "muted"}>
                    {t(`cli.severity.${conflict.severity}`)}
                  </Badge>
                  {conflict.blocksLaunch ? (
                    <Badge tone="danger">{t("cli.conflict.blocksLaunch")}</Badge>
                  ) : null}
                </div>
                <p className="mt-1">{t(`cli.conflict.${conflict.reasonCode}`)}</p>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <section>
        <h4 className="text-xs font-semibold text-muted-foreground">{t("cli.diagnostics.sources")}</h4>
        <ul className="mt-2 grid gap-2">
          {snapshot.sources.map((source) => (
            <li className="rounded-md border border-border p-2 text-xs" key={source.sourceId}>
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">{t(`cli.source.${source.sourceId}`)}</span>
                <Badge tone={source.management === "managed" ? "muted" : "warning"}>
                  {t(`cli.management.${source.management}`)}
                </Badge>
                {!source.supportedOnThisPlatform ? (
                  <Badge tone="muted">{t("cli.source.unsupportedHere")}</Badge>
                ) : null}
                <span className="ml-auto text-muted-foreground">
                  {source.availableVersionCount === null
                    ? t("cli.source.noCatalog")
                    : t("cli.source.catalogSize", { count: source.availableVersionCount })}
                </span>
              </div>
              {source.guidanceCode ? (
                <p className="mt-1 text-muted-foreground">{t(source.guidanceCode)}</p>
              ) : null}
            </li>
          ))}
        </ul>
      </section>

      <Button disabled={running} variant="outline" onClick={onRerun}>
        <Stethoscope aria-hidden="true" className={running ? "h-4 w-4 animate-spin" : "h-4 w-4"} />
        {t("cli.diagnostics.rerun")}
      </Button>
    </div>
  );
}
