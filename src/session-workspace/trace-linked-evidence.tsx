import { useTranslation } from "react-i18next";
import type { ExecutionLink } from "../types/execution-observability";
import type { SpanEvidence } from "./use-span-evidence";

/**
 * The sections that show what a span touched, fetched from whoever owns each record.
 *
 * Split from the drawer because these five have a shape the other four do not: every one of them
 * can be loading, can have failed, and can legitimately be empty — three states the sections built
 * from the span's own fields never have.
 */

export interface TraceSectionParts {
  Section: (props: { children: React.ReactNode; title: string }) => React.ReactElement;
  Field: (props: { label: string; value: string | null }) => React.ReactElement;
  Empty: (props: { text: string }) => React.ReactElement;
}

export function TraceLinkedEvidenceSections({
  evidence,
  files,
  parts,
  related,
}: {
  evidence: SpanEvidence;
  files: readonly ExecutionLink[];
  parts: TraceSectionParts;
  related: readonly ExecutionLink[];
}) {
  const { t } = useTranslation();
  const { Empty, Field, Section } = parts;

  return (
    <>
      <Section title={t("traces.section.logs")}>
        <Body evidence={evidence} parts={parts}>
          {evidence.logs.length ? (
            evidence.logs.map((entry) => (
              <Field key={entry.id} label={entry.level} value={entry.message} />
            ))
          ) : (
            <Empty text={t("traces.section.noLinks")} />
          )}
        </Body>
      </Section>

      <Section title={t("traces.section.commands")}>
        <Body evidence={evidence} parts={parts}>
          {evidence.commands.length ? (
            evidence.commands.map((record) => (
              <Field
                key={record.id}
                label={record.status}
                // Already redacted by the producer, and shown as such. The raw argument vector
                // never leaves the native side, so there is nothing here to redact again.
                value={record.kind === "command" ? (record.redactedDisplay ?? null) : null}
              />
            ))
          ) : (
            <Empty text={t("traces.section.noLinks")} />
          )}
        </Body>
      </Section>

      <Section title={t("traces.section.findings")}>
        <Body evidence={evidence} parts={parts}>
          {evidence.findings.length ? (
            evidence.findings.map((record) => (
              <Field
                key={record.id}
                label={record.kind === "verification" ? record.verificationName : record.kind}
                value={record.kind === "verification" ? record.outcome : null}
              />
            ))
          ) : (
            <Empty text={t("traces.section.noLinks")} />
          )}
        </Body>
      </Section>

      {/* Files has no owning service to ask yet, so it shows what the span itself points at. When
          one exists this section changes shape like the three above; until then it says what it
          knows rather than an empty list that would read as "no files were touched". */}
      <Section title={t("traces.section.files")}>
        <LinkList empty={Empty} field={Field} links={files} />
      </Section>
      <Section title={t("traces.section.related")}>
        <LinkList empty={Empty} field={Field} links={related} />
      </Section>
    </>
  );
}

function LinkList({
  empty: Empty,
  field: Field,
  links,
}: {
  empty: TraceSectionParts["Empty"];
  field: TraceSectionParts["Field"];
  links: readonly ExecutionLink[];
}) {
  const { t } = useTranslation();
  if (!links.length) {
    // "None linked" rather than "none exists". This section lists what the span points at, and a
    // span that pointed at nothing is not evidence that nothing happened.
    return <Empty text={t("traces.section.noLinks")} />;
  }
  return (
    <>
      {links.map((link, index) => (
        <Field
          key={`${link.relationship}-${link.spanId ?? link.runId}-${index}`}
          label={link.relationship}
          value={link.spanId ?? link.runId}
        />
      ))}
    </>
  );
}

/**
 * Shows a queried section, or says why it cannot.
 *
 * A failed lookup and an empty result render differently, and that is the whole point: an empty
 * section means the span linked to nothing, and a failed one means nobody knows. Drawing them the
 * same way turns "we could not look" into "there is nothing there".
 */
function Body({
  children,
  evidence,
  parts,
}: {
  children: React.ReactNode;
  evidence: SpanEvidence;
  parts: TraceSectionParts;
}) {
  const { t } = useTranslation();
  if (evidence.failed) {
    return (
      <p className="ucd-status-warning rounded border px-2 py-1 text-[11px]" role="status">
        {t("traces.section.linkedUnavailable")}
      </p>
    );
  }
  if (evidence.loading) return <parts.Empty text={t("traces.section.loadingLinked")} />;
  return <>{children}</>;
}
