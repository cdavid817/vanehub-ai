import { AudioLines } from "lucide-react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "../../components/ui/button";
import { localMediaMessageKey } from "../../session-workspace/local-media/local-media-errors";
import type { LocalMediaEngine } from "../../types/local-media";
import { pickPageStatus } from "../settings-page-status";
import { compatibilityMessageKey } from "./local-media/compatibility-notice";
import { OcrCard } from "./local-media/ocr-card";
import { PythonEnvironmentPanel } from "./local-media/python-environment-panel";
import { SetupOverview } from "./local-media/setup-overview";
import { SttCard } from "./local-media/stt-card";
import { TtsCard } from "./local-media/tts-card";
import { ToggleField } from "./local-media/profile-fields";
import { issueKeyFor, useLocalMediaSettings } from "./local-media/use-local-media-settings";
import { PageHeader, SectionPanel } from "./page-parts";
import type { SettingsPageContext } from "../settings-pages";

/**
 * Local media configuration: OCR, speech recognition, and speech synthesis.
 *
 * The page configures software the user installed themselves and never installs any of it. There
 * is no download button, no "get models" link, and no automatic retry that would reach the
 * network -- an engine that is not present stays not present, and the page says so.
 */
export function LocalMediaPage({ isActive, onStatusChange }: SettingsPageContext) {
  const { t } = useTranslation();
  const model = useLocalMediaSettings(isActive);
  const { draft, status } = model;

  // Task 12.16: the same conditions the page's own banners already render from -- reported so
  // this `draft-only` page (task 12.17) keeps flagging itself while backgrounded, not only while
  // it is the active page.
  useEffect(() => {
    const notAvailable = !model.nativeAvailable && !model.loading;
    onStatusChange?.(pickPageStatus([
      model.loadError || model.saveState.kind === "failed"
        ? { kind: "error", labelKey: "localMedia.settings.statusError" }
        : null,
      notAvailable ? { kind: "dependency-unavailable", labelKey: "localMedia.settings.nativeOnly" } : null,
      model.dirty ? { kind: "unsaved", labelKey: "localMedia.settings.unsaved" } : null,
    ]));
    return () => onStatusChange?.(null);
  }, [model.dirty, model.loadError, model.loading, model.nativeAvailable, model.saveState.kind, onStatusChange]);

  const statusFor = (engine: LocalMediaEngine) =>
    status?.engines.find((entry) => entry.engine === engine);

  /**
   * A field's current problem: a validation issue, or the one field a readiness failure named.
   *
   * Both are reported against the same input, because to the user they are the same question --
   * "what is wrong with this box?" -- and a compatibility failure that only appeared in a banner
   * would leave every path field looking fine while none of them worked. Exactly one field is
   * marked: `readiness.field` is absent unless the engine attributed the failure itself.
   */
  const issueFor = (engine: LocalMediaEngine | null, field: string) => {
    const validation = model.issues.get(issueKeyFor(engine, field))?.messageKey;
    if (validation) return validation;
    if (!engine) return undefined;
    const readiness = statusFor(engine)?.readiness;
    if (readiness?.state !== "unavailable" || readiness.field !== field) return undefined;
    return compatibilityMessageKey(readiness.code);
  };

  // Probing the draft would answer a question about a configuration that is not stored, so a
  // dirty form disables the check rather than silently probing the previous values.
  const probeBlockedKey = model.dirty
    ? "localMedia.settings.probeBlockedDirty"
    : !model.nativeAvailable
      ? "localMedia.composer.nativeOnly"
      : null;

  return (
    <div className="mx-auto w-full max-w-5xl pb-24">
      <PageHeader
        description={t("localMedia.settings.description")}
        icon={AudioLines}
        title={t("localMedia.settings.title")}
      />

      <div className="grid gap-5">
        {!model.nativeAvailable && !model.loading ? (
          <p className="ucd-status-warning rounded-lg px-4 py-3 text-sm" role="status">
            {t("localMedia.settings.nativeOnly")}
          </p>
        ) : null}
        {status && status.platformSupport !== "supported" ? (
          <p className="ucd-status-warning rounded-lg px-4 py-3 text-sm">
            {t(`localMedia.settings.platform.${status.platformSupport}`)}
          </p>
        ) : null}
        <SaveFeedback model={model} />

        {model.loadError ? (
          <p className="ucd-status-danger rounded-lg px-4 py-3 text-sm" role="alert">
            {t(localMediaMessageKey(model.loadError))}
          </p>
        ) : null}

        {draft ? (
          <>
            {/* Task 12.15: every field below shares one profile revision (`use-local-media-settings.ts`),
                so a change to any single one invalidates the last-known readiness of all three engines
                at once -- stated once, here, rather than repeated on every field or card. */}
            <p className="rounded-lg px-4 py-3 text-xs leading-5 text-muted-foreground">
              {t("localMedia.settings.restartNotice")}
            </p>
            <SetupOverview
              dirty={model.dirty}
              discovery={model.pythonDiscovery}
              profile={draft}
              status={status}
            />
            <SectionPanel
              description={t("localMedia.settings.master.description")}
              title={t("localMedia.settings.master.title")}
              variant="settings"
            >
              <div className="flex items-center justify-between gap-4 px-5 py-4 sm:px-6">
                <p className="text-xs leading-5 text-muted-foreground">
                  {t("localMedia.settings.master.hint")}
                </p>
                <ToggleField
                  checked={draft.enabled}
                  label={t("localMedia.settings.master.title")}
                  onChange={(enabled) => model.update((current) => ({ ...current, enabled }))}
                />
              </div>
            </SectionPanel>

            <PythonEnvironmentPanel
              discovery={model.pythonDiscovery}
              issueFor={issueFor}
              loading={model.pythonDiscoveryLoading}
              onRefresh={model.refreshPythonDiscovery}
              onSelect={model.setPythonForEngines}
              profile={draft}
            />

            <OcrCard
              issueFor={issueFor}
              // Saved through the ordinary save path, then re-probed. Nothing here retries or
              // degrades on its own: this runs only when the user presses the confirm button.
              onDisableAcceleration={() => {
                model.save({ ...draft, ocr: { ...draft.ocr, cpuAcceleration: "disabled" } });
                model.probe("ocr");
              }}
              onProbe={() => model.probe("ocr")}
              onUpdate={(mutate) => model.update((current) => ({ ...current, ocr: mutate(current.ocr) }))}
              probeDisabledReasonKey={probeBlockedKey}
              probing={model.probing === "ocr"}
              profile={draft.ocr}
              status={statusFor("ocr")}
            />
            <SttCard
              devices={model.devices}
              issueFor={issueFor}
              onProbe={() => model.probe("stt")}
              onUpdate={(mutate) => model.update((current) => ({ ...current, stt: mutate(current.stt) }))}
              probeDisabledReasonKey={probeBlockedKey}
              probing={model.probing === "stt"}
              profile={draft.stt}
              status={statusFor("stt")}
            />
            <TtsCard
              devices={model.devices}
              issueFor={issueFor}
              onProbe={() => model.probe("tts")}
              onUpdate={(mutate) => model.update((current) => ({ ...current, tts: mutate(current.tts) }))}
              probeDisabledReasonKey={probeBlockedKey}
              probing={model.probing === "tts"}
              profile={draft.tts}
              status={statusFor("tts")}
            />

            <p className="text-xs leading-5 text-muted-foreground">
              {t("localMedia.settings.privacyNote")}
            </p>
            <SaveActionBar model={model} />
          </>
        ) : null}
      </div>
    </div>
  );
}

