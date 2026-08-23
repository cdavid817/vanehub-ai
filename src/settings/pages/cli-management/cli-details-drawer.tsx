import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import type { OperationTask } from "../../../types/operation";
import { CliDiagnosticsTab } from "./cli-diagnostics-tab";
import { CliInstallationsTab } from "./cli-installations-tab";
import { CliOperationsTab } from "./cli-operations-tab";
import { CliOverviewTab } from "./cli-overview-tab";

type TabId = "overview" | "installations" | "diagnostics" | "operations";

const TABS: Array<{ id: TabId; labelKey: string }> = [
  { id: "overview", labelKey: "cli.tab.overview" },
  { id: "installations", labelKey: "cli.tab.installations" },
  { id: "diagnostics", labelKey: "cli.tab.diagnostics" },
  { id: "operations", labelKey: "cli.tab.operations" },
];

/**
 * The four-tab detail view.
 *
 * Built on `ApplicationDialog`, which already owns the focus trap, Escape handling, and focus
 * restore -- reimplementing those here would give this one surface its own subtly different
 * behaviour.
 *
 * Tabs use roving `tabIndex` with arrow-key movement, so the tab strip is one stop rather than
 * four, which is what the tab pattern expects.
 */
export function CliDetailsDrawer({
  snapshot,
  panelId,
  operation,
  diagnosticsRunning,
  onClose,
  onRerunDiagnostics,
  onCancelOperation,
  returnFocus,
}: {
  snapshot: CliEnvironmentSnapshot;
  /** What the card's trigger names in `aria-controls`, so the two are provably the same thing. */
  panelId: string;
  operation?: OperationTask;
  diagnosticsRunning: boolean;
  onClose: () => void;
  onRerunDiagnostics: () => void;
  onCancelOperation: () => void;
  returnFocus?: HTMLElement | null;
}) {
  const { t } = useTranslation();
  const [active, setActive] = useState<TabId>("overview");
  const baseId = useId();

  function moveFocus(offset: number) {
    const index = TABS.findIndex((tab) => tab.id === active);
    const next = TABS[(index + offset + TABS.length) % TABS.length];
    setActive(next.id);
    document.getElementById(`${baseId}-tab-${next.id}`)?.focus();
  }

  return (
    <ApplicationDialog
      description={t("cli.details.description")}
      returnFocus={returnFocus}
      title={t("cli.details.title", { name: snapshot.displayName })}
      onClose={onClose}
    >
      <div className="space-y-4" id={panelId}>
        <div
          aria-label={t("cli.details.tabs")}
          className="flex flex-wrap gap-1 border-b border-border"
          role="tablist"
          onKeyDown={(event) => {
            if (event.key === "ArrowRight") moveFocus(1);
            if (event.key === "ArrowLeft") moveFocus(-1);
          }}
        >
          {TABS.map((tab) => {
            const selected = tab.id === active;
            return (
              <button
                aria-controls={`${baseId}-panel-${tab.id}`}
                aria-selected={selected}
                className={`-mb-px border-b-2 px-3 py-2 text-sm transition-colors focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring ${
                  selected
                    ? "border-primary font-medium text-primary"
                    : "border-transparent text-muted-foreground hover:text-foreground"
                }`}
                data-dialog-autofocus={selected ? "" : undefined}
                id={`${baseId}-tab-${tab.id}`}
                key={tab.id}
                role="tab"
                tabIndex={selected ? 0 : -1}
                type="button"
                onClick={() => setActive(tab.id)}
              >
                {t(tab.labelKey)}
              </button>
            );
          })}
        </div>

        <div
          aria-labelledby={`${baseId}-tab-${active}`}
          id={`${baseId}-panel-${active}`}
          role="tabpanel"
          tabIndex={0}
        >
          {active === "overview" ? <CliOverviewTab snapshot={snapshot} /> : null}
          {active === "installations" ? <CliInstallationsTab snapshot={snapshot} /> : null}
          {active === "diagnostics" ? (
            <CliDiagnosticsTab
              running={diagnosticsRunning}
              snapshot={snapshot}
              onRerun={onRerunDiagnostics}
            />
          ) : null}
          {active === "operations" ? (
            <CliOperationsTab operation={operation} onCancel={onCancelOperation} />
          ) : null}
        </div>
      </div>
    </ApplicationDialog>
  );
}
