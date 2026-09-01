import { CheckCircle2, ExternalLink, GitFork } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { ActionMenu, type ActionMenuItem } from "../../../ui/actions/ActionMenu";
import { MutationStatus } from "../../../ui/async/MutationStatus";
import type { MutationState } from "../../../ui/async/mutation-state";
import { StatusBadge, type StatusTone } from "../../../ui/status/StatusBadge";
import type {
  PluginIntegrationDefinition,
  PluginIntegrationState,
  PluginIntegrationStatus,
  PluginIntegrationTestResult,
} from "../../../types/plugin-integration";
import { statusKey } from "./plugin-integration-utils";

const STATUS_TONE: Record<PluginIntegrationStatus, StatusTone> = {
  configured: "success",
  "not-configured": "warning",
  "missing-cli": "warning",
  unavailable: "warning",
  error: "danger",
};

export function PluginIntegrationCard({
  definition,
  lastResult,
  nativeChecksAvailable,
  onTest,
  state,
  testState,
}: {
  definition: PluginIntegrationDefinition;
  lastResult: PluginIntegrationTestResult | undefined;
  nativeChecksAvailable: boolean;
  onTest: (definition: PluginIntegrationDefinition) => void;
  state: PluginIntegrationState;
  testState: MutationState | undefined;
}) {
  const { t } = useTranslation();
  // Prefers the freshly-completed test's own message over the last-known state's static reason,
  // matching this page's own pre-existing behavior (both are i18n keys, never pre-formatted text).
  const messageKey = lastResult?.integrationId === definition.id ? lastResult.message : state.statusReasonKey;
  const items: ActionMenuItem[] = [
    {
      icon: ExternalLink,
      id: "docs",
      label: t("plugins.action.docs"),
      onSelect: () => window.open(definition.docsUrl, "_blank", "noopener,noreferrer"),
    },
    {
      disabled: !nativeChecksAvailable || !state.canTest || testState?.pending,
      icon: CheckCircle2,
      id: "test",
      label: testState?.pending ? t("plugins.action.testing") : t("plugins.action.test"),
      onSelect: () => onTest(definition),
    },
  ];

  return (
    <article className="ucd-panel ucd-interactive rounded-lg p-4" data-testid={`plugin-card-${definition.id}`}>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border bg-[hsl(var(--panel-muted))] text-foreground">
            <GitFork className="h-5 w-5" aria-hidden="true" />
          </span>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-base font-semibold">{t(definition.nameKey)}</h3>
              <Badge tone="muted">v{definition.version}</Badge>
            </div>
            <p className="mt-1 text-sm leading-6 text-muted-foreground">{t(definition.descriptionKey)}</p>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <StatusBadge label={t(statusKey(state.status))} tone={STATUS_TONE[state.status]} />
          <ActionMenu items={items} triggerLabel={t("plugins.rowActions", { name: t(definition.nameKey) })} />
        </div>
      </div>

      <div className="mt-4 space-y-2">
        {definition.setupSteps.map((step, index) => (
          <div className="flex gap-2 text-sm" key={step.id}>
            <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-sm bg-muted text-xs font-semibold text-muted-foreground">
              {index + 1}
            </span>
            <span className="text-muted-foreground">{t(step.labelKey)}</span>
          </div>
        ))}
        {messageKey ? <div className="text-xs text-muted-foreground">{t(messageKey)}</div> : null}
        {state.lastCheckedAt ? (
          <div className="text-xs text-muted-foreground">{t("plugins.lastChecked", { time: state.lastCheckedAt })}</div>
        ) : null}
      </div>

      <MutationStatus state={testState} />
    </article>
  );
}
