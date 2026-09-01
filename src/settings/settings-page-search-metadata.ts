import type {
  SettingsPageId,
  SettingsRiskLevel,
  SettingsSaveMode,
  SettingsSearchField,
} from "./settings-page-types";

/**
 * Per-page search/save/risk metadata (task 12.1), split out of `settings-pages.ts` for the same
 * reason `settings-page-lifecycle.ts` is separate: the page list is where navigation and product
 * ordering live and has to stay readable at a glance and under the line budget, while this lookup
 * grows by one self-contained entry at a time and has no bearing on navigation order.
 */
export interface SettingsPageSearchMetadata {
  /** Bounded, localized summary shown under the label in search results (task 12.5). */
  descriptionKey: string;
  /** Extra page-level search synonyms beyond `labelKey`/`descriptionKey` themselves. */
  keywords: string[];
  /** Empty is valid and honest for a page whose fields have not been indexed yet -- the page
   *  still matches on label/description/keywords, it just contributes no field-level hits. */
  fields: SettingsSearchField[];
  saveMode: SettingsSaveMode;
  risk: SettingsRiskLevel;
}

export const SETTINGS_PAGE_SEARCH_METADATA: Record<SettingsPageId, SettingsPageSearchMetadata> = {
  basic: {
    descriptionKey: "settings.pages.basic.description",
    keywords: ["language", "theme", "proxy", "startup", "reset"],
    fields: [],
    saveMode: "immediate",
    risk: "normal",
  },
  "agent-configurations": {
    descriptionKey: "settings.pages.agentConfigurations.description",
    keywords: ["api key", "onepiece", "provider", "credentials", "profile"],
    fields: [],
    saveMode: "mixed",
    risk: "sensitive",
  },
  "agent-policies": {
    descriptionKey: "settings.pages.agentPolicies.description",
    keywords: ["trust", "yolo", "auto-execute", "permission", "sandbox"],
    fields: [],
    saveMode: "immediate",
    risk: "dangerous",
  },
  "cli-parameters": {
    descriptionKey: "settings.pages.cliParameters.description",
    keywords: ["flags", "arguments", "argv", "launch options"],
    fields: [],
    saveMode: "draft",
    risk: "normal",
  },
  "code-intelligence": {
    descriptionKey: "settings.pages.codeIntelligence.description",
    keywords: ["lsp", "language server", "workspace trust", "autocomplete"],
    fields: [],
    saveMode: "immediate",
    risk: "normal",
  },
  mcp: {
    descriptionKey: "settings.pages.mcp.description",
    keywords: ["tool", "context protocol", "transport", "stdio"],
    fields: [],
    saveMode: "mixed",
    risk: "sensitive",
  },
  skills: {
    descriptionKey: "settings.pages.skills.description",
    keywords: ["prompt library", "capability", "mount path", "import"],
    fields: [],
    saveMode: "immediate",
    risk: "normal",
  },
  personalization: {
    descriptionKey: "settings.pages.personalization.description",
    keywords: ["instructions", "memory", "context", "system prompt"],
    fields: [],
    saveMode: "draft",
    risk: "normal",
  },
  "prompt-hooks": {
    descriptionKey: "settings.pages.promptHooks.description",
    keywords: ["automation", "trigger", "binding", "trace"],
    fields: [],
    saveMode: "mixed",
    risk: "normal",
  },
  "expert-roles": {
    descriptionKey: "settings.pages.expertRoles.description",
    keywords: ["persona", "role", "responsibility", "avatar"],
    fields: [],
    saveMode: "mixed",
    risk: "normal",
  },
  "local-media": {
    descriptionKey: "settings.pages.localMedia.description",
    keywords: ["ocr", "speech", "stt", "tts", "python"],
    fields: [],
    saveMode: "draft",
    risk: "normal",
  },
  providers: {
    descriptionKey: "settings.pages.providers.description",
    keywords: ["install", "upgrade", "cli agent", "doctor", "version"],
    fields: [],
    saveMode: "immediate",
    risk: "normal",
  },
  extensions: {
    descriptionKey: "settings.pages.extensions.description",
    keywords: ["sandbox", "runtime", "framework", "capability"],
    fields: [],
    saveMode: "mixed",
    risk: "normal",
  },
  plugins: {
    descriptionKey: "settings.pages.plugins.description",
    keywords: ["integration", "connector", "readiness", "third-party"],
    fields: [],
    saveMode: "immediate",
    risk: "normal",
  },
  im: {
    descriptionKey: "settings.pages.im.description",
    keywords: ["bot token", "webhook", "wechat", "chat platform"],
    fields: [],
    saveMode: "immediate",
    risk: "sensitive",
  },
  "ssh-connections": {
    descriptionKey: "settings.pages.sshConnections.description",
    keywords: ["ssh", "remote", "password", "key"],
    fields: [],
    saveMode: "mixed",
    risk: "sensitive",
  },
  observability: {
    descriptionKey: "settings.pages.observability.description",
    keywords: ["otlp", "telemetry", "trace", "retention", "export"],
    fields: [],
    saveMode: "immediate",
    // Corrected from an initial "normal" pass: `observability-settings-page.tsx`'s OTLP export
    // section renders a real `type="password"` field for `otlpAuthToken` (the collector's bearer
    // token), `autoComplete="new-password"`, "configured vs. not configured" placeholder -- the
    // same shape `im`/`ssh-connections` already use for their own credential fields.
    risk: "sensitive",
  },
  usage: {
    descriptionKey: "settings.pages.usage.description",
    keywords: ["tokens", "cost", "billing", "consumption"],
    fields: [],
    saveMode: "immediate",
    risk: "normal",
  },
  help: {
    descriptionKey: "settings.pages.help.description",
    keywords: ["readme", "guide", "manual", "repository"],
    fields: [],
    saveMode: "immediate",
    risk: "normal",
  },
  about: {
    descriptionKey: "settings.pages.about.description",
    keywords: ["version", "update", "changelog", "release"],
    fields: [],
    saveMode: "immediate",
    risk: "normal",
  },
};
