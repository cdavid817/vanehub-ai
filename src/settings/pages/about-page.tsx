import { CheckCircle2, ExternalLink, Github, Info, Monitor, RefreshCw, Rocket, ScrollText, Sparkles, Terminal } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import {
  aboutBuildChannel,
  aboutCurrentVersion,
  aboutReleasesUrl,
  aboutRepositoryUrl,
  checkAboutUpdates,
  type AboutUpdateInfo,
} from "../../services/about-service";
import { PageHeader, SectionPanel } from "./page-parts";

const supportedAgents = ["Claude Code", "OpenCode", "Codex CLI", "Gemini CLI"];
const supportedRuntimes = ["Tauri 2 Desktop", "Web / Mock Adapter"];
const changelogKeys = ["about.changelog.item1", "about.changelog.item2", "about.changelog.item3"];

function MetadataRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 break-all text-sm font-medium text-foreground">{value}</div>
    </div>
  );
}

function formatCheckedAt(value: string, language: string) {
  return new Intl.DateTimeFormat(language, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

export function AboutPage() {
  const { i18n, t } = useTranslation();
  const [checking, setChecking] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<AboutUpdateInfo | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);

  async function handleCheckUpdates() {
    setChecking(true);
    setUpdateError(null);

    try {
      setUpdateInfo(await checkAboutUpdates());
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setUpdateError(message);
    } finally {
      setChecking(false);
    }
  }

  const updateStatus = updateInfo?.updateAvailable
    ? t("about.update.available", { version: updateInfo.latestVersion })
    : updateInfo
      ? t("about.update.current", { version: updateInfo.currentVersion })
      : t("about.update.notChecked");

  return (
    <div className="space-y-4">
      <PageHeader description={t("about.description")} icon={Info} title={t("about.title")} />

      <section className="ucd-panel rounded-lg p-4">
        <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_auto] xl:items-center">
          <div className="flex min-w-0 items-start gap-3">
            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-lg border border-primary bg-[hsl(var(--nav-active-soft))] text-xl font-bold text-primary">
              V
            </div>
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <h3 className="text-lg font-semibold tracking-tight">VaneHub AI</h3>
                <Badge tone="muted">v{aboutCurrentVersion}</Badge>
                <Badge tone="success">{aboutBuildChannel}</Badge>
              </div>
              <p className="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">{t("about.productSummary")}</p>
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            <Button asChild variant="outline">
              <a href={aboutRepositoryUrl} rel="noreferrer" target="_blank">
                <Github className="h-4 w-4" aria-hidden="true" />
                {t("about.github")}
              </a>
            </Button>
            <Button asChild variant="outline">
              <a href={aboutReleasesUrl} rel="noreferrer" target="_blank">
                <ExternalLink className="h-4 w-4" aria-hidden="true" />
                {t("about.releaseNotes")}
              </a>
            </Button>
            <Button disabled={checking} onClick={() => void handleCheckUpdates()}>
              <RefreshCw className={checking ? "h-4 w-4 animate-spin" : "h-4 w-4"} aria-hidden="true" />
              {checking ? t("about.update.checking") : t("about.update.check")}
            </Button>
          </div>
        </div>
      </section>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="grid gap-4">
          <SectionPanel title={t("about.software.title")} description={t("about.software.description")}>
            <div className="grid gap-3 md:grid-cols-2">
              <MetadataRow label={t("about.software.version")} value={`v${aboutCurrentVersion}`} />
              <MetadataRow label={t("about.software.channel")} value={aboutBuildChannel} />
              <MetadataRow label={t("about.software.repository")} value={aboutRepositoryUrl} />
              <MetadataRow label={t("about.software.license")} value={t("about.software.licenseValue")} />
            </div>
          </SectionPanel>

          <SectionPanel title={t("about.changelog.title")} description={t("about.changelog.description")}>
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
          </SectionPanel>
        </div>

        <div className="grid gap-4">
          <SectionPanel title={t("about.update.title")} description={t("about.update.description")}>
            <div className="grid gap-3 text-sm">
              <div className="flex items-start gap-2 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
                {updateInfo?.updateAvailable ? (
                  <Rocket className="mt-0.5 h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
                ) : (
                  <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
                )}
                <div className="min-w-0">
                  <div className="font-medium">{updateStatus}</div>
                  {updateInfo ? (
                    <div className="mt-1 text-xs text-muted-foreground">
                      {t("about.update.checkedAt", { time: formatCheckedAt(updateInfo.checkedAt, i18n.language) })}
                    </div>
                  ) : null}
                </div>
              </div>
              {updateInfo?.releaseNotes ? (
                <p className="line-clamp-4 text-xs leading-5 text-muted-foreground">{updateInfo.releaseNotes}</p>
              ) : null}
              {updateError ? <div className="rounded-md border p-3 text-xs ucd-status-warning">{t("about.update.failed", { message: updateError })}</div> : null}
            </div>
          </SectionPanel>

          <SectionPanel title={t("about.runtime.title")} description={t("about.runtime.description")}>
            <div className="grid gap-3">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Monitor className="h-4 w-4 text-primary" aria-hidden="true" />
                {t("about.runtime.supported")}
              </div>
              <div className="flex flex-wrap gap-2">
                {supportedRuntimes.map((runtime) => (
                  <Badge key={runtime} tone="muted">
                    {runtime}
                  </Badge>
                ))}
              </div>
              <div className="mt-2 flex items-center gap-2 text-sm font-medium">
                <Terminal className="h-4 w-4 text-primary" aria-hidden="true" />
                {t("about.agents.supported")}
              </div>
              <div className="flex flex-wrap gap-2">
                {supportedAgents.map((agent) => (
                  <Badge key={agent} tone="muted">
                    {agent}
                  </Badge>
                ))}
              </div>
            </div>
          </SectionPanel>

          <SectionPanel title={t("about.highlights.title")}>
            <div className="grid gap-2 text-sm text-muted-foreground">
              <div className="flex gap-2">
                <Sparkles className="mt-0.5 h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
                <span>{t("about.highlights.multiAgent")}</span>
              </div>
              <div className="flex gap-2">
                <ScrollText className="mt-0.5 h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
                <span>{t("about.highlights.localFirst")}</span>
              </div>
            </div>
          </SectionPanel>
        </div>
      </div>
    </div>
  );
}
