import { ArrowUpRight } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { EvidenceCoverageState, ReportSectionCoverage } from "../types/session-workspace-evidence";

/**
 * One report section, with its own coverage and its own way out.
 *
 * Coverage is per section rather than per report because a report is useful while one of its
 * sources is still indexing. A single banner at the top would either hide that or discredit the
 * eight sections that are fine.
 */
export function ReportSection({
  children,
  coverage,
  onOpenEvidence,
  title,
}: {
  children: ReactNode;
  coverage: ReportSectionCoverage;
  /** Absent when this build cannot build a destination for the section. */
  onOpenEvidence?: () => void;
  title: string;
}) {
  const { t } = useTranslation();
  return (
    <section className="rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
      <header className="mb-3 flex flex-wrap items-center gap-2">
        <h3 className="text-sm font-semibold">{title}</h3>
        <CoverageBadge coverage={coverage} />
        {onOpenEvidence ? (
          <button
            className="ml-auto flex h-6 items-center gap-1 rounded-full border border-border bg-background px-2 text-[11px] text-muted-foreground hover:bg-muted"
            onClick={onOpenEvidence}
            type="button"
          >
            {t("sessionTabs.report.openEvidence")}
            <ArrowUpRight aria-hidden="true" className="h-3 w-3" />
          </button>
        ) : null}
      </header>
      {children}
    </section>
  );
}

/**
 * Rendered for every state including `complete`.
 *
 * The other sections' badges are the reason: a badge that appeared only on degraded sections would
 * make its absence ambiguous between "this one is complete" and "this build forgot to render one".
 */
export function CoverageBadge({ coverage }: { coverage: ReportSectionCoverage }) {
  const { t } = useTranslation();
  const state: EvidenceCoverageState = coverage.state;
  return (
    <span
      className={cn(
        "rounded-full border px-2 py-0.5 text-[11px]",
        state === "complete"
          ? "border-border bg-background text-muted-foreground"
          : state === "indexing"
            ? "border-border bg-muted text-muted-foreground"
            : "ucd-status-warning",
      )}
      role="status"
    >
      {t(`evidence.coverage.${state}`)}
      {coverage.reasonCodes.length > 0 ? ` · ${coverage.reasonCodes.join(", ")}` : null}
    </span>
  );
}

/**
 * A figure, or the fact that there is none.
 *
 * `undefined` renders as an em dash rather than as zero. The whole backend is built so that a
 * measurement nobody took arrives absent, and formatting it as `0` here would undo that at the last
 * step — in the one place a reader actually looks.
 */
export function ReportMetric({
  label,
  value,
}: {
  label: string;
  value: number | undefined;
}) {
  const { i18n, t } = useTranslation();
  const formatted =
    value === undefined ? "—" : new Intl.NumberFormat(i18n.language).format(value);
  return (
    <div className="rounded-lg border border-border bg-background p-3">
      <p className="text-xs text-muted-foreground">{label}</p>
      <strong className="mt-1 block text-xl text-primary" title={value === undefined ? t("sessionTabs.report.notMeasured") : undefined}>
        {formatted}
      </strong>
    </div>
  );
}

/** Empty because nothing happened, said in words rather than by showing an empty box. */
export function ReportEmptyRow({ message }: { message: string }) {
  return <p className="text-sm text-muted-foreground">{message}</p>;
}
