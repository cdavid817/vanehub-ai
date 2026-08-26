import { normalizeLspConfiguration } from "../../../services/lsp-contract";
import {
  type JsonObject,
  type LspConfiguration,
  type LspLanguageDescriptor,
  type LspLanguageId,
} from "../../../types/lsp";

const maximumInitializationOptionsBytes = 32 * 1024;
const maximumStartupArguments = 32;
const maximumStartupArgumentBytes = 4 * 1024;

export interface LspLanguageDraft {
  enabled: boolean;
  executableOverride: string;
  /** Empty string means "not overridden"; the registry default applies. */
  startupArguments: string;
  initializationOptions: string;
}

export interface LspConfigurationDraft {
  enabled: boolean;
  descriptors: LspLanguageDescriptor[];
  languages: Record<LspLanguageId, LspLanguageDraft>;
}

export type LspFieldErrors = Record<LspLanguageId, string>;

function languageDraft(configuration: LspConfiguration, language: LspLanguageId): LspLanguageDraft {
  const entry = configuration.languages.find((candidate) => candidate.language === language);
  return {
    enabled: entry?.enabled ?? false,
    executableOverride: entry?.executableOverride ?? "",
    startupArguments: entry?.startupArguments?.join("\n") ?? "",
    initializationOptions: JSON.stringify(entry?.initializationOptions ?? {}, null, 2),
  };
}

export function createLspConfigurationDraft(
  configuration: LspConfiguration,
): LspConfigurationDraft {
  return {
    enabled: configuration.enabled,
    descriptors: configuration.descriptors,
    languages: Object.fromEntries(configuration.descriptors.map((descriptor) => [
      descriptor.language, languageDraft(configuration, descriptor.language),
    ])),
  };
}

/**
 * One argument per line. A blank field is "not overridden", which is why it maps to `null` rather
 * than to an empty list: an empty list is the user choosing to pass no arguments at all, and
 * collapsing the two would silently restore a default such as `--stdio`.
 */
function parseStartupArguments(source: string): { error: string | null; value: string[] | null } {
  if (source.trim() === "") return { error: null, value: null };
  const parsed = source.split("\n").map((line) => line.trim()).filter((line) => line !== "");
  if (parsed.length > maximumStartupArguments) {
    return { error: "lspSettings.startupArguments.tooMany", value: null };
  }
  if (new TextEncoder().encode(parsed.join("")).length > maximumStartupArgumentBytes) {
    return { error: "lspSettings.startupArguments.tooLarge", value: null };
  }
  return { error: null, value: parsed };
}

function parseOptions(source: string): { error: string | null; options: JsonObject | null } {
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
  return { error: null, options: parsed as JsonObject };
}

export function validateLspConfigurationDraft(draft: LspConfigurationDraft): {
  configuration: LspConfiguration | null;
  errors: LspFieldErrors;
} {
  const errors: LspFieldErrors = {};
  const languages = draft.descriptors.map((descriptor) => {
    const entry = draft.languages[descriptor.language];
    const options = parseOptions(entry.initializationOptions);
    const startupArguments = parseStartupArguments(entry.startupArguments);
    const error = options.error ?? startupArguments.error;
    if (error) errors[descriptor.language] = error;
    return {
      language: descriptor.language,
      enabled: entry.enabled,
      executableOverride: entry.executableOverride.trim() || null,
      startupArguments: startupArguments.value,
      initializationOptions: options.options ?? {},
    };
  });
  if (Object.keys(errors).length > 0) return { configuration: null, errors };

  try {
    return {
      configuration: normalizeLspConfiguration({
        enabled: draft.enabled,
        languages,
        descriptors: draft.descriptors,
      }),
      errors,
    };
  } catch {
    return {
      configuration: null,
      errors: Object.fromEntries(draft.descriptors.map((descriptor) => [
        descriptor.language, "lspSettings.reason.invalid_configuration",
      ])),
    };
  }
}
