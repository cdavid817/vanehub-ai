import type { ActivitySeverity } from "./activity-contracts";
import type { ActivityNavigator } from "./activity-navigation";
import { decodeActivityPayload } from "./activity-payload-decoder";
import { ActivityPayloadRenderer } from "./activity-payload-renderer";

export interface SafeActivityPayloadProps {
  payload: unknown;
  eventCode: string;
  occurredAtMs: number;
  severity: ActivitySeverity;
  translate: (key: string, values?: Record<string, string | number>) => string;
  onNavigate?: ActivityNavigator;
}

export function SafeActivityPayload(props: SafeActivityPayloadProps) {
  const payload = decodeActivityPayload(props.payload);
  if (payload) {
    return (
      <ActivityPayloadRenderer
        onNavigate={props.onNavigate}
        payload={payload}
        translate={props.translate}
      />
    );
  }
  const isoTimestamp = Number.isFinite(props.occurredAtMs)
    ? new Date(props.occurredAtMs).toISOString()
    : "";
  return (
    <section
      aria-label={props.translate("systemActivity.payload.unavailable.label")}
      className="rounded-xl border border-amber-300/70 bg-amber-50/80 p-3 text-sm text-amber-950 dark:border-amber-700/70 dark:bg-amber-950/30 dark:text-amber-100"
      data-payload-schema="safe-fallback"
    >
      <p className="font-semibold">
        {props.translate("systemActivity.payload.unavailable", {
          eventCode: props.eventCode,
          severity: props.severity,
        })}
      </p>
      {isoTimestamp ? <time className="mt-1 block text-xs opacity-75" dateTime={isoTimestamp}>{isoTimestamp}</time> : null}
    </section>
  );
}
