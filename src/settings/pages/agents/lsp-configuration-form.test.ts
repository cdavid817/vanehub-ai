import { describe, expect, it } from "vitest";
import type { LspConfiguration } from "../../../types/lsp";
import { lspTestDescriptors } from "../../../test/lsp-fixtures";
import {
  createLspConfigurationDraft,
  validateLspConfigurationDraft,
} from "./lsp-configuration-form";

const configuration: LspConfiguration = {
  enabled: false,
  languages: [
    {
      language: "rust",
      enabled: false,
      executableOverride: null,
      startupArguments: null,
      initializationOptions: {},
    },
    {
      language: "typescript_javascript",
      enabled: false,
      executableOverride: null,
      startupArguments: null,
      initializationOptions: {},
    },
  ],
  descriptors: lspTestDescriptors(),
};

describe("validateLspConfigurationDraft", () => {
  it("requires initialization options to be a JSON object", () => {
    const draft = createLspConfigurationDraft(configuration);
    draft.languages.rust.initializationOptions = "[]";

    const result = validateLspConfigurationDraft(draft);

    expect(result.configuration).toBeNull();
    expect(result.errors.rust).toBe("lspSettings.initialization.objectRequired");
  });

  it("rejects initialization options larger than 32 KiB", () => {
    const draft = createLspConfigurationDraft(configuration);
    draft.languages.rust.initializationOptions = JSON.stringify({ value: "x".repeat(32 * 1024) });

    const result = validateLspConfigurationDraft(draft);

    expect(result.configuration).toBeNull();
    expect(result.errors.rust).toBe("lspSettings.initialization.tooLarge");
  });
});
