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

export interface ExecutionSpanSummary {
  spanId: string;
  parentSpanId?: string | null;
  name: string;
  status: ExecutionStatus;
  fidelity: ExecutionFidelity;
  startedAt: string;
  endedAt?: string | null;
  durationMs?: number | null;
  errorClassification?: string | null;
  attributes: Record<string, SafeAttribute>;
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
