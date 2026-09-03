import { useQuery } from "@tanstack/react-query";
import { Eye } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { EffectivePreview, SessionPersonalizationMode } from "../../../types/personalization";
import { SectionPanel } from "../page-parts";
import { useScopeOptions } from "./use-scope-options";

const MODES: SessionPersonalizationMode[] = ["standard", "project-only", "temporary"];

/**
 * The id a hypothetical resolution is made under.
 *
 * Used when no real session is selected. It identifies the snapshot rather than selecting anything,
 * which is what lets the mode and workspace below be the user's choice rather than some session's.
 */
const PREVIEW_SESSION_ID = "personalization-runtime-preview";

export function RuntimePreviewSection({ service = defaultAgentService }: { service?: AgentService }) {
  const { t } = useTranslation();
  const { agents, workspaces } = useScopeOptions(service);
  const [agentId, setAgentId] = useState("");
  const [workspaceKey, setWorkspaceKey] = useState("");
  const [mode, setMode] = useState<SessionPersonalizationMode>("standard");
  const [sessionId, setSessionId] = useState("");

  const sessionsQuery = useQuery({
    queryKey: ["personalization", "preview-sessions"] as const,
    queryFn: () => service.listSessions(),
  });
  const sessions = sessionsQuery.data ?? [];
  const selectedSession = sessions.find((session) => session.id === sessionId) ?? null;

  // A real session's own mode and workspace are what it is actually running under, so choosing one
  // answers "what is this session using" rather than "what would a session like this use". The two
  // controls go read-only rather than hidden: the values are the answer, and a control that
  // vanished would leave the user unsure whether the preview had used them at all.
  const sessionWorkspaceQuery = useQuery({
    queryKey: ["personalization", "preview-session-workspace", selectedSession?.id ?? null] as const,
    queryFn: () =>
      service.resolvePersonalizationWorkspace({
        projectPath: selectedSession?.projectPath ?? undefined,
        worktreePath: selectedSession?.worktreePath ?? undefined,
      }),
    enabled: selectedSession !== null,
  });

  const effectiveMode = selectedSession?.personalizationMode ?? mode;
  const effectiveWorkspaceKey = selectedSession
    ? (sessionWorkspaceQuery.data?.workspaceKey ?? undefined)
    : workspaceKey || undefined;
  const resolvingSession = selectedSession !== null && sessionWorkspaceQuery.isPending;

  const previewQuery = useQuery({
    queryKey: [
      "personalization",
      "runtime-preview",
      { agentId, workspaceKey: effectiveWorkspaceKey ?? "", mode: effectiveMode, sessionId },
    ] as const,
    queryFn: () =>
      service.previewEffectivePersonalization({
        agentId,
        sessionId: selectedSession?.id ?? PREVIEW_SESSION_ID,
        workspaceKey: effectiveWorkspaceKey,
        sessionMode: effectiveMode,
      }),
    enabled: agentId !== "" && !resolvingSession,
  });

  return (
    <SectionPanel
      description={t("personalization.runtimePreview.description")}
      icon={Eye}
      title={t("personalization.runtimePreview.title")}
    >
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4" data-testid="personalization-preview-inputs">
        <label className="flex min-w-0 flex-col gap-1 text-xs font-medium">
          {t("personalization.scope.agent")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid="personalization-preview-agent"
            onChange={(event) => setAgentId(event.target.value)}
            value={agentId}
          >
            <option value="">{t("personalization.scope.chooseAgent")}</option>
            {agents.map((agent) => (
              <option key={agent.agentId} value={agent.agentId}>
                {agent.displayName}
              </option>
            ))}
          </select>
        </label>
        <label className="flex min-w-0 flex-col gap-1 text-xs font-medium">
          {t("personalization.scope.workspace")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid="personalization-preview-workspace"
            disabled={selectedSession !== null}
            onChange={(event) => setWorkspaceKey(event.target.value)}
            value={selectedSession ? (effectiveWorkspaceKey ?? "") : workspaceKey}
          >
            <option value="">{t("personalization.preview.noWorkspace")}</option>
            {workspaces.map((workspace) => (
              <option key={workspace.workspaceKey} value={workspace.workspaceKey}>
                {workspace.displayName}
              </option>
            ))}
          </select>
        </label>
        <label className="flex min-w-0 flex-col gap-1 text-xs font-medium">
          {t("personalization.preview.session")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid="personalization-preview-session"
            onChange={(event) => setSessionId(event.target.value)}
            value={sessionId}
          >
            <option value="">{t("personalization.preview.hypotheticalSession")}</option>
            {sessions.map((session) => (
              <option key={session.id} value={session.id}>
                {session.title}
              </option>
            ))}
          </select>
        </label>
        <label className="flex min-w-0 flex-col gap-1 text-xs font-medium">
          {t("personalization.preview.mode")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid="personalization-preview-mode"
            disabled={selectedSession !== null}
            onChange={(event) => setMode(event.target.value as SessionPersonalizationMode)}
            value={effectiveMode}
          >
            {MODES.map((value) => (
              <option key={value} value={value}>
                {t(`personalization.preview.modeValue.${value}`)}
              </option>
            ))}
          </select>
        </label>
      </div>

      {selectedSession ? (
        <p className="mt-3 text-xs text-muted-foreground" data-testid="personalization-preview-session-note">
          {t("personalization.preview.sessionFixes")}
        </p>
      ) : null}

      <div className="mt-4">
        {agentId === "" ? (
          <p className="text-sm text-muted-foreground" data-testid="personalization-runtime-preview-empty">
            {t("personalization.runtimePreview.empty")}
          </p>
        ) : previewQuery.error ? (
          <p className="text-sm ucd-status-danger" data-testid="personalization-preview-error" role="alert">
            {t("personalization.preview.failed")}
          </p>
        ) : previewQuery.isPending || !previewQuery.data ? (
          <p className="text-sm text-muted-foreground">{t("personalization.memory.loading")}</p>
        ) : (
          <PreviewOutput preview={previewQuery.data} />
        )}
      </div>

      <p className="mt-4 text-xs text-muted-foreground" data-testid="personalization-preview-cli-compaction">
        {t("personalization.runtimePreview.cliCompaction")}
      </p>
    </SectionPanel>
  );
}

function PreviewOutput({ preview }: { preview: EffectivePreview }) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-4" data-testid="personalization-preview-output">
      {preview.warnings.length > 0 ? (
        <ul className="text-xs ucd-status-warning" data-testid="personalization-preview-warnings" role="alert">
          {preview.warnings.map((warning) => (
            <li key={warning}>{t(`personalization.warning.${warning}`)}</li>
          ))}
        </ul>
      ) : null}

      <section>
        <h4 className="text-sm font-semibold">{t("personalization.preview.applied")}</h4>
        {preview.includedInstructions.length === 0 ? (
          <p className="text-sm text-muted-foreground" data-testid="personalization-preview-no-instructions">
            {t("personalization.preview.noInstructions")}
          </p>
        ) : (
          <ul className="mt-2 grid gap-2" data-testid="personalization-preview-included">
            {preview.includedInstructions.map((segment) => (
              <li className="rounded-md border border-border/70 p-2" key={`${segment.field}:${segment.scopeKind}:${segment.scopeKey}`}>
                <div className="flex flex-wrap items-baseline gap-2 text-xs text-muted-foreground">
                  <Badge tone="muted">{t(`personalization.preview.field.${segment.field}`)}</Badge>
                  <span>
                    {segment.scopeKey
                      ? `${t(`personalization.overview.source.${segment.scopeKind}`)} (${segment.scopeKey})`
                      : t(`personalization.overview.source.${segment.scopeKind}`)}
                  </span>
                  <span>{t("personalization.inheritance.revision", { revision: segment.policyRevision })}</span>
                  <span>{t(`personalization.preview.mergeAction.${segment.mergeAction}`)}</span>
                  <span>{t("personalization.overview.characters", { count: segment.characters })}</span>
                </div>
                {/* Redacted by the native side through the same rule the logs use. A settings
                    screen gets screenshotted into issues, so a token a user pasted into their own
                    instructions is not handed back here. */}
                <p className="mt-1 wrap-break-word whitespace-pre-wrap text-sm">{segment.redactedText}</p>
              </li>
            ))}
          </ul>
        )}
      </section>

      {preview.excludedInstructions.length > 0 ? (
        <section>
          <h4 className="text-sm font-semibold">{t("personalization.preview.excludedInstructions")}</h4>
          <ul className="mt-1 list-disc pl-5 text-xs text-muted-foreground" data-testid="personalization-preview-excluded">
            {preview.excludedInstructions.map((segment) => (
              <li key={`${segment.field}:${segment.scopeKind}:${segment.scopeKey}:${segment.reason}`}>
                {t(`personalization.preview.field.${segment.field}`)} —{" "}
                {t(`personalization.preview.exclusion.${segment.reason}`)}
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <section>
        <h4 className="text-sm font-semibold">{t("personalization.preview.memory")}</h4>
        <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          <Badge tone={preview.memoryRead ? "success" : "muted"} data-testid="personalization-preview-delivery">
            {t(`personalization.overview.delivery.${preview.memoryDelivery}`)}
          </Badge>
          <span data-testid="personalization-preview-counts">
            {t("personalization.preview.eligible", {
              eligible: preview.eligibleMemoryCount,
              considered: preview.consideredMemoryCount,
            })}
          </span>
          <span>{t("personalization.preview.tokens", { count: preview.approximateTokens })}</span>
        </div>
        {preview.memoryExclusions.length > 0 ? (
          <ul className="mt-2 list-disc pl-5 text-xs text-muted-foreground" data-testid="personalization-preview-memory-exclusions">
            {preview.memoryExclusions.map((exclusion) => (
              <li key={exclusion.reason}>
                {t(`personalization.preview.memoryExclusion.${exclusion.reason}`)} — {exclusion.count}
              </li>
            ))}
          </ul>
        ) : null}
      </section>
    </div>
  );
}
