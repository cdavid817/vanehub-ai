import type {
  AntigravityConfigPayload,
  AntigravityToolPermission,
  ClaudeCodeConfigPayload,
  CliConfigPayload,
  CodexCliConfigPayload,
  GeminiCliConfigPayload,
  OpenCodeConfigPayload,
} from "../../../types/cli-agent-config";
import { useTranslation } from "react-i18next";

const inputClass = "ucd-input h-9 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

function TextField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="flex flex-col gap-1 text-sm">
      {label}
      <input className={inputClass} onChange={(event) => onChange(event.target.value)} value={value} />
    </label>
  );
}

function ClaudeFields({
  payload,
  onChange,
  advanced,
}: {
  payload: ClaudeCodeConfigPayload;
  onChange: (payload: ClaudeCodeConfigPayload) => void;
  advanced: boolean;
}) {
  const { t } = useTranslation();
  if (advanced) return (
    <div className="grid gap-3 sm:grid-cols-2">
      <TextField label={t("agents.globalConfig.field.haikuModel")} value={payload.haikuModel} onChange={(haikuModel) => onChange({ ...payload, haikuModel })} />
      <TextField label={t("agents.globalConfig.field.sonnetModel")} value={payload.sonnetModel} onChange={(sonnetModel) => onChange({ ...payload, sonnetModel })} />
      <TextField label={t("agents.globalConfig.field.opusModel")} value={payload.opusModel} onChange={(opusModel) => onChange({ ...payload, opusModel })} />
    </div>
  );
  return (
    <div className="grid gap-3 sm:grid-cols-2">
      <TextField label={t("agents.globalConfig.field.baseUrl")} value={payload.baseUrl} onChange={(baseUrl) => onChange({ ...payload, baseUrl })} />
      <label className="flex flex-col gap-1 text-sm">
        {t("agents.globalConfig.field.authentication")}
        <select className={inputClass} value={payload.authMode} onChange={(event) => onChange({ ...payload, authMode: event.target.value as ClaudeCodeConfigPayload["authMode"] })}>
          <option value="auth-token">{t("agents.globalConfig.field.authToken")}</option>
          <option value="api-key">{t("agents.globalConfig.field.apiKey")}</option>
          <option value="none">{t("agents.globalConfig.field.existingAuth")}</option>
        </select>
      </label>
      <TextField label={t("agents.globalConfig.field.primaryModel")} value={payload.model} onChange={(model) => onChange({ ...payload, model })} />
    </div>
  );
}

function CodexFields({
  payload,
  onChange,
  advanced,
}: {
  payload: CodexCliConfigPayload;
  onChange: (payload: CodexCliConfigPayload) => void;
  advanced: boolean;
}) {
  const { t } = useTranslation();
  if (!advanced) return (
    <div className="grid gap-3 sm:grid-cols-2">
      <TextField label={t("agents.globalConfig.field.baseUrl")} value={payload.baseUrl} onChange={(baseUrl) => onChange({ ...payload, baseUrl })} />
      <TextField label={t("agents.globalConfig.field.model")} value={payload.model} onChange={(model) => onChange({ ...payload, model })} />
      <label className="flex flex-col gap-1 text-sm">
        {t("agents.globalConfig.field.authentication")}
        <select className={inputClass} value={payload.authStrategy} onChange={(event) => onChange({ ...payload, authStrategy: event.target.value as CodexCliConfigPayload["authStrategy"] })}>
          <option value="preserve-official">{t("agents.globalConfig.field.preserveOfficial")}</option>
          <option value="bearer-token">{t("agents.globalConfig.field.bearerToken")}</option>
          <option value="replace-auth">{t("agents.globalConfig.field.replaceAuth")}</option>
        </select>
      </label>
    </div>
  );
  return (
    <div className="grid gap-3 sm:grid-cols-2">
      <TextField label={t("agents.globalConfig.field.providerId")} value={payload.providerId} onChange={(providerId) => onChange({ ...payload, providerId })} />
      <label className="flex flex-col gap-1 text-sm">
        {t("agents.globalConfig.field.wireApi")}
        <select className={inputClass} value={payload.wireApi} onChange={(event) => onChange({ ...payload, wireApi: event.target.value as CodexCliConfigPayload["wireApi"] })}>
          <option value="responses">Responses</option>
          <option value="chat">Chat Completions</option>
        </select>
      </label>
      <label className="flex flex-col gap-1 text-sm">
        {t("agents.globalConfig.field.reasoning")}
        <select className={inputClass} value={payload.reasoningEffort} onChange={(event) => onChange({ ...payload, reasoningEffort: event.target.value as CodexCliConfigPayload["reasoningEffort"] })}>
          {(["none", "low", "medium", "high", "xhigh", "max"] as const).map((value) => <option key={value} value={value}>{value}</option>)}
        </select>
      </label>
    </div>
  );
}

