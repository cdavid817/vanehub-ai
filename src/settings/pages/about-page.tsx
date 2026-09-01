import { CheckCircle2, Download, ExternalLink, GitFork, Info, RefreshCw, Rocket, RotateCcw, ScrollText, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import {
  aboutBuildChannel,
  aboutCurrentVersion,
  aboutReleasesUrl,
  aboutRepositoryUrl,
} from "../../services/about-service";
import { agentService } from "../../services/runtime-agent-client";
import type { DesktopUpdateSnapshot, UpdatePreferences } from "../../types/desktop-update";
import { pickPageStatus } from "../settings-page-status";
import type { SettingsPageStatus } from "../settings-page-types";
import { PageHeader, SectionPanel } from "./page-parts";

const changelogKeys = ["about.changelog.item1", "about.changelog.item2", "about.changelog.item3"];

function MetadataRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      <div className="mt-1 break-all text-sm font-semibold text-foreground">{value}</div>
    </div>
  );
}

function formatCheckedAt(value: string, language: string) {
  return new Intl.DateTimeFormat(language, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

export function AboutPage({ onStatusChange }: { onStatusChange?: (status: SettingsPageStatus | null) => void }) {
  const { i18n, t } = useTranslation();
  const [checking, setChecking] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<DesktopUpdateSnapshot | null>(null);
  const [preferences, setPreferences] = useState<UpdatePreferences>({ automaticCheck: false, channel: "preview" });
  const [updateError, setUpdateError] = useState<string | null>(null);

  async function handleCheckUpdates() {
    setChecking(true);
    setUpdateError(null);

    try {
      const receipt = await agentService.checkForDesktopUpdate();
      setUpdateInfo(await awaitTerminalSnapshot(receipt.snapshot));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setUpdateError(message);
    } finally {
      setChecking(false);
    }
  }

  useEffect(() => {
    void Promise.all([agentService.getDesktopUpdateSnapshot(), agentService.getDesktopUpdatePreferences()])
      .then(([nextSnapshot, nextPreferences]) => { setUpdateInfo(nextSnapshot); setPreferences(nextPreferences); })
      .catch(() => undefined);
  }, []);

  async function savePreferences(next: UpdatePreferences) {
    setPreferences(await agentService.saveDesktopUpdatePreferences(next));
  }

  async function installUpdate() {
    setUpdateError(null);
    try { setUpdateInfo(await awaitTerminalSnapshot((await agentService.downloadAndInstallDesktopUpdate()).snapshot)); }
    catch (error) { setUpdateError(error instanceof Error ? error.message : String(error)); }
  }

  async function awaitTerminalSnapshot(initial: DesktopUpdateSnapshot) {
    let next = initial;
    for (let attempt = 0; attempt < 300 && ["queued", "checking", "downloading"].includes(next.phase); attempt += 1) {
      await new Promise((resolve) => window.setTimeout(resolve, 200));
      next = await agentService.getDesktopUpdateSnapshot();
      setUpdateInfo(next);
    }
    return next;
  }

  const updateStatus = updateInfo?.phase === "available"
    ? t("about.update.available", { version: updateInfo.latestVersion })
    : updateInfo?.phase === "ready-to-restart"
      ? t("about.update.readyRestart")
      : updateInfo?.phase === "failed"
        ? t("about.update.failed", { message: updateInfo.error })
        : updateInfo
      ? t("about.update.current", { version: updateInfo.currentVersion })
      : t("about.update.notChecked");

  // Task 12.16: the same desktop-update phases the banner above already renders from, combined
  // via the shared priority order (error > restart-required > update-available) so a failed
  // recheck takes over from an earlier successful one that had already found something ready to
  // install or restart for.
  useEffect(() => {
    onStatusChange?.(pickPageStatus([
      updateError || updateInfo?.phase === "failed"
        ? { kind: "error", labelKey: "about.status.error" }
        : null,
      // Reuses this page's own already-rendered copy for these two (`about.update.readyRestart`,
      // `about.update.available`) rather than duplicating it under a new key.
      updateInfo?.phase === "ready-to-restart"
        ? { kind: "restart-required", labelKey: "about.update.readyRestart" }
        : null,
      updateInfo?.phase === "available"
        ? { kind: "update-available", labelKey: "about.update.available", labelParams: { version: updateInfo.latestVersion ?? "" } }
        : null,
    ]));
    return () => onStatusChange?.(null);
  }, [onStatusChange, updateError, updateInfo?.error, updateInfo?.latestVersion, updateInfo?.phase]);

  return (
    <div className="space-y-4">
      <PageHeader description={t("about.description")} icon={Info} title={t("about.title")} />

      <div className="grid items-start gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(420px,0.9fr)]">
        <SectionPanel icon={Info} title={t("about.software.title")} description={t("about.software.description")}>
          <div className="grid gap-5">
            <div className="flex min-w-0 items-start gap-4">
              <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-md border border-primary/30 bg-[hsl(var(--nav-active-soft))] text-xl font-bold text-primary">
                V
              </div>
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <h3 className="text-lg font-semibold tracking-tight">VaneHub AI</h3>
                  <Badge tone="muted">v{aboutCurrentVersion}</Badge>
                  {aboutBuildChannel === "Preview" ? <Badge tone="success">{aboutBuildChannel}</Badge> : null}
                </div>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">{t("about.productSummary")}</p>
              </div>
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              <MetadataRow label={t("about.software.version")} value={`v${aboutCurrentVersion}`} />
              <MetadataRow label={t("about.software.channel")} value={aboutBuildChannel} />
              <MetadataRow label={t("about.software.repository")} value={aboutRepositoryUrl} />
              <MetadataRow label={t("about.software.license")} value={t("about.software.licenseValue")} />
            </div>

            <div className="grid gap-4 border-t border-border/70 pt-5">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="flex items-center gap-2 text-sm font-semibold">
                  <RefreshCw className="h-4 w-4 text-primary" aria-hidden="true" />
                  {t("about.update.title")}
                </div>
                <Button disabled={checking} onClick={() => void handleCheckUpdates()}>
                  <RefreshCw className={checking ? "h-4 w-4 animate-spin" : "h-4 w-4"} aria-hidden="true" />
                  {checking ? t("about.update.checking") : t("about.update.check")}
                </Button>
              </div>
              <p className="text-sm leading-6 text-muted-foreground">{t("about.update.description")}</p>
              <div className="grid gap-3 sm:grid-cols-2">
                <label className="grid gap-1 text-xs font-medium text-muted-foreground">
                  {t("about.update.channel")}
                  <select className="h-9 rounded-md border border-input bg-background px-3 text-sm text-foreground" value={preferences.channel} onChange={(event) => void savePreferences({ ...preferences, channel: event.target.value === "stable" ? "stable" : "preview" })}>
                    <option value="stable">{t("about.update.stable")}</option>
                    <option value="preview">{t("about.update.preview")}</option>
                  </select>
                </label>
                <label className="flex items-center gap-2 self-end rounded-md border border-border px-3 py-2 text-sm">
                  <input checked={preferences.automaticCheck} onChange={(event) => void savePreferences({ ...preferences, automaticCheck: event.target.checked })} type="checkbox" />
                  {t("about.update.automatic")}
                </label>
              </div>
              <div className="flex items-start gap-3 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3 text-sm">
                {updateInfo?.phase === "available" ? (
                  <Rocket className="mt-0.5 h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
                ) : (
                  <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
                )}
                <div className="min-w-0">
                  <div className="font-medium">{updateStatus}</div>
                  {updateInfo?.checkedAt ? (
                    <div className="mt-1 text-xs text-muted-foreground">
                      {t("about.update.checkedAt", { time: formatCheckedAt(updateInfo.checkedAt, i18n.language) })}
                    </div>
                  ) : null}
                </div>
              </div>
              {updateError || updateInfo?.phase === "failed" ? (
                <div className="flex flex-wrap items-center justify-between gap-3 rounded-md border p-3 text-xs ucd-status-warning" role="alert">
                  <span>{t("about.update.failed", { message: updateError ?? updateInfo?.error })}</span>
                  <Button className="h-7 px-2 text-xs" disabled={checking} onClick={() => void handleCheckUpdates()} size="sm" type="button" variant="outline">
                    <RefreshCw aria-hidden="true" className={checking ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"} />
                    {t("about.update.check")}
                  </Button>
                </div>
              ) : null}
              {updateInfo?.releaseNotes ? (
                <p className="line-clamp-4 text-xs leading-5 text-muted-foreground">{updateInfo.releaseNotes}</p>
              ) : null}
              {updateInfo?.phase === "available" ? <Button onClick={() => void installUpdate()}><Download className="h-4 w-4" aria-hidden="true" />{t("about.update.install")}</Button> : null}
              {updateInfo?.phase === "downloading" ? <div className="text-xs text-muted-foreground">{t("about.update.progress", { downloaded: updateInfo.downloadedBytes ?? 0, total: updateInfo.totalBytes ?? 0 })}</div> : null}
              {updateInfo?.phase === "ready-to-restart" ? <Button onClick={() => void agentService.restartAfterDesktopUpdate()}><RotateCcw className="h-4 w-4" aria-hidden="true" />{t("about.update.restart")}</Button> : null}
            </div>

            <div className="flex flex-wrap gap-2 border-t border-border/70 pt-5">
              <Button asChild variant="outline">
                <a href={aboutRepositoryUrl} rel="noreferrer" target="_blank">
                  <GitFork className="h-4 w-4" aria-hidden="true" />
                  {t("about.github")}
                </a>
              </Button>
              <Button asChild variant="outline">
                <a href={aboutReleasesUrl} rel="noreferrer" target="_blank">
                  <ExternalLink className="h-4 w-4" aria-hidden="true" />
                  {t("about.releaseNotes")}
                </a>
              </Button>
            </div>
          </div>
        </SectionPanel>

        <SectionPanel icon={ScrollText} title={`${t("about.changelog.title")} / ${t("about.highlights.title")}`} description={t("about.changelog.description")}>
          <div className="grid gap-5">
            <div className="grid gap-3">
              {changelogKeys.map((key, index) => (
                <div className="flex gap-3 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3" key={key}>
                  <span className="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-md border border-border bg-[hsl(var(--panel-glass))] text-xs font-semibold text-primary">
                    {index + 1}
                  </span>
                  <p className="text-sm leading-6 text-muted-foreground">{t(key)}</p>
                </div>
              ))}
            </div>

            <div className="grid gap-4 border-t border-border/70 pt-5">
              <div className="flex items-center gap-2 text-sm font-semibold">
                <Sparkles className="h-4 w-4 text-primary" aria-hidden="true" />
                {t("about.highlights.title")}
              </div>
              <div className="grid gap-3 text-sm text-muted-foreground">
                <div className="flex gap-2">
                  <Sparkles className="mt-0.5 h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
                  <span>{t("about.highlights.multiAgent")}</span>
                </div>
                <div className="flex gap-2">
                  <ScrollText className="mt-0.5 h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
                  <span>{t("about.highlights.localFirst")}</span>
                </div>
              </div>
            </div>
          </div>
        </SectionPanel>
      </div>
    </div>
  );
}