function SaveActionBar({ model }: { model: ReturnType<typeof useLocalMediaSettings> }) {
  const { t } = useTranslation();
  return (
    <div className="sticky bottom-3 z-20 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border bg-background/95 px-4 py-3 shadow-lg backdrop-blur">
      <span className="text-xs text-muted-foreground" role="status">
        {t(model.dirty ? "localMedia.settings.unsaved" : "localMedia.settings.noUnsaved")}
      </span>
      <div className="flex flex-wrap items-center gap-2">
        <Button disabled={!model.dirty || model.saveState.kind === "saving"} onClick={model.discard} type="button" variant="outline">
          {t("localMedia.settings.discard")}
        </Button>
        <Button
          data-testid="local-media-save"
          disabled={!model.dirty || model.saveState.kind === "saving"}
          onClick={() => model.save()}
          type="button"
        >
          {t(model.saveState.kind === "saving" ? "localMedia.settings.saving" : "localMedia.settings.save")}
        </Button>
      </div>
    </div>
  );
}

function SaveFeedback({ model }: { model: ReturnType<typeof useLocalMediaSettings> }) {
  const { t } = useTranslation();
  const { saveState } = model;

  if (saveState.kind === "conflict") {
    return (
      <div className="ucd-status-warning flex flex-wrap items-center gap-3 rounded-lg px-4 py-3 text-sm" role="alert">
        <span>{t("localMedia.settings.conflict")}</span>
        <Button onClick={model.reloadFromNative} size="sm" type="button" variant="outline">
          {t("localMedia.settings.reload")}
        </Button>
      </div>
    );
  }
  if (saveState.kind === "failed") {
    return (
      <p className="ucd-status-danger rounded-lg px-4 py-3 text-sm" role="alert">
        {t(localMediaMessageKey(saveState.code))}
      </p>
    );
  }
  if (saveState.kind === "saved") {
    return (
      <p className="ucd-status-success rounded-lg px-4 py-3 text-sm" role="status">
        {t("localMedia.settings.saved")}
      </p>
    );
  }
  return null;
}
