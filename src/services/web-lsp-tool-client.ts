/**
 * The Web/mock side of the LSP Agent tools. Split from the configuration mock because they answer
 * different questions: that one models a settings backend, this one models a runtime that is never
 * there. Nine tools' worth of envelopes also put the combined file over the 300-line rule.
 */
import type {
  LspToolName,
  LspToolResult,
  LspToolResultMetadata,
} from "../types/lsp";

export interface WebLspToolClient {
  execute(tool: LspToolName): Promise<LspToolResult>;
}

function unavailableToolMetadata(): LspToolResultMetadata {
  return {
    status: "unavailable",
    server: null,
    language: null,
    document_version: null,
    stale: false,
    returned_count: 0,
    total: 0,
    truncated: false,
    filtered_count: 0,
    reason_code: "web_runtime_unavailable",
  };
}

function unavailableToolResult(tool: LspToolName): LspToolResult {
  const metadata = unavailableToolMetadata();
  switch (tool) {
    case "find_definition":
      return { metadata, definitions: [] };
    case "find_references":
      return { metadata, references: [] };
    case "get_hover":
      return { metadata, hover: null };
    case "get_diagnostics":
      return { metadata, diagnostics: [] };
    // Type definitions and implementations answer in the definition shape, the same way the
    // desktop side reuses one normalization for all three.
    case "find_type_definition":
    case "find_implementations":
      return { metadata, definitions: [] };
    case "find_workspace_symbols":
    case "get_document_symbols":
      return { metadata, symbols: [] };
    case "find_call_hierarchy":
      return { metadata, relations: [] };
  }
}

export const webLspToolClient: WebLspToolClient = {
  // Freshly built each call, so there is nothing shared to clone defensively against.
  async execute(tool) {
    return unavailableToolResult(tool);
  },
};
