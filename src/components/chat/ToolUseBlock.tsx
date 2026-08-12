import { useEffect, useId, useRef, useState } from "react";
import type { TFunction } from "i18next";
import { AlertCircle, Check, CheckCircle2, ChevronDown, CircleEllipsis, LoaderCircle, ShieldAlert, Wrench, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import { permissionsService } from "../../services/runtime-permissions-client";
import { normalizeToolUse } from "../../services/tool-use";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import type { MessageStatus, ToolUseBlock as ToolUseBlockType } from "../../types/chat";
import type { ApprovalScope, PendingApprovalEntry, RiskLevel } from "../../types/permissions";

function formatJson(value: unknown) {
  if (value === undefined) return "";
  return JSON.stringify(value, null, 2);
}

interface FailureGroup {
  occurrences: ToolUseBlockType[];
  signature: string;
}

const previewKeys = ["command", "cmd", "path", "query", "pattern", "action", "resource"] as const;

export function toolActivityPreview(input: unknown): string {
  if (!input || typeof input !== "object" || Array.isArray(input)) return "";
  const record = input as Record<string, unknown>;
  const value = previewKeys.map((key) => record[key]).find((candidate) => typeof candidate === "string");
  if (typeof value !== "string") return "";
  const normalized = value
    .replace(/((?:api[_-]?key|token|password|secret)\s*(?:=|:)\s*)\S+/gi, "$1••••")
    .replace(/\s+/g, " ")
    .trim();
  return normalized.length > 120 ? `${normalized.slice(0, 117)}…` : normalized;
}

function failureSignature(tool: ToolUseBlockType) {
  return `${tool.name}\u0000${toolActivityPreview(tool.input)}\u0000${formatJson(tool.output)}`;
}

export function groupConsecutiveFailures(toolUse: readonly ToolUseBlockType[]): FailureGroup[] {
  const groups: FailureGroup[] = [];
  let previousWasFailure = false;
  for (const tool of toolUse) {
    if (tool.status !== "failed") {
      previousWasFailure = false;
      continue;
    }
    const signature = failureSignature(tool);
    const previous = groups[groups.length - 1];
    if (previousWasFailure && previous?.signature === signature) previous.occurrences.push(tool);
    else groups.push({ occurrences: [tool], signature });
    previousWasFailure = true;
  }
  return groups;
}

const riskTone: Record<RiskLevel, "muted" | "default" | "warning" | "danger"> = {
  L0: "muted", L1: "default", L2: "warning", L3: "danger",
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
      if (!cancelled) setPending(entries.find((entry) => entry.sessionId === sessionId && entry.callId === callId) ?? null);
    });
    return () => { cancelled = true; };
  }, [sessionId, callId]);

  async function resolve(approved: boolean) {
    if (!pending) return;
    setResolving(approved ? "approve" : "deny");
    try { await permissionsService.resolvePendingApproval(pending.id, approved, scope); }
    finally { setResolving(null); }
  }

  return (
    <div className="flex flex-col gap-2 border-t border-warning/30 bg-warning/5 px-3 py-2">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-muted-foreground">{t("chat.toolApproval.prompt")}</span>
        {pending ? <Badge tone={riskTone[pending.riskLevel]}>{t(`chat.toolApproval.riskLevel.${pending.riskLevel}`)}</Badge> : null}
      </div>
      {pending ? (
        <dl className="grid grid-cols-[auto_1fr] gap-x-2 gap-y-1 text-[11px] text-muted-foreground">
          <dt className="font-medium">{t("chat.toolApproval.agent")}</dt><dd className="truncate font-mono">{pending.agentId}</dd>
          <dt className="font-medium">{t("chat.toolApproval.action")}</dt><dd className="truncate font-mono">{pending.action}</dd>
          <dt className="font-medium">{t("chat.toolApproval.resource")}</dt><dd className="truncate font-mono">{pending.resource}</dd>
        </dl>
      ) : null}
      <div className="flex flex-wrap items-center gap-1">
        <span className="mr-1 text-muted-foreground">{t("chat.toolApproval.rememberLabel")}</span>
        {scopeOptions.map((option) => (
          <Button aria-pressed={scope === option} key={option} onClick={() => setScope(option)} size="sm" variant={scope === option ? "default" : "outline"}>
            {t(`chat.toolApproval.scope.${option}`)}
          </Button>
        ))}
      </div>
      <div className="flex items-center gap-2">
        <Button className="ml-auto" disabled={resolving !== null || !pending} onClick={() => void resolve(true)} size="sm" variant="outline">
          <Check className="h-3.5 w-3.5" aria-hidden="true" />{t("chat.toolApproval.approve")}
        </Button>
        <Button disabled={resolving !== null || !pending} onClick={() => void resolve(false)} size="sm" variant="outline">
          <X className="h-3.5 w-3.5" aria-hidden="true" />{t("chat.toolApproval.deny")}
        </Button>
      </div>
    </div>
  );
}

