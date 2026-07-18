import { Database, FolderOpen } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import type { DataManagementInfo } from "../../types/settings";
import { useSettings } from "../settings-provider";
import { SectionPanel } from "./page-parts";

export function DataManagementSection() {
  const { t } = useTranslation();
  const { getDataManagementInfo, openDatabaseDirectory, reportClientLogEvent } = useSettings();
  const [info, setInfo] = useState<DataManagementInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void getDataManagementInfo()
      .then((nextInfo) => {
        if (active) setInfo(nextInfo);
      })
      .catch((cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      });
    return () => {
      active = false;
    };
  }, [getDataManagementInfo]);

  function handleOpen() {
    setError(null);
    void openDatabaseDirectory().catch((cause) => {
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(message);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "DataManagementSection.openDatabaseDirectory",
        details: { databaseDirectory: info?.databaseDirectory ?? "" },
      });
    });
  }

  return (
    <SectionPanel title={t("basic.dataManagement")} description={t("basic.dataManagementDesc")}>
      <div className="grid gap-4">
        {error ? <div className="rounded border p-3 text-xs ucd-status-danger">{error}</div> : null}
        <div className="flex items-start gap-3 rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
          <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-border bg-background text-primary">
            <Database className="h-5 w-5" aria-hidden="true" />
          </span>
          <div className="min-w-0 flex-1 text-sm">
            <div className="font-medium">{t("basic.databaseLocation")}</div>
            <div className="mt-1 break-all text-xs text-muted-foreground">
              {info?.databasePath ?? t("basic.databaseLoading")}
            </div>
            <div className="mt-2 break-all text-xs text-muted-foreground">
              {info?.databaseDirectory ?? t("basic.databaseDirectoryUnavailable")}
            </div>
          </div>
          <Button disabled={!info?.canOpenDirectory} onClick={handleOpen} variant="outline">
            <FolderOpen className="h-4 w-4" aria-hidden="true" />
            {t("basic.openDatabaseDirectory")}
          </Button>
        </div>
        {!info?.canOpenDirectory ? (
          <div className="rounded border p-3 text-xs ucd-status-warning">{t("basic.databaseOpenUnavailable")}</div>
        ) : null}
      </div>
    </SectionPanel>
  );
}
