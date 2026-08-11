import { describe, expect, it } from "vitest";
import {
  normalizeLspConfiguration,
  normalizeLspServerDiscoveries,
  normalizeLspServerStatuses,
  normalizeLspServerTestInput,
  normalizeLspServerTestResult,
  normalizeLspWorkspaceTrust,
  normalizeLspWorkspaceTrustList,
  normalizeLspWorkspaceTrustUpdate,
} from "./lsp-contract";

const configuration = {
  enabled: true,
  languages: [
    {
      language: "rust",
      enabled: true,
      executableOverride: "C:/tools/rust-analyzer.exe",
      initializationOptions: { cargo: { allTargets: false } },
    },
    {
      language: "typescript_javascript",
      enabled: false,
      executableOverride: null,
      initializationOptions: {},
    },
  ],
};

const capabilities = {
  positionEncoding: "utf16",
  documentSync: "incremental",
  definition: true,
  references: true,
  hover: true,
  diagnostics: true,
};

const discoveries = [
  {
    language: "rust",
    server: "rust_analyzer",
    availability: "available",
    executablePath: "C:/tools/rust-analyzer.exe",
    arguments: [],
    reasonCode: null,
  },
  {
    language: "typescript_javascript",
    server: "typescript_language_server",
    availability: "unavailable",
    executablePath: null,
    arguments: ["--stdio"],
    reasonCode: "executable_not_found",
  },
];

const serverTest = {
  server: "rust_analyzer",
  phases: [
    { phase: "discovery", status: "succeeded", reasonCode: null },
    { phase: "spawn", status: "succeeded", reasonCode: null },
    { phase: "initialize", status: "succeeded", reasonCode: null },
    { phase: "cleanup", status: "succeeded", reasonCode: null },
  ],
  negotiatedCapabilities: capabilities,
};

const serverStatus = {
  language: "rust",
  server: "rust_analyzer",
  relativeProjectRoot: ".",
  state: "ready",
  restartCount: 1,
  lastResponseAt: "2026-08-10T12:34:56.123Z",
  diagnosticCount: 2,
  reasonCode: null,
  negotiatedCapabilities: capabilities,
};

describe("LSP runtime contracts", () => {
  it("normalizes every valid configuration, trust, discovery, test, and status payload", () => {
    expect(normalizeLspConfiguration(configuration)).toEqual(configuration);
    expect(normalizeLspWorkspaceTrustList([{
      canonicalRoot: "D:/code/app",
      trusted: true,
      revision: 2,
    }])).toEqual([{
      canonicalRoot: "D:/code/app",
      trusted: true,
      revision: 2,
    }]);
    expect(normalizeLspWorkspaceTrust({
      canonicalRoot: "D:/code/app",
      trusted: true,
      revision: 2,
    }).revision).toBe(2);
    expect(normalizeLspServerDiscoveries(discoveries)).toEqual(discoveries);
    expect(normalizeLspServerTestResult(serverTest, "rust")).toEqual(serverTest);
    expect(normalizeLspServerStatuses([serverStatus])).toEqual([serverStatus]);
  });

  it("rejects incomplete, duplicate, and non-object language configurations", () => {
    expect(() => normalizeLspConfiguration({
      ...configuration,
      languages: [configuration.languages[0]],
    })).toThrow("invalid LSP response");
    expect(() => normalizeLspConfiguration({
      ...configuration,
      languages: [configuration.languages[0], configuration.languages[0]],
    })).toThrow("invalid LSP response");
    expect(() => normalizeLspConfiguration({
      ...configuration,
      languages: [
        { ...configuration.languages[0], initializationOptions: [] },
        configuration.languages[1],
      ],
    })).toThrow("invalid LSP response");
    expect(() => normalizeLspConfiguration({
      ...configuration,
      languages: [
        {
          ...configuration.languages[0],
          initializationOptions: { payload: "x".repeat(33 * 1024) },
        },
        configuration.languages[1],
      ],
    })).toThrow("invalid LSP response");
  });

  it("rejects malformed trust records and validates mutation inputs", () => {
    expect(normalizeLspWorkspaceTrustUpdate({
      canonicalRoot: "D:/code/app",
      trusted: false,
    })).toEqual({ canonicalRoot: "D:/code/app", trusted: false });
    expect(() => normalizeLspWorkspaceTrustList([{
      canonicalRoot: "D:/code/app",
      trusted: true,
      revision: -1,
    }])).toThrow("invalid LSP response");
    expect(() => normalizeLspWorkspaceTrustList([
      { canonicalRoot: "D:/code/app", trusted: true, revision: 1 },
      { canonicalRoot: "D:/code/app", trusted: false, revision: 2 },
    ])).toThrow("invalid LSP response");
    expect(() => normalizeLspWorkspaceTrustUpdate({
      canonicalRoot: "",
      trusted: true,
    })).toThrow("invalid LSP response");
  });

  it("rejects mismatched server identities and inconsistent discovery states", () => {
    expect(() => normalizeLspServerDiscoveries([
      { ...discoveries[0], server: "typescript_language_server" },
      discoveries[1],
    ])).toThrow("invalid LSP response");
    expect(() => normalizeLspServerDiscoveries([
      { ...discoveries[0], executablePath: null },
      discoveries[1],
    ])).toThrow("invalid LSP response");
    expect(() => normalizeLspServerDiscoveries([
      discoveries[0],
      { ...discoveries[1], reasonCode: "raw-process-error" },
    ])).toThrow("invalid LSP response");
  });

  it("requires all isolated-test phases and validates the test input", () => {
    expect(normalizeLspServerTestInput({ language: "rust" })).toEqual({ language: "rust" });
    expect(() => normalizeLspServerTestInput({ language: "python" })).toThrow("invalid LSP response");
    expect(() => normalizeLspServerTestResult({
      ...serverTest,
      phases: serverTest.phases.slice(0, 3),
    }, "rust")).toThrow("invalid LSP response");
    expect(() => normalizeLspServerTestResult({
      ...serverTest,
      phases: serverTest.phases.map((phase) => (
        phase.phase === "initialize" ? { ...phase, status: "waiting" } : phase
      )),
    }, "rust")).toThrow("invalid LSP response");
  });

  it("rejects invalid status timestamps, counts, states, and capabilities", () => {
    expect(() => normalizeLspServerStatuses([{ ...serverStatus, state: "indexing" }]))
      .toThrow("invalid LSP response");
    expect(() => normalizeLspServerStatuses([{ ...serverStatus, restartCount: -1 }]))
      .toThrow("invalid LSP response");
    expect(() => normalizeLspServerStatuses([{ ...serverStatus, lastResponseAt: "tomorrow" }]))
      .toThrow("invalid LSP response");
    expect(() => normalizeLspServerStatuses([{
      ...serverStatus,
      negotiatedCapabilities: { ...capabilities, definition: "yes" },
    }])).toThrow("invalid LSP response");
  });
});