const toolLabelKeys: Record<string, string> = {
  shell: "chat.toolActivity.tool.shell", glob: "chat.toolActivity.tool.glob",
  file: "chat.toolActivity.tool.file", read_file: "chat.toolActivity.tool.readFile",
  grep: "chat.toolActivity.tool.search", remember: "chat.toolActivity.tool.remember",
};

function toolLabel(name: string, t: TFunction) {
  const key = toolLabelKeys[name];
  return key ? t(key) : name;
}

function StatusIcon({ status }: { status: ToolUseBlockType["status"] }) {
  if (status === "awaiting_approval") return <ShieldAlert className="h-3.5 w-3.5 text-warning" aria-hidden="true" />;
  if (status === "pending" || status === "running") return <LoaderCircle className="h-3.5 w-3.5 animate-spin text-primary motion-reduce:animate-none" aria-hidden="true" />;
  if (status === "failed") return <AlertCircle className="h-3.5 w-3.5 text-destructive" aria-hidden="true" />;
  return <CheckCircle2 className="h-3.5 w-3.5 text-success" aria-hidden="true" />;
}

function ActivityRow({ occurrences = [], sessionId, tool, t }: { occurrences?: ToolUseBlockType[]; sessionId: string; tool: ToolUseBlockType; t: TFunction }) {
  const preview = toolActivityPreview(tool.input);
  const repeated = occurrences.length > 1;
  const detailValue = repeated
    ? { occurrences: occurrences.map(({ id, input, output }) => ({ id, input, output })) }
    : { input: tool.input, output: tool.output };
  return (
    <div className={cn("overflow-hidden rounded-md border bg-background/70", tool.status === "failed" && "border-destructive/40", tool.status === "awaiting_approval" && "border-warning/40")} data-tool-status={tool.status}>
      <details>
        <summary className="flex min-h-9 cursor-pointer items-center gap-2 px-3 py-1.5 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
          <StatusIcon status={tool.status} />
          <span className="shrink-0 font-medium text-foreground">{toolLabel(tool.name, t)}</span>
          {preview ? <span className="min-w-0 truncate font-mono text-[11px] text-muted-foreground">{preview}</span> : null}
          {repeated ? <Badge className="shrink-0" tone="muted">×{occurrences.length}</Badge> : null}
          <Badge className="ml-auto shrink-0" tone={tool.status === "failed" ? "danger" : tool.status === "awaiting_approval" ? "warning" : tool.status === "completed" ? "success" : "default"}>
            {t(`chat.toolActivity.status.${tool.status}`)}
          </Badge>
        </summary>
        <pre className="max-h-64 overflow-auto border-t border-border px-3 py-2 font-mono text-[11px] leading-5 text-muted-foreground">
          {formatJson(detailValue)}
        </pre>
      </details>
      {tool.status === "awaiting_approval" ? <ApprovalCard callId={tool.id} sessionId={sessionId} /> : null}
    </div>
  );
}

function CountBadge({ count, label, tone }: { count: number; label: string; tone: "default" | "success" | "warning" | "danger" | "muted" }) {
  if (count === 0) return null;
  return <Badge tone={tone}>{label}</Badge>;
}

