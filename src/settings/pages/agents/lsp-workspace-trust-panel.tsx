import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { LoaderCircle, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { LspWorkspaceTrustUpdate } from "../../../types/lsp";
import { SectionPanel } from "../page-parts";
import { lspServerStatusQueryKey } from "./lsp-configuration-section";

export const lspWorkspaceTrustQueryKey = ["agents", "lsp-workspace-trust"] as const;

export function LspWorkspaceTrustPanel({
  service = defaultAgentService,
}: {
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [canonicalRoot, setCanonicalRoot] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const trustQuery = useQuery({
    queryKey: lspWorkspaceTrustQueryKey,
    queryFn: () => service.listLspWorkspaceTrust(),
  });
  const updateMutation = useMutation({
    mutationFn: (update: LspWorkspaceTrustUpdate) => service.updateLspWorkspaceTrust(update),
    onMutate: () => setNotice(null),
    onSuccess: async (record) => {
      if (record.trusted) setCanonicalRoot("");
      setNotice(t(record.trusted ? "lspSettings.trust.trusted" : "lspSettings.trust.untrusted"));
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: lspWorkspaceTrustQueryKey }),
        queryClient.invalidateQueries({ queryKey: lspServerStatusQueryKey }),
      ]);
    },
  });
  const trustedRecords = trustQuery.data?.filter((record) => record.trusted) ?? [];
  const pending = updateMutation.isPending;

  function updateTrust(update: LspWorkspaceTrustUpdate): void {
    updateMutation.mutate(update);
  }

  return (
    <SectionPanel
      description={t("lspSettings.trust.description")}
      icon={ShieldCheck}
      title={t("lspSettings.trust.title")}
      variant="settings"
    >
      <div className="space-y-4 p-5 sm:p-6">
        <div className="rounded-lg border border-amber-500/30 bg-amber-500/10 p-4 text-sm leading-6">
          <p className="font-medium">{t("lspSettings.trust.explanation")}</p>
          <p className="mt-2 text-xs leading-5 text-muted-foreground" id="lsp-trust-boundary">
            {t("lspSettings.trust.notSandboxed")}
          </p>
        </div>

        <form
          className="flex flex-col gap-3 sm:flex-row sm:items-end"
          onSubmit={(event) => {
            event.preventDefault();
            const root = canonicalRoot.trim();
            if (root) updateTrust({ canonicalRoot: root, trusted: true });
          }}
        >
          <label className="min-w-0 flex-1 text-sm font-medium" htmlFor="lsp-workspace-root">
            {t("lspSettings.trust.rootPlaceholder")}
            <input
              aria-describedby="lsp-trust-boundary"
              className="mt-1 min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-xs focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60"
              disabled={pending}
              id="lsp-workspace-root"
              maxLength={4096}
              onChange={(event) => setCanonicalRoot(event.target.value)}
              placeholder={t("lspSettings.trust.rootPlaceholder")}
              spellCheck={false}
              value={canonicalRoot}
            />
          </label>
          <Button disabled={pending || canonicalRoot.trim().length === 0} type="submit">
            {pending ? t("lspSettings.trust.updating") : t("lspSettings.trust.grant")}
          </Button>
        </form>

        {trustQuery.isLoading ? (
          <div className="flex min-h-20 items-center justify-center gap-2 text-sm text-muted-foreground">
            <LoaderCircle className="h-4 w-4 animate-spin" aria-hidden="true" />
            {t("lspSettings.loading")}
          </div>
        ) : null}
        {!trustQuery.isLoading && trustQuery.error ? (
          <div>
            <p className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">
              {t("lspSettings.loadError")}
            </p>
            <Button className="mt-3" onClick={() => { void trustQuery.refetch(); }} size="sm" type="button" variant="outline">
              {t("lspSettings.retry")}
            </Button>
          </div>
        ) : null}
        {!trustQuery.isLoading && !trustQuery.error && trustedRecords.length === 0 ? (
          <p className="rounded-md border border-dashed border-border p-4 text-sm text-muted-foreground">
            {t("lspSettings.trust.empty")}
          </p>
        ) : null}
        {trustedRecords.length > 0 ? (
          <ul className="divide-y divide-border rounded-lg border border-border">
            {trustedRecords.map((record) => (
              <li className="flex flex-col gap-3 p-4 sm:flex-row sm:items-center sm:justify-between" key={record.canonicalRoot}>
                <div className="min-w-0">
                  <Badge tone="success">{t("lspSettings.trust.trusted")}</Badge>
                  <code className="mt-2 block break-all text-xs text-muted-foreground">{record.canonicalRoot}</code>
                </div>
                <Button
                  aria-label={`${t("lspSettings.trust.revoke")} ${record.canonicalRoot}`}
                  disabled={pending}
                  onClick={() => updateTrust({ canonicalRoot: record.canonicalRoot, trusted: false })}
                  size="sm"
                  type="button"
                  variant="outline"
                >
                  {t("lspSettings.trust.revoke")}
                </Button>
              </li>
            ))}
          </ul>
        ) : null}
        {updateMutation.error ? (
          <p className="rounded-md border p-3 text-sm ucd-status-danger" role="alert">
            {t("lspSettings.trust.updateError")}
          </p>
        ) : null}
        {notice ? <p className="sr-only" role="status">{notice}</p> : null}
      </div>
    </SectionPanel>
  );
}
