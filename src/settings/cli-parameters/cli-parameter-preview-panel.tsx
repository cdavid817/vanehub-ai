import { useState } from "react";
import { Copy } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import type { CliArgumentSegments, CliArgumentToken } from "../../types/cli-parameter-profile";
import { SectionPanel } from "../pages/page-parts";

function TokenRow({ label, tokens }: { label: string; tokens: readonly CliArgumentToken[] }) {
  if (tokens.length === 0) return null;
  return (
    <div className="space-y-1">
      <p className="text-xs font-medium text-muted-foreground">{label}</p>
      <ul className="flex flex-wrap gap-1">
        {tokens.map((token, index) => (
          <li key={`${token.parameterId}-${index}`}>
            <code className="rounded border border-border bg-muted px-2 py-1 text-xs leading-5 text-foreground">
              {token.value}
            </code>
          </li>
        ))}
      </ul>
    </div>
  );
}

export interface CliParameterPreviewPanelProps {
  segments: CliArgumentSegments | null;
  refreshing: boolean;
  stale: boolean;
}

/**
 * Tokens, never a joined command line. A joined string reads like something you could paste into a
 * shell, and it is not: a value containing a space is one argv entry here and two after a shell
 * splits it.
 */
export function CliParameterPreviewPanel({
  segments,
  refreshing,
  stale,
}: CliParameterPreviewPanelProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const argv = segments
    ? [...segments.global, ...segments.invocation].map((token) => token.value)
    : [];

  async function copyArgv() {
    await navigator.clipboard.writeText(JSON.stringify(argv));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <SectionPanel
      description={t("cliParameters.preview.description")}
      title={t("cliParameters.preview.title")}
    >
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <Button disabled={argv.length === 0} onClick={() => void copyArgv()} size="sm" variant="outline">
          <Copy aria-hidden="true" />
          {t(copied ? "cliParameters.actions.copied" : "cliParameters.actions.copyArgv")}
        </Button>
        {refreshing ? <Badge tone="muted">{t("cliParameters.preview.refreshing")}</Badge> : null}
        {stale && !refreshing ? (
          <Badge tone="warning">{t("cliParameters.preview.stale")}</Badge>
        ) : null}
      </div>
      <div aria-label={t("cliParameters.preview.tokenList")} aria-live="polite" className="space-y-3">
        {argv.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("cliParameters.preview.empty")}</p>
        ) : (
          <>
            <TokenRow label={t("cliParameters.preview.globalSegment")} tokens={segments?.global ?? []} />
            <TokenRow
              label={t("cliParameters.preview.invocationSegment")}
              tokens={segments?.invocation ?? []}
            />
          </>
        )}
      </div>
    </SectionPanel>
  );
}