export function ToolUseBlock({ messageFailed = false, messageStatus = "streaming", sessionId, toolUse }: { messageFailed?: boolean; messageStatus?: MessageStatus; sessionId: string; toolUse: ToolUseBlockType[] }) {
  const { t } = useTranslation();
  const contentId = useId();
  const normalized = normalizeToolUse(toolUse);
  const approval = normalized.filter((tool) => tool.status === "awaiting_approval");
  const active = normalized.filter((tool) => tool.status === "pending" || tool.status === "running");
  const failed = normalized.filter((tool) => tool.status === "failed");
  const completed = normalized.filter((tool) => tool.status === "completed");
  const terminalFailed = messageFailed || messageStatus === "failed";
  const userSelectedDisclosure = useRef(false);
  const [preferredOpen, setPreferredOpen] = useState(() => approval.length > 0 || active.length > 0 || terminalFailed || messageStatus === "streaming");
  const [failureHistoryOpen, setFailureHistoryOpen] = useState(terminalFailed);
  useEffect(() => {
    if (terminalFailed) setFailureHistoryOpen(true);
  }, [terminalFailed]);
  useEffect(() => {
    if (userSelectedDisclosure.current) return;
    if (terminalFailed || active.length > 0) setPreferredOpen(true);
    else if (messageStatus === "completed" || messageStatus === "cancelled") setPreferredOpen(false);
  }, [active.length, messageStatus, terminalFailed]);
  if (normalized.length === 0) return null;
  const actionable = [...approval, ...active];
  const failureGroups = groupConsecutiveFailures(normalized).reverse();
  const latestFailure = failureGroups[0]?.occurrences.at(-1);
  const onlyCompleted = completed.length === 1 && actionable.length === 0 && failed.length === 0 ? completed[0] : null;
  const approvalForcesOpen = approval.length > 0;
  const activityOpen = approvalForcesOpen || preferredOpen;
  const summaryTool = active.at(-1) ?? failed.at(-1) ?? completed.at(-1);

  function toggleActivity() {
    if (approvalForcesOpen) return;
    userSelectedDisclosure.current = true;
    setPreferredOpen((open) => !open);
  }

  return (
    <section aria-label={t("chat.toolActivity.title")} className="mt-3 overflow-hidden rounded-lg border border-border bg-muted/35 text-xs" data-testid="tool-activity">
      <div aria-live="polite" className={cn(activityOpen && "border-b border-border")}>
        <button
          aria-controls={contentId}
          aria-expanded={activityOpen}
          aria-label={t(activityOpen ? "chat.toolActivity.collapseActivity" : "chat.toolActivity.expandActivity")}
          className="flex min-h-10 w-full cursor-pointer flex-wrap items-center gap-1.5 px-3 py-2 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring disabled:cursor-default"
          disabled={approvalForcesOpen}
          onClick={toggleActivity}
          type="button"
        >
          <ChevronDown className={cn("h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform", activityOpen && "rotate-180")} aria-hidden="true" />
          <Wrench className="h-3.5 w-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
          <span className="font-medium text-foreground">{t("chat.toolActivity.title")}</span>
          {!activityOpen && summaryTool ? <><span className="text-border">·</span><span className="shrink-0 text-foreground">{toolLabel(summaryTool.name, t)}</span><span className="min-w-16 flex-1 truncate font-mono text-[11px] text-muted-foreground">{toolActivityPreview(summaryTool.input)}</span></> : <span className="mr-auto" />}
          <CountBadge count={approval.length} label={t("chat.toolActivity.count.approval", { count: approval.length })} tone="warning" />
          <CountBadge count={active.length} label={t("chat.toolActivity.count.active", { count: active.length })} tone="default" />
          <CountBadge count={failed.length} label={t("chat.toolActivity.count.failed", { count: failed.length })} tone="danger" />
          <CountBadge count={completed.length} label={t("chat.toolActivity.count.completed", { count: completed.length })} tone="success" />
        </button>
      </div>
      <div data-testid="tool-activity-content" hidden={!activityOpen} id={contentId}>
        {actionable.length > 0 ? <div className="grid gap-1.5 p-2">{actionable.map((tool) => <ActivityRow key={tool.id} sessionId={sessionId} t={t} tool={tool} />)}</div> : null}
        {failed.length > 0 ? (
        <details
          className={cn("border-border", actionable.length > 0 && "border-t")}
          data-testid="failed-tool-history"
          onToggle={(event) => setFailureHistoryOpen(event.currentTarget.open)}
          open={failureHistoryOpen}
        >
          <summary className="flex min-h-9 cursor-pointer items-center gap-2 px-3 py-2 text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
            <AlertCircle className="h-3.5 w-3.5 shrink-0 text-destructive" aria-hidden="true" />
            <span className="shrink-0 font-medium text-destructive">{t("chat.toolActivity.failedHistory", { count: failed.length })}</span>
            {latestFailure ? <><span className="text-border">·</span><span className="shrink-0 text-foreground">{toolLabel(latestFailure.name, t)}</span><span className="min-w-0 truncate font-mono text-[11px]">{toolActivityPreview(latestFailure.input)}</span></> : null}
            <span className="ml-auto shrink-0">{t("chat.toolActivity.showDetails")}</span>
          </summary>
          <div className="grid gap-1.5 border-t border-border p-2">
            {failureGroups.map((group) => {
              const tool = group.occurrences[group.occurrences.length - 1];
              return <ActivityRow key={tool.id} occurrences={group.occurrences} sessionId={sessionId} t={t} tool={tool} />;
            })}
          </div>
        </details>
        ) : null}
        {completed.length > 0 ? (
        <details className={cn("border-border", (actionable.length > 0 || failed.length > 0) && "border-t")} data-testid="completed-tool-history">
          <summary className="flex min-h-9 cursor-pointer items-center gap-2 px-3 py-2 text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
            <CircleEllipsis className="h-3.5 w-3.5" aria-hidden="true" />
            {onlyCompleted ? (
              <><span className="font-medium text-foreground">{toolLabel(onlyCompleted.name, t)}</span><span className="min-w-0 truncate font-mono text-[11px]">{toolActivityPreview(onlyCompleted.input)}</span></>
            ) : <span>{t("chat.toolActivity.completedHistory", { count: completed.length })}</span>}
            <span className="ml-auto">{t("chat.toolActivity.showDetails")}</span>
          </summary>
          <div className="grid gap-1.5 border-t border-border p-2">
            {completed.map((tool) => <ActivityRow key={tool.id} sessionId={sessionId} t={t} tool={tool} />)}
          </div>
        </details>
        ) : null}
      </div>
    </section>
  );
}
