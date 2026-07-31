import { useState } from "react";
import { Check, Wrench, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../../services/runtime-agent-client";
import { Button } from "../ui/button";
import type { ToolUseBlock as ToolUseBlockType } from "../../types/chat";

function formatJson(value: unknown) {
  if (value === undefined) return "";
  return JSON.stringify(value, null, 2);
}

function ApprovalPrompt({ sessionId, callId }: { sessionId: string; callId: string }) {
  const { t } = useTranslation();
  const [pending, setPending] = useState<"approve" | "deny" | null>(null);

  async function resolve(approved: boolean) {
    setPending(approved ? "approve" : "deny");
    try {
      await agentService.resolveToolApproval(sessionId, callId, approved);
    } finally {
      setPending(null);
    }
  }

  return (
    <div className="flex items-center gap-2 border-t border-border px-3 py-2">
      <span className="text-muted-foreground">{t("chat.toolApproval.prompt")}</span>
      <Button
        className="ml-auto"
        disabled={pending !== null}
        onClick={() => void resolve(true)}
        size="sm"
        variant="outline"
      >
        <Check className="h-3.5 w-3.5" aria-hidden="true" />
        {t("chat.toolApproval.approve")}
      </Button>
      <Button
        disabled={pending !== null}
        onClick={() => void resolve(false)}
        size="sm"
        variant="outline"
      >
        <X className="h-3.5 w-3.5" aria-hidden="true" />
        {t("chat.toolApproval.deny")}
      </Button>
    </div>
  );
}

export function ToolUseBlock({ sessionId, toolUse }: { sessionId: string; toolUse: ToolUseBlockType[] }) {
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
          {tool.status === "awaiting_approval" ? (
            <ApprovalPrompt callId={tool.id} sessionId={sessionId} />
          ) : null}
        </details>
      ))}
    </div>
  );
}
