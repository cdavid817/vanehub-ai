import { useEffect, useState } from "react";
import { Check, Wrench, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { permissionsService } from "../../services/runtime-permissions-client";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import type { ToolUseBlock as ToolUseBlockType } from "../../types/chat";
import type { ApprovalScope, PendingApprovalEntry, RiskLevel } from "../../types/permissions";

function formatJson(value: unknown) {
  if (value === undefined) return "";
  return JSON.stringify(value, null, 2);
}

const riskTone: Record<RiskLevel, "muted" | "default" | "warning" | "danger"> = {
  L0: "muted",
  L1: "default",
  L2: "warning",
  L3: "danger",
};

const scopeOptions: ApprovalScope[] = ["once", "session", "project", "global"];

function ApprovalCard({ sessionId, callId }: { sessionId: string; callId: string }) {
  const { t } = useTranslation();
  const [pending, setPending] = useState<PendingApprovalEntry | null>(null);
  const [scope, setScope] = useState<ApprovalScope>("once");
  const [resolving, setResolving] = useState<"approve" | "deny" | null>(null);

  useEffect(() => {
    let cancelled = false;
    void permissionsService.listPendingApprovals().then((entries) => {
      if (cancelled) return;
      const match = entries.find((entry) => entry.sessionId === sessionId && entry.callId === callId);
      setPending(match ?? null);
    });
    return () => {
      cancelled = true;
    };
  }, [sessionId, callId]);

  async function resolve(approved: boolean) {
    if (!pending) return;
    setResolving(approved ? "approve" : "deny");
    try {
      await permissionsService.resolvePendingApproval(pending.id, approved, scope);
    } finally {
      setResolving(null);
    }
  }

  return (
    <div className="flex flex-col gap-2 border-t border-border px-3 py-2">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-muted-foreground">{t("chat.toolApproval.prompt")}</span>
        {pending ? <Badge tone={riskTone[pending.riskLevel]}>{t(`chat.toolApproval.riskLevel.${pending.riskLevel}`)}</Badge> : null}
      </div>
      {pending ? (
        <dl className="grid grid-cols-[auto_1fr] gap-x-2 gap-y-1 text-[11px] text-muted-foreground">
          <dt className="font-medium">{t("chat.toolApproval.agent")}</dt>
          <dd className="truncate font-mono">{pending.agentId}</dd>
          <dt className="font-medium">{t("chat.toolApproval.action")}</dt>
          <dd className="truncate font-mono">{pending.action}</dd>
          <dt className="font-medium">{t("chat.toolApproval.resource")}</dt>
          <dd className="truncate font-mono">{pending.resource}</dd>
        </dl>
      ) : null}
      <div className="flex flex-wrap items-center gap-1">
        <span className="mr-1 text-muted-foreground">{t("chat.toolApproval.rememberLabel")}</span>
        {scopeOptions.map((option) => (
          <Button
            aria-pressed={scope === option}
            key={option}
            onClick={() => setScope(option)}
            size="sm"
            variant={scope === option ? "default" : "outline"}
          >
            {t(`chat.toolApproval.scope.${option}`)}
          </Button>
        ))}
      </div>
      <div className="flex items-center gap-2">
        <Button
          className="ml-auto"
          disabled={resolving !== null}
          onClick={() => void resolve(true)}
          size="sm"
          variant="outline"
        >
          <Check className="h-3.5 w-3.5" aria-hidden="true" />
          {t("chat.toolApproval.approve")}
        </Button>
        <Button disabled={resolving !== null} onClick={() => void resolve(false)} size="sm" variant="outline">
          <X className="h-3.5 w-3.5" aria-hidden="true" />
          {t("chat.toolApproval.deny")}
        </Button>
      </div>
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
            <ApprovalCard callId={tool.id} sessionId={sessionId} />
          ) : null}
        </details>
      ))}
    </div>
  );
}
