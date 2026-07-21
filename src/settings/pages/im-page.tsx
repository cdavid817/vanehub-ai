import { MessagesSquare, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import type { ImConnectorKind, ImConnectorView, WeChatAuthorization } from "../../contracts/im";
import { normalizeDisplayPath } from "../../lib/session-path";
import { agentService } from "../../services/runtime-agent-client";
import { imService } from "../../services/runtime-im-client";
import { detectRuntimeKind } from "../../services/runtime-adapter";
import type { AgentRegistryEntry, KnownProject } from "../../types/agent";
import { ImConnectorRow } from "./im/im-connector-row";
import { validateRouting } from "./im/im-form";
import { ImRoutingSection } from "./im/im-routing-section";
import { ImWeChatAuthorization } from "./im/im-wechat-authorization";
import { PageHeader } from "./page-parts";

type PendingByKind = Partial<Record<ImConnectorKind, string>>;

export function ImPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const isWebRuntime = detectRuntimeKind() !== "tauri";
  const [connectors, setConnectors] = useState<ImConnectorView[]>([]);
  const [agents, setAgents] = useState<AgentRegistryEntry[]>([]);
  const [projects, setProjects] = useState<KnownProject[]>([]);
  const [agentId, setAgentId] = useState("");
  const [projectPath, setProjectPath] = useState("");
  const [savedRouting, setSavedRouting] = useState<{ agentId: string; projectPath: string } | null>(null);
  const [routingErrors, setRoutingErrors] = useState<{ agentId?: string; projectPath?: string }>({});
  const [pending, setPending] = useState<PendingByKind>({});
  const [routingPending, setRoutingPending] = useState(false);
  const [authorization, setAuthorization] = useState<WeChatAuthorization | null>(null);
  const [authorizationPending, setAuthorizationPending] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setError(null);
    setLoading(true);
    try {
      const [viewsResult, routingResult, agentListResult, knownProjectsResult] = await Promise.allSettled([
        imService.listConnectors(), imService.getRouting(), agentService.listAgents(), agentService.listKnownProjects(),
      ]);
      const loadErrors = [viewsResult, routingResult, agentListResult, knownProjectsResult]
        .filter((result): result is PromiseRejectedResult => result.status === "rejected")
        .map((result) => imErrorMessage(result.reason, t));
      if (viewsResult.status === "fulfilled") setConnectors(viewsResult.value);
      else setConnectors([]);
      if (routingResult.status === "fulfilled") {
        setAgentId(routingResult.value?.agentId ?? "");
        const normalizedProjectPath = routingResult.value?.projectPath ? normalizeDisplayPath(routingResult.value.projectPath) : "";
        setProjectPath(normalizedProjectPath);
        setSavedRouting(routingResult.value ? { ...routingResult.value, projectPath: normalizedProjectPath } : null);
      } else {
        setSavedRouting(null);
      }
      const agentList = agentListResult.status === "fulfilled" ? agentListResult.value : [];
      setAgents(agentList.filter((agent) => (
        agent.supportedInteractionModes.includes("cli")
        && (agent.availabilityState === "available" || isWebRuntime)
      )));
      setProjects(knownProjectsResult.status === "fulfilled"
        ? knownProjectsResult.value.map((project) => ({ ...project, path: normalizeDisplayPath(project.path) }))
        : []);
      setError(loadErrors[0] ?? null);
    } catch (reason) {
      setError(imErrorMessage(reason, t));
    } finally {
      setLoading(false);
    }
  }, [isWebRuntime, t]);

  useEffect(() => { void load(); }, [load]);
  const routingReady = useMemo(
    () => Boolean(savedRouting && savedRouting.agentId === agentId && savedRouting.projectPath === projectPath),
    [agentId, projectPath, savedRouting],
  );

  async function saveRouting() {
    const validation = validateRouting(agentId, projectPath);
    setRoutingErrors(validation);
    if (Object.keys(validation).length > 0) return;
    setRoutingPending(true);
    setError(null);
    try {
      setSavedRouting(await imService.saveRouting({ agentId, projectPath }));
      setNotice(t("im.notice.routingSaved"));
    } catch (reason) {
      setError(imErrorMessage(reason, t));
    } finally {
      setRoutingPending(false);
    }
  }

  async function browseProject() {
    const path = await agentService.selectProjectDirectory().catch((reason: unknown) => {
      setError(imErrorMessage(reason, t));
      return null;
    });
    if (path) setProjectPath(normalizeDisplayPath(path));
  }

  async function connectorAction(kind: ImConnectorKind, action: string, credentials?: Record<string, string>) {
    setPending((current) => ({ ...current, [kind]: action }));
    setError(null);
    try {
      const view = connectors.find((item) => item.descriptor.kind === kind);
      if (!view) return;
      if (action === "save") await imService.saveConnector({ kind, enabled: view.config.enabled, displayName: view.config.displayName, publicConfig: view.config.publicConfig, credentials });
      if (action === "enable" || action === "disable") await imService.setConnectorEnabled(kind, action === "enable");
      if (action === "test") await imService.testConnector(kind);
      if (action === "restart") await imService.restartConnector(kind);
      if (action === "clear") await imService.clearConnector(kind);
      setNotice(t(`im.notice.${action}`));
      setConnectors(await imService.listConnectors());
    } catch (reason) {
      setError(imErrorMessage(reason, t));
    } finally {
      setPending((current) => ({ ...current, [kind]: undefined }));
    }
  }

  async function authorizationAction(action: "begin" | "poll" | "cancel") {
    setAuthorizationPending(true);
    setError(null);
    try {
      if (action === "begin") setAuthorization(await imService.beginWeChatAuthorization());
      if (action === "poll") setAuthorization(await imService.pollWeChatAuthorization());
      if (action === "cancel") {
        await imService.cancelWeChatAuthorization();
        setAuthorization(null);
      }
      setConnectors(await imService.listConnectors());
    } catch (reason) {
      setError(imErrorMessage(reason, t));
    } finally {
      setAuthorizationPending(false);
    }
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={<Button disabled={loading} onClick={() => void load()} variant="outline"><RefreshCw aria-hidden="true" />{t("im.actions.refresh")}</Button>}
        description={t("im.description")}
        icon={MessagesSquare}
        title={t("im.title")}
      />
      {isWebRuntime ? <div className="rounded-md border p-3 text-sm ucd-status-warning">{t("im.webNotice")}</div> : null}
      {error ? <div aria-live="assertive" className="rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}
      {notice ? <div aria-live="polite" className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}
      <ImRoutingSection
        agentId={agentId} agents={agents} errors={routingErrors} onAgentChange={setAgentId} onBrowse={() => void browseProject()}
        onProjectChange={setProjectPath} onSave={() => void saveRouting()} pending={routingPending} projectPath={projectPath} projects={projects}
      />
      <div className="space-y-3">
        {loading ? <div className="ucd-panel rounded-lg p-6 text-sm text-muted-foreground">{t("im.loading")}</div> : connectors.map((view) => (
          <ImConnectorRow
            authorization={view.descriptor.kind === "weixin" ? <ImWeChatAuthorization authorization={authorization} onBegin={() => void authorizationAction("begin")} onCancel={() => void authorizationAction("cancel")} onPoll={() => void authorizationAction("poll")} pending={authorizationPending} /> : undefined}
            key={view.descriptor.kind}
            onAction={(action, credentials) => connectorAction(view.descriptor.kind, action, credentials)}
            pendingAction={pending[view.descriptor.kind] ?? null}
            routingReady={routingReady}
            searchTerm={searchTerm}
            view={view}
          />
        ))}
      </div>
    </div>
  );
}

export function imErrorMessage(reason: unknown, t: ReturnType<typeof useTranslation>["t"]): string {
  const message = reason instanceof Error ? reason.message : String(reason);
  if (message.includes("communications-repository-failed")) return t("im.errors.repositoryFailed");
  if (message.includes("communications-repository-unavailable")) return t("im.errors.repositoryUnavailable");
  return message;
}
