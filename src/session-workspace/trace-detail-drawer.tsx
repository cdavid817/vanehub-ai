import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { formatAppDateTime } from "../i18n/format";
import type {
  ExecutionEvent,
  ExecutionLink,
  ExecutionSpanSummary,
} from "../types/execution-observability";
import { TraceStatusBadge } from "./trace-span-row";
import { TraceLinkedEvidenceSections } from "./trace-linked-evidence";
import { useSpanEvidence } from "./use-span-evidence";

/**
 * Which relationship names belong under which section.
 *
 * Named rather than pattern-matched: a relationship this build has never heard of goes to
 * "related" instead of being guessed into a section, because a log listed under Files is worse
 * than one listed under a heading that admits it does not know.
 */
const LINK_SECTIONS: Record<string, "logs" | "commands" | "files" | "findings"> = {
  log: "logs",
  "log-record": "logs",
  command: "commands",
  "command-run": "commands",
  file: "files",
  "file-change": "files",
  finding: "findings",
  verification: "findings",
};

/**
 * Attribute prefixes that describe usage rather than the work itself.
 *
 * Usage gets its own section because it is the one group a reader scans for a number rather than
 * reading — tokens and cost, next to each other, without the twenty other attributes in between.
 */
const USAGE_PREFIXES = ["gen_ai.usage.", "vanehub.usage.", "gen_ai.response.model"];

export function TraceDetailDrawer({
  events,
  onClose,
  runId,
  service,
  sessionId,
  span,
  traceId,
}: {
  events: readonly ExecutionEvent[];
  onClose: () => void;
  runId: string;
  service?: Parameters<typeof useSpanEvidence>[0]["service"];
  sessionId: string | null;
  span: ExecutionSpanSummary;
  traceId: string;
}) {
  const { i18n, t } = useTranslation();
  // Fetched from whoever owns each record rather than embedded in the trace payload: log text and
  // command output are exactly what redaction exists for, and a trace DTO is one of the places it
  // has no second chance to run.
  const evidence = useSpanEvidence({
    enabled: true,
    runId,
    service,
    sessionId,
    spanId: span.spanId,
    traceId,
  });
  const spanEvents = events.filter((event) => event.spanId === span.spanId);
  const attributes = Object.entries(span.attributes);
  const usage = attributes.filter(([key]) => USAGE_PREFIXES.some((prefix) => key.startsWith(prefix)));
  const rest = attributes.filter(([key]) => !USAGE_PREFIXES.some((prefix) => key.startsWith(prefix)));
  const grouped = groupLinks(span.links);

  return (
    <aside
      aria-label={t("traces.detail")}
      // A dialog on narrow viewports, a panel on wide ones. The role follows the layout because
      // that is what decides whether the reader can still see the list behind it — announcing a
      // dialog over a visible list would tell them they are trapped when they are not.
      className="ucd-panel flex min-h-0 flex-col gap-3 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3 max-lg:absolute max-lg:inset-0 max-lg:z-10"
      role="region"
      tabIndex={-1}
    >
      <header className="flex items-start justify-between gap-2 border-b border-border pb-2">
        <div className="min-w-0">
          <h3 className="wrap-break-word font-mono text-sm font-medium">{span.name}</h3>
          <div className="mt-1 flex flex-wrap items-center gap-1.5">
            <TraceStatusBadge status={span.status} />
            <span className="text-[11px] text-muted-foreground">
              {t(`traces.fidelity.${span.fidelity}`)}
            </span>
          </div>
        </div>
        <button
          aria-label={t("traces.closeDetail")}
          className="flex h-7 w-7 shrink-0 items-center justify-center rounded border border-border hover:bg-muted"
          onClick={onClose}
          type="button"
        >
          <X className="h-3.5 w-3.5" aria-hidden="true" />
        </button>
      </header>

      <Section title={t("traces.section.overview")}>
        <Field label={t("traces.spanId")} value={span.spanId} />
        <Field label={t("traces.parentSpanId")} value={span.parentSpanId ?? null} />
        <Field
          label={t("traces.startedAt")}
          value={formatAppDateTime(span.startedAt, i18n.language, {
            dateStyle: "short",
            timeStyle: "medium",
          })}
        />
        <Field
          label={t("traces.durationLabel")}
          // Absent rather than zero while it runs: those two mean opposite things about whether
          // the work is done.
          value={span.completedDurationMs === undefined
            ? t("traces.stillRunning")
            : t("traces.duration", { duration: span.completedDurationMs })}
        />
        {span.attempt === undefined ? null : (
          <Field label={t("traces.attemptLabel")} value={String(span.attempt)} />
        )}
        {span.criticalPath ? <Field label={t("traces.criticalPath")} value="—" /> : null}
      </Section>

      {span.errorClassification ? (
        <Section title={t("traces.section.error")}>
          {/* A stable classification code, never a message. A message would be the one place in
              this panel where unredacted producer text could appear. */}
          <p className="ucd-status-warning rounded border px-2 py-1 font-mono text-[11px]">
            {span.errorClassification}
          </p>
        </Section>
      ) : null}

      {usage.length ? (
        <Section title={t("traces.section.usage")}>
          {usage.map(([key, value]) => (
            <Field key={key} label={key} value={String(value)} />
          ))}
        </Section>
      ) : null}

      <Section title={t("traces.section.attributes")}>
        {rest.length ? (
          rest.map(([key, value]) => <Field key={key} label={key} value={String(value)} />)
        ) : (
          <Empty text={t("traces.section.noAttributes")} />
        )}
      </Section>

      <Section title={t("traces.section.events")}>
        {spanEvents.length ? (
          spanEvents.map((event) => (
            <div className="rounded border border-border p-2 text-[11px]" key={event.sequence}>
              <div className="flex items-center justify-between gap-2">
                <span className="font-mono">{event.name}</span>
                <time className="text-muted-foreground">
                  {formatAppDateTime(event.timestamp, i18n.language, {
                    dateStyle: "short",
                    timeStyle: "medium",
                  })}
                </time>
              </div>
            </div>
          ))
        ) : (
          <Empty text={t("traces.section.noEvents")} />
        )}
      </Section>

      <TraceLinkedEvidenceSections
        evidence={evidence}
        files={grouped.files}
        parts={{ Empty, Field, Section }}
        related={grouped.related}
      />
    </aside>
  );
}

function groupLinks(links: readonly ExecutionLink[]) {
  const grouped = {
    logs: [] as ExecutionLink[],
    commands: [] as ExecutionLink[],
    files: [] as ExecutionLink[],
    findings: [] as ExecutionLink[],
    related: [] as ExecutionLink[],
  };
  for (const link of links) {
    grouped[LINK_SECTIONS[link.relationship] ?? "related"].push(link);
  }
  return grouped;
}

function Section({ children, title }: { children: React.ReactNode; title: string }) {
  return (
    <section>
      <h4 className="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
        {title}
      </h4>
      <div className="grid gap-1">{children}</div>
    </section>
  );
}

function Field({ label, value }: { label: string; value: string | null }) {
  return (
    <div className="grid grid-cols-[minmax(6rem,40%)_minmax(0,1fr)] gap-2 text-[11px]">
      <span className="truncate text-muted-foreground" title={label}>{label}</span>
      <span className="truncate font-mono" title={value ?? "—"}>{value ?? "—"}</span>
    </div>
  );
}

function Empty({ text }: { text: string }) {
  return <p className="text-[11px] text-muted-foreground">{text}</p>;
}
