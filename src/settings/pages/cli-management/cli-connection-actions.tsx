import { ExternalLink, PlugZap } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import type { CliConnectionCheck, CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";

/**
 * The two explicit, user-initiated actions detection is not allowed to take on its own.
 *
 * "Check connection" starts the vendor's agent for one ACP handshake and releases it; it never
 * creates a session or sends a prompt, and its result is a negotiation summary, not a sign-in
 * state. "Sign-in guide" opens the vendor's own documentation through the HTTPS-only external
 * link service on a click; the application never signs in on the user's behalf.
 */
export function CliConnectionActions({ snapshot }: { snapshot: CliEnvironmentSnapshot }) {
  const { t } = useTranslation();
  const [checking, setChecking] = useState(false);
  const [report, setReport] = useState<CliConnectionCheck | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [linkFailure, setLinkFailure] = useState(false);
  const canCheck = snapshot.managedTransport === "acp-stdio" && snapshot.pathSelectedInstallationId !== null;
  if (!canCheck && !snapshot.loginDocsUrl) return null;

  async function check() {
    if (checking) return;
    setChecking(true);
    setFailure(null);
    try {
      setReport(await agentService.checkCliConnection(snapshot.agentId, null));
    } catch (error) {
      setReport(null);
      setFailure(error instanceof Error ? error.message : String(error));
    } finally {
      setChecking(false);
    }
  }

  async function openDocs() {
    if (!snapshot.loginDocsUrl) return;
    setLinkFailure(false);
    try {
      await agentService.openExternalUrl(snapshot.loginDocsUrl);
    } catch {
      setLinkFailure(true);
    }
  }

  return (
    <div className="grid gap-2 text-xs" data-testid={`cli-connection-actions-${snapshot.agentId}`}>
      <div className="flex flex-wrap items-center gap-2">
        {canCheck ? (
          <Button disabled={checking} onClick={() => void check()} size="sm" variant="outline">
            <PlugZap aria-hidden="true" />
            {checking ? t("cli.connection.checking") : t("cli.connection.check")}
          </Button>
        ) : null}
        {snapshot.loginDocsUrl ? (
          <Button onClick={() => void openDocs()} size="sm" variant="ghost" title={snapshot.loginDocsUrl}>
            <ExternalLink aria-hidden="true" />
            {t("cli.login.openDocs")}
          </Button>
        ) : null}
      </div>
      {snapshot.loginDocsUrl ? <p className="text-muted-foreground">{t("cli.login.terminalHint")}</p> : null}
      {linkFailure ? <p className="ucd-status-warning" role="alert">{t("cli.login.linkBlocked")}</p> : null}
      {report ? (
        <dl className="grid gap-1 rounded-md border p-2 sm:grid-cols-2" data-testid="cli-connection-report">
          <div>
            <dt className="text-muted-foreground">{t("cli.connection.agent")}</dt>
            <dd className="font-mono">{report.agentName ?? t("cli.versionUnknown")} {report.agentVersion ?? ""}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">{t("cli.connection.protocol")}</dt>
            <dd className="font-mono">{t("cli.connection.protocolValue", { version: report.protocolVersion, ms: report.elapsedMs })}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">{t("cli.connection.resume")}</dt>
            <dd>{report.loadSession ? t("cli.connection.resumeSupported") : t("cli.connection.resumeUnsupported")}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">{t("cli.connection.authMethods")}</dt>
            <dd className="font-mono">{report.authMethods.length > 0 ? report.authMethods.join(", ") : t("cli.connection.authMethodsNone")}</dd>
          </div>
          <p className="text-muted-foreground sm:col-span-2">{t("cli.connection.notSignInProof")}</p>
        </dl>
      ) : null}
      {failure ? (
        <p className="ucd-status-warning" role="alert">{t("cli.connection.failed", { reason: failure })}</p>
      ) : null}
    </div>
  );
}
