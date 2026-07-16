import { CheckCircle2, CircleAlert, Wrench } from "lucide-react";
import type { McpServerStatus, McpTestResult } from "../../../types/mcp";

export function McpTestResultPanel({
  result,
  status,
}: {
  result?: McpTestResult | null;
  status?: McpServerStatus | null;
}) {
  const tools = result?.tools ?? status?.tools ?? [];
  const success = result?.success ?? status?.connectionStatus === "connected";
  const error = result?.error ?? status?.error;
  const duration = result?.durationMs ?? status?.durationMs;

  if (!result && !status) return null;

  return (
    <div className={`rounded-md border p-3 text-xs ${success ? "ucd-status-success" : error ? "ucd-status-danger" : "border-border bg-muted"}`}>
      <div className="mb-2 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 font-medium">
          {success ? <CheckCircle2 className="h-4 w-4" aria-hidden="true" /> : <CircleAlert className="h-4 w-4" aria-hidden="true" />}
          {success ? "Recent test passed" : error ? "Recent test failed" : "Not tested"}
        </div>
        {duration ? <span>{duration}ms</span> : null}
      </div>
      {error ? <div className="mb-2 break-words">{error}</div> : null}
      {tools.length ? (
        <div className="grid gap-1">
          {tools.slice(0, 5).map((tool) => (
            <div className="flex items-start gap-2 rounded border border-current/20 p-2" key={tool.name}>
              <Wrench className="mt-0.5 h-3.5 w-3.5 shrink-0" aria-hidden="true" />
              <div className="min-w-0">
                <div className="truncate font-medium">{tool.name}</div>
                {tool.description ? <div className="mt-0.5 line-clamp-2 opacity-80">{tool.description}</div> : null}
              </div>
            </div>
          ))}
          {tools.length > 5 ? <div className="opacity-80">{tools.length - 5} more tools</div> : null}
        </div>
      ) : null}
    </div>
  );
}
