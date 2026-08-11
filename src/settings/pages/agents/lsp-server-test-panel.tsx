import { useMutation } from "@tanstack/react-query";
import { LoaderCircle, TestTube2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import {
  lspLanguageIds,
  type LspLanguageId,
  type LspServerTestResult,
} from "../../../types/lsp";
import { SectionPanel } from "../page-parts";

function ServerTestResult({ result }: { result: LspServerTestResult }) {
  const { t } = useTranslation();
  const succeeded = result.phases.every((phase) => phase.status === "succeeded");

  return (
    <div
      className={`mt-4 rounded-md border p-3 text-sm ${succeeded ? "ucd-status-success" : "ucd-status-danger"}`}
      role={succeeded ? "status" : "alert"}
    >
      <p className="font-medium">
        {t(succeeded ? "lspSettings.test.succeeded" : "lspSettings.test.failed")}
      </p>
      <code className="mt-1 block text-xs opacity-80">{result.server}</code>
      <ol className="mt-3 space-y-2">
        {result.phases.map((phase) => (
          <li className="flex flex-wrap items-center justify-between gap-2" key={phase.phase}>
            <span>{t(`lspSettings.test.phase.${phase.phase}`)}</span>
            <span className="flex items-center gap-2">
              {phase.reasonCode ? <span className="text-xs">{t(`lspSettings.reason.${phase.reasonCode}`)}</span> : null}
              <Badge tone={phase.status === "succeeded" ? "success" : phase.status === "failed" ? "danger" : "muted"}>
                {t(`lspSettings.test.status.${phase.status}`)}
              </Badge>
            </span>
          </li>
        ))}
      </ol>
    </div>
  );
}

function ServerTestCard({ language, service }: { language: LspLanguageId; service: AgentService }) {
  const { t } = useTranslation();
  const languageName = t(`lspSettings.language.${language}`);
  const mutation = useMutation({ mutationFn: () => service.testLspServer(language) });
  const failedResult = mutation.data?.phases.some((phase) => phase.status === "failed") ?? false;
  const canRetry = mutation.isError || failedResult;
  const action = t(canRetry ? "lspSettings.retry" : "lspSettings.test.run");

  return (
    <article className="rounded-lg border border-border bg-muted/15 p-4" aria-busy={mutation.isPending}>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h4 className="text-sm font-semibold">{languageName}</h4>
        <Button
          aria-label={`${action} ${languageName}`}
          disabled={mutation.isPending}
          onClick={() => mutation.mutate()}
          size="sm"
          type="button"
          variant="outline"
        >
          {mutation.isPending ? <LoaderCircle className="animate-spin" aria-hidden="true" /> : <TestTube2 aria-hidden="true" />}
          {mutation.isPending ? t("lspSettings.test.running") : action}
        </Button>
      </div>
      {mutation.isError && !mutation.isPending ? (
        <p className="mt-4 rounded-md border p-3 text-sm ucd-status-danger" role="alert">
          {t("lspSettings.test.failed")}
        </p>
      ) : null}
      {mutation.data && !mutation.isPending ? <ServerTestResult result={mutation.data} /> : null}
    </article>
  );
}

export function LspServerTestPanel({
  service = defaultAgentService,
}: {
  service?: AgentService;
}) {
  const { t } = useTranslation();

  return (
    <SectionPanel
      description={t("lspSettings.test.description")}
      icon={TestTube2}
      title={t("lspSettings.test.title")}
      variant="settings"
    >
      <div className="grid gap-4 p-5 sm:p-6 xl:grid-cols-2">
        {lspLanguageIds.map((language) => (
          <ServerTestCard key={language} language={language} service={service} />
        ))}
      </div>
    </SectionPanel>
  );
}
