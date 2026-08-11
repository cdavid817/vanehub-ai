import { normalizeLspConfiguration } from "../../../services/lsp-contract";
import {
  lspLanguageIds,
  type JsonObject,
  type LspConfiguration,
  type LspLanguageId,
} from "../../../types/lsp";

const maximumInitializationOptionsBytes = 32 * 1024;

export interface LspLanguageDraft {
  enabled: boolean;
  executableOverride: string;
  initializationOptions: string;
}

export interface LspConfigurationDraft {
  enabled: boolean;
  languages: Record<LspLanguageId, LspLanguageDraft>;
}

export type LspInitializationErrors = Partial<Record<LspLanguageId, string>>;

function languageDraft(configuration: LspConfiguration, language: LspLanguageId): LspLanguageDraft {
  const entry = configuration.languages.find((candidate) => candidate.language === language);
  return {
    enabled: entry?.enabled ?? false,
    executableOverride: entry?.executableOverride ?? "",
    initializationOptions: JSON.stringify(entry?.initializationOptions ?? {}, null, 2),
  };
}

export function createLspConfigurationDraft(
  configuration: LspConfiguration,
): LspConfigurationDraft {
  return {
    enabled: configuration.enabled,
    languages: {
      rust: languageDraft(configuration, "rust"),
      typescript_javascript: languageDraft(configuration, "typescript_javascript"),
    },
  };
}

function parseOptions(
  language: LspLanguageId,
  source: string,
): { error: string | null; options: JsonObject | null } {
  let parsed: unknown;
  try {
    parsed = JSON.parse(source);
  } catch {
    return { error: "lspSettings.initialization.invalidJson", options: null };
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    return { error: "lspSettings.initialization.objectRequired", options: null };
  }
  if (new TextEncoder().encode(JSON.stringify(parsed)).length > maximumInitializationOptionsBytes) {
    return { error: "lspSettings.initialization.tooLarge", options: null };
  }

  try {
    const normalized = normalizeLspConfiguration({
      enabled: false,
      languages: lspLanguageIds.map((candidate) => ({
        language: candidate,
        enabled: false,
        executableOverride: null,
        initializationOptions: candidate === language ? parsed : {},
      })),
    });
    const entry = normalized.languages.find((candidate) => candidate.language === language);
    return entry
      ? { error: null, options: entry.initializationOptions }
      : { error: "lspSettings.reason.invalid_configuration", options: null };
  } catch {
    return { error: "lspSettings.reason.invalid_configuration", options: null };
  }
}

export function validateLspConfigurationDraft(draft: LspConfigurationDraft): {
  configuration: LspConfiguration | null;
  errors: LspInitializationErrors;
} {
  const errors: LspInitializationErrors = {};
  const options: Partial<Record<LspLanguageId, JsonObject>> = {};
  for (const language of lspLanguageIds) {
    const result = parseOptions(language, draft.languages[language].initializationOptions);
    if (result.error) errors[language] = result.error;
    if (result.options) options[language] = result.options;
  }
  if (Object.keys(errors).length > 0) return { configuration: null, errors };

  try {
    return {
      configuration: normalizeLspConfiguration({
        enabled: draft.enabled,
        languages: lspLanguageIds.map((language) => ({
          language,
          enabled: draft.languages[language].enabled,
          executableOverride: draft.languages[language].executableOverride.trim() || null,
          initializationOptions: options[language] ?? {},
        })),
      }),
      errors,
    };
  } catch {
    return {
      configuration: null,
      errors: {
        rust: "lspSettings.reason.invalid_configuration",
        typescript_javascript: "lspSettings.reason.invalid_configuration",
      },
    };
  }
}
