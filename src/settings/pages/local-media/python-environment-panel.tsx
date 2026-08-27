import { RefreshCw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "../../../components/ui/button";
import type {
  LocalMediaEngine,
  LocalMediaProfile,
  PythonEnvironmentCandidate,
  PythonEnvironmentDiscovery,
} from "../../../types/local-media";
import { SectionPanel } from "../page-parts";
import { PathField } from "./profile-fields";
import type { FieldIssueLookup } from "./shared";

const ENGINES: LocalMediaEngine[] = ["ocr", "stt", "tts"];
const CONTROL_CLASS =
  "min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

function versionOf(candidate: PythonEnvironmentCandidate) {
  const { major, minor, patch } = candidate.version;
  return `${major}.${minor}.${patch}`;
}

function valueFor(profile: LocalMediaProfile, engine: LocalMediaEngine) {
  return profile[engine].pythonExecutable;
}

export function PythonEnvironmentPanel({
  discovery,
  loading,
  issueFor,
  onRefresh,
  onSelect,
  profile,
}: {
  discovery: PythonEnvironmentDiscovery | null;
  issueFor: FieldIssueLookup;
  loading: boolean;
  onRefresh: () => void;
  onSelect: (path: string, engines: LocalMediaEngine[]) => void;
  profile: LocalMediaProfile;
}) {
  const { t } = useTranslation();
  const candidates = discovery?.candidates ?? [];
  const compatible = candidates.filter((item) => item.compatibility === "compatible");
  const [sharedPath, setSharedPath] = useState("");

  return (
    <SectionPanel
      description={t("localMedia.settings.python.description")}
      title={t("localMedia.settings.python.title")}
      variant="settings"
    >
      <div className="grid gap-4 px-5 py-4 sm:px-6">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <p className="text-xs leading-5 text-muted-foreground" role="status">
            {t(
              loading
                ? "localMedia.settings.python.detecting"
                : discovery?.availability === "available" && compatible.length > 0
                  ? "localMedia.settings.python.detected"
                  : "localMedia.settings.python.manual",
              { count: compatible.length },
            )}
          </p>
          <Button disabled={loading} onClick={onRefresh} size="sm" type="button" variant="outline">
            <RefreshCw className={loading ? "animate-spin" : ""} />
            {t("localMedia.settings.python.refresh")}
          </Button>
        </div>

        {candidates.length > 0 ? (
          <ul className="grid gap-2" aria-label={t("localMedia.settings.python.inventory")}>
            {candidates.map((candidate) => (
              <li className="rounded-lg border border-border bg-muted/30 px-3 py-2" key={candidate.executablePath}>
                <div className="flex flex-wrap items-center justify-between gap-2 text-xs">
                  <span className="font-medium">
                    Python {versionOf(candidate)} · {t(`localMedia.settings.python.${candidate.compatibility}`)}
                  </span>
                  <span className="text-muted-foreground">
                    {t(`localMedia.settings.python.source.${candidate.source}`)}
                  </span>
                </div>
                <code className="mt-1 block break-all text-xs text-muted-foreground">
                  {candidate.executablePath}
                </code>
              </li>
            ))}
          </ul>
        ) : null}

        <div className="grid gap-4 lg:grid-cols-3">
          {ENGINES.map((engine) => {
            const current = valueFor(profile, engine);
            const detected = candidates.some((item) => item.executablePath === current);
            return (
              <div className="grid content-start gap-2 rounded-lg border border-border p-3" key={engine}>
                <label className="text-xs font-semibold" htmlFor={`local-media-${engine}-python-select`}>
                  {t(`localMedia.settings.${engine}.shortTitle`)}
                </label>
                <select
                  className={CONTROL_CLASS}
                  id={`local-media-${engine}-python-select`}
                  onChange={(event) => onSelect(event.currentTarget.value, [engine])}
                  value={detected ? current : ""}
                >
                  <option value="">{t(current ? "localMedia.settings.python.notDetected" : "localMedia.settings.python.choose")}</option>
                  {compatible.map((candidate) => (
                    <option key={candidate.executablePath} value={candidate.executablePath}>
                      Python {versionOf(candidate)} · {candidate.executablePath}
                    </option>
                  ))}
                </select>
                <PathField
                  id={`local-media-${engine}-python`}
                  issueKey={issueFor(engine, "pythonExecutable")}
                  kind="file"
                  label={t("localMedia.settings.python.custom")}
                  onChange={(path) => onSelect(path, [engine])}
                  value={current}
                />
              </div>
            );
          })}
        </div>

        {compatible.length > 0 ? (
          <div className="flex flex-wrap items-end gap-2 rounded-lg bg-muted/40 p-3">
            <label className="grid min-w-0 flex-1 gap-1 text-xs font-medium" htmlFor="local-media-python-all">
              {t("localMedia.settings.python.applyAllLabel")}
              <select className={CONTROL_CLASS} id="local-media-python-all" onChange={(event) => setSharedPath(event.currentTarget.value)} value={sharedPath}>
                <option value="">{t("localMedia.settings.python.choose")}</option>
                {compatible.map((candidate) => (
                  <option key={candidate.executablePath} value={candidate.executablePath}>
                    Python {versionOf(candidate)} · {candidate.executablePath}
                  </option>
                ))}
              </select>
            </label>
            <Button disabled={!sharedPath} onClick={() => onSelect(sharedPath, ENGINES)} type="button" variant="outline">
              {t("localMedia.settings.python.applyAll")}
            </Button>
          </div>
        ) : null}
      </div>
    </SectionPanel>
  );
}
