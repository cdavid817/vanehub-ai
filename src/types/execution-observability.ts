export type CapturePolicy = "metadata_only" | "redacted_content";
export type OtlpProtocol = "http_protobuf";

export interface ObservabilitySettings {
  localTimelineEnabled: boolean;
  otlpEnabled: boolean;
  otlpEndpoint?: string | null;
  otlpProtocol: OtlpProtocol;
  samplingRatio: number;
  retentionDays: number;
  capturePolicy: CapturePolicy;
  mcpRelayEnabled: boolean;
  otlpAuthConfigured: boolean;
  /** Write-only. Native responses always omit this value. Empty clears the stored credential. */
  otlpAuthToken?: string | null;
}

export type ExecutionStatus =
  | "accepted"
  | "running"
  | "succeeded"
  | "failed"
  | "cancelled"
  | "incomplete";
export type ExecutionFidelity = "native" | "proxied" | "inferred" | "opaque";
export type ExecutionSource = "desktop" | "instant_message" | "scheduled";
export type SafeAttribute = boolean | number | string;

export interface ExecutionRunSummary {
  runId: string;
  traceId: string;
  rootSpanId: string;
  source: ExecutionSource;
  sourceId?: string | null;
  status: ExecutionStatus;
  startedAt: string;
  endedAt?: string | null;
  durationMs?: number | null;
  sessionId?: string | null;
  operationId?: string | null;
  agentId?: string | null;
}

/**
 * What a span is, decided on the native side from what its producer asserted.
 *
 * Sent rather than inferred here. The view has only the name to go on, and a name is a label
 * somebody chose: classifying by substring reads `chat.completion.tool_choice` as a tool call and
 * `list_resources` as nothing at all, and neither mistake is visible from the screen.
 *
 * `unknown` is a real answer — it says the producer did not declare a kind and carried no
 * conventional attribute that implies one.
 */
export type ExecutionSpanKind =
  | "model"
  | "tool"
  | "mcp"
  | "process"
  | "delegation"
  | "file"
  | "network"
  | "container"
  | "unknown";

/** One relationship between spans or runs. Identifiers only, never what they point at. */
export interface ExecutionLink {
  runId: string;
  traceId: string;
  spanId?: string | null;
  relationship: string;
}

export interface ExecutionSpanSummary {
  spanId: string;
  parentSpanId?: string | null;
  name: string;
  kind: ExecutionSpanKind;
  status: ExecutionStatus;
  fidelity: ExecutionFidelity;
  startedAt: string;
  endedAt?: string | null;
  durationMs?: number | null;
  errorClassification?: string | null;
  attributes: Record<string, SafeAttribute>;
  /** Distance from a root span. Zero for a root. */
  depth: number;
  /**
   * Milliseconds from the run's start to this span's start.
   *
   * Absent when either timestamp could not be read. A bar placed at zero because a timestamp
   * failed to parse would put work at the beginning of the run that did not happen there.
   */
  startOffsetMs?: number;
  /**
   * Duration of a span that finished. Absent while it is still running — elapsed-so-far would make
   * a running span indistinguishable from one that finished in exactly that time.
   */
  completedDurationMs?: number;
  /** Which attempt this was, when a producer counted. Absent when nobody did. */
  attempt?: number;
  delegated: boolean;
  /**
   * Whether this span is on the chain that determined the run's duration.
   *
   * Only ever true once every span in the run has finished: a critical path through work that is
   * still running is a prediction, and this field reports an observation.
   */
  criticalPath: boolean;
  links: ExecutionLink[];
}

export interface ExecutionEvent {
  sequence: number;
  spanId: string;
  name: string;
  timestamp: string;
  attributes: Record<string, SafeAttribute>;
}

export interface ExecutionTimeline {
  run: ExecutionRunSummary;
  spans: ExecutionSpanSummary[];
  events: ExecutionEvent[];
}

export interface PageRequest {
  limit: number;
  pageToken?: string | null;
}

export interface ExecutionRunPage {
  items: ExecutionRunSummary[];
  nextPageToken?: string | null;
}

export type McpTransport = "stdio" | "http";

export interface ExecutionObservationCapability {
  agentId: string;
  transport: McpTransport;
  toolFidelity: ExecutionFidelity;
  mcpFidelity: ExecutionFidelity;
  relaySupported: boolean;
  detail: string;
}

export type ObservabilityErrorCode =
  | "invalid_settings"
  | "invalid_page_token"
  | "run_not_found"
  | "storage_unavailable"
  | "exporter_unavailable";

export interface ObservabilityCommandError {
  code: ObservabilityErrorCode;
  message: string;
  field?: string | null;
}