function OpenCodeFields({
  payload,
  onChange,
  advanced,
}: {
  payload: OpenCodeConfigPayload;
  onChange: (payload: OpenCodeConfigPayload) => void;
  advanced: boolean;
}) {
  const { t } = useTranslation();
  const modelsText = payload.models.map((model) => `${model.id}:${model.name}`).join("\n");
  return (
    <div className="grid gap-3 sm:grid-cols-2">
      {advanced ? <TextField label={t("agents.globalConfig.field.providerId")} value={payload.providerId} onChange={(providerId) => onChange({ ...payload, providerId })} /> : null}
      {!advanced ? <TextField label={t("agents.globalConfig.field.providerName")} value={payload.providerName} onChange={(providerName) => onChange({ ...payload, providerName })} /> : null}
      {advanced ? <TextField label={t("agents.globalConfig.field.npm")} value={payload.npm} onChange={(npm) => onChange({ ...payload, npm })} /> : null}
      {!advanced ? <TextField label={t("agents.globalConfig.field.baseUrl")} value={payload.baseUrl} onChange={(baseUrl) => onChange({ ...payload, baseUrl })} /> : null}
      {!advanced ? <TextField label={t("agents.globalConfig.field.defaultModel")} value={payload.defaultModel} onChange={(defaultModel) => onChange({ ...payload, defaultModel })} /> : null}
      {advanced ? <label className="flex flex-col gap-1 text-sm sm:col-span-2">
        {t("agents.globalConfig.field.models")}
        <textarea
          className="ucd-input min-h-24 rounded px-3 py-2 font-mono text-xs outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          onChange={(event) => {
            const models = event.target.value
              .split(/\r?\n/)
              .map((line) => line.trim())
              .filter(Boolean)
              .map((line) => {
                const separator = line.indexOf(":");
                return separator < 0
                  ? { id: line, name: line }
                  : { id: line.slice(0, separator).trim(), name: line.slice(separator + 1).trim() };
              });
            onChange({ ...payload, models });
          }}
          value={modelsText}
        />
      </label> : null}
    </div>
  );
}

function GeminiFields({ payload, onChange, advanced }: {
  payload: GeminiCliConfigPayload;
  onChange: (payload: GeminiCliConfigPayload) => void;
  advanced: boolean;
}) {
  const { t } = useTranslation();
  if (advanced) return null;
  return (
    <div className="grid gap-3 sm:grid-cols-2">
      <TextField label={t("agents.globalConfig.field.baseUrl")} value={payload.baseUrl} onChange={(baseUrl) => onChange({ ...payload, baseUrl })} />
      <TextField label={t("agents.globalConfig.field.model")} value={payload.model} onChange={(model) => onChange({ ...payload, model })} />
      <label className="flex flex-col gap-1 text-sm">
        {t("agents.globalConfig.field.authentication")}
        <select className={inputClass} value={payload.authStrategy} onChange={(event) => onChange({ ...payload, authStrategy: event.target.value as GeminiCliConfigPayload["authStrategy"] })}>
          <option value="preserve-official">{t("agents.globalConfig.field.preserveOfficial")}</option>
          <option value="api-key">{t("agents.globalConfig.field.apiKey")}</option>
        </select>
      </label>
    </div>
  );
}

export function CliConfigPayloadFields({
  payload,
  onChange,
  section = "basic",
}: {
  payload: CliConfigPayload;
  onChange: (payload: CliConfigPayload) => void;
  section?: "basic" | "advanced";
}) {
  const advanced = section === "advanced";
  if (payload.kind === "claude-code") return <ClaudeFields advanced={advanced} payload={payload} onChange={onChange} />;
  if (payload.kind === "codex-cli") return <CodexFields advanced={advanced} payload={payload} onChange={onChange} />;
  if (payload.kind === "antigravity") return <AntigravityFields advanced={advanced} payload={payload} onChange={onChange} />;
  if (payload.kind === "gemini-cli") return <GeminiFields advanced={advanced} payload={payload} onChange={onChange} />;
  return <OpenCodeFields advanced={advanced} payload={payload} onChange={onChange} />;
}

export function payloadHasAdvancedFields(payload: CliConfigPayload) {
  return payload.kind !== "gemini-cli";
}

const toolPermissions: AntigravityToolPermission[] = [
  "request-review",
  "proceed-in-sandbox",
  "always-proceed",
  "strict",
];

/**
 * No base URL and no credential field: Antigravity authenticates through the system keyring and
 * accepts no custom endpoint, so this form edits the settings it does honor.
 */
function AntigravityFields({
  payload,
  onChange,
  advanced,
}: {
  payload: AntigravityConfigPayload;
  onChange: (payload: AntigravityConfigPayload) => void;
  advanced: boolean;
}) {
  const { t } = useTranslation();
  return (
    <div className="grid gap-3 sm:grid-cols-2">
      {!advanced ? <TextField label={t("agents.globalConfig.field.model")} value={payload.model} onChange={(model) => onChange({ ...payload, model })} /> : null}
      {advanced ? <label className="flex flex-col gap-1 text-sm">
        {t("agents.globalConfig.field.toolPermission")}
        <select
          className={inputClass}
          onChange={(event) => onChange({ ...payload, toolPermission: event.target.value as AntigravityToolPermission })}
          value={payload.toolPermission}
        >
          {toolPermissions.map((value) => (
            <option key={value} value={value}>{t(`agents.globalConfig.antigravity.toolPermission.${value}`)}</option>
          ))}
        </select>
      </label> : null}
      {advanced ? <TextField label={t("agents.globalConfig.field.verbosity")} value={payload.verbosity} onChange={(verbosity) => onChange({ ...payload, verbosity })} /> : null}
      {advanced ? <label className="flex items-center gap-2 text-sm">
        <input
          checked={payload.enableTerminalSandbox}
          onChange={(event) => onChange({ ...payload, enableTerminalSandbox: event.target.checked })}
          type="checkbox"
        />
        {t("agents.globalConfig.field.terminalSandbox")}
      </label> : null}
    </div>
  );
}
