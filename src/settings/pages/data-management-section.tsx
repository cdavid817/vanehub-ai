import { Database, FolderOpen, HardDrive, Palette } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { normalizeDisplayPath } from "../../lib/session-path";
import type { DataManagementInfo } from "../../types/settings";
import { useSettings } from "../settings-provider";
import { SectionPanel } from "./page-parts";

function StorageNote({ icon: Icon, label, value }: { icon: LucideIcon; label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-background p-3">
      <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
        <Icon className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
        {label}
      </div>
      <div className="mt-1 text-sm leading-6 text-muted-foreground">{value}</div>
    </div>
  );
}

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
    <SectionPanel icon={Database} title={t("basic.storage")} description={t("basic.storageDesc")}>
      <div className="grid gap-4">
        {error ? <div className="rounded border p-3 text-xs ucd-status-danger">{error}</div> : null}
        <div className="grid gap-3 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-4">
          <div className="flex items-center gap-3">
            <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-border bg-[hsl(var(--panel-muted))] text-primary">
              <Database className="h-4 w-4" aria-hidden="true" />
            </span>
            <div className="min-w-0">
              <div className="text-sm font-medium text-foreground">{t("basic.databaseLocation")}</div>
              <div className="mt-1 break-all text-sm text-muted-foreground">
                {info?.databasePath ? normalizeDisplayPath(info.databasePath) : t("basic.databaseLoading")}
              </div>
            </div>
          </div>
          <div className="flex min-w-0 flex-col gap-3 border-t border-border/70 pt-3 sm:flex-row sm:items-center">
            <span className="min-w-0 flex-1 break-all text-sm leading-6 text-muted-foreground">
              {info?.databaseDirectory ? normalizeDisplayPath(info.databaseDirectory) : t("basic.databaseDirectoryUnavailable")}
            </span>
            <Button className="shrink-0" disabled={!info?.canOpenDirectory} onClick={handleOpen} variant="outline">
              <FolderOpen className="h-4 w-4" aria-hidden="true" />
              {t("basic.openDatabaseDirectory")}
            </Button>
          </div>
        </div>
        <div className="grid gap-3 md:grid-cols-3">
          <StorageNote icon={HardDrive} label={t("basic.storageDesktopRuntime")} value={t("basic.desktopStorage")} />
          <StorageNote icon={HardDrive} label={t("basic.storageWebRuntime")} value={t("basic.webStorage")} />
          <StorageNote icon={Palette} label={t("basic.storageThemeEntry")} value={t("basic.themeEntry")} />
        </div>
        {!info?.canOpenDirectory ? (
          <div className="rounded border p-3 text-xs ucd-status-warning">{t("basic.databaseOpenUnavailable")}</div>
        ) : null}
      </div>
    </SectionPanel>
  );
}
