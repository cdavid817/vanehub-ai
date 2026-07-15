import { Wrench } from "lucide-react";
import type { ToolUseBlock as ToolUseBlockType } from "../../types/chat";

function formatJson(value: unknown) {
  if (value === undefined) return "";
  return JSON.stringify(value, null, 2);
}

export function ToolUseBlock({ toolUse }: { toolUse: ToolUseBlockType[] }) {
  if (toolUse.length === 0) return null;
  return (
    <div className="mt-3 grid gap-2">
      {toolUse.map((tool) => (
        <details className="rounded-md border border-border bg-muted/60 text-xs" key={tool.id}>
          <summary className="flex cursor-pointer items-center gap-2 px-3 py-2 text-muted-foreground">
            <Wrench className="h-3.5 w-3.5" aria-hidden="true" />
            <span className="truncate">{tool.name}</span>
            <span className="ml-auto rounded border border-border px-1.5 py-0.5 font-mono">{tool.status}</span>
          </summary>
          <pre className="max-h-64 overflow-auto border-t border-border px-3 py-2 font-mono text-[11px] leading-5 text-muted-foreground">
            {formatJson({ input: tool.input, output: tool.output })}
          </pre>
        </details>
      ))}
    </div>
  );
}
