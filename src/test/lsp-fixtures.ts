import type {
  LspConfiguration,
  LspLanguageConfiguration,
  LspLanguageDescriptor,
} from "../types/lsp";

/**
 * Mirrors what the backend registry declares today. Kept in one place so a test asserting on
 * configuration does not also have to restate the language set, which is exactly the duplication
 * the registry exists to remove.
 */
export function lspTestDescriptors(): LspLanguageDescriptor[] {
  return [
    {
      language: "rust",
      server: "rust_analyzer",
      supportedOnHost: true,
      defaultStartupArguments: [],
    },
    {
      language: "typescript_javascript",
      server: "typescript_language_server",
      supportedOnHost: true,
      defaultStartupArguments: ["--stdio"],
    },
  ];
}

export function lspTestLanguage(
  language: string,
  overrides: Partial<Omit<LspLanguageConfiguration, "language">> = {},
): LspLanguageConfiguration {
  return {
    language,
    enabled: false,
    executableOverride: null,
    startupArguments: null,
    initializationOptions: {},
    ...overrides,
  };
}

export function lspTestConfiguration(
  overrides: Partial<LspConfiguration> = {},
): LspConfiguration {
  return {
    enabled: false,
    languages: [lspTestLanguage("rust"), lspTestLanguage("typescript_javascript")],
    descriptors: lspTestDescriptors(),
    ...overrides,
  };
}
