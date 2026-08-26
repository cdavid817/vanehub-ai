import { describe, expect, it } from "vitest";
import { lspTestDescriptors } from "../test/lsp-fixtures";
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

const descriptors = lspTestDescriptors();

const configuration = {
  enabled: true,
  languages: [
    {
      language: "rust",
      enabled: true,
      executableOverride: "C:/tools/rust-analyzer.exe",
      startupArguments: null,
      initializationOptions: { cargo: { allTargets: false } },
    },
    {
      language: "typescript_javascript",
      enabled: false,
      executableOverride: null,
      startupArguments: ["--stdio"],
      initializationOptions: {},
    },
  ],
  descriptors,
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
    expect(normalizeLspServerTestResult(serverTest)).toEqual(serverTest);
    expect(normalizeLspServerStatuses([serverStatus])).toEqual([serverStatus]);
  });

  it("accepts a configuration that names only some of the described languages", () => {
    // The old rule was "exactly these two, in this order". It only held while the set was compiled
    // in; a build that registers a new language has to read configurations written before it.
    const partial = { ...configuration, languages: [configuration.languages[0]] };
    expect(normalizeLspConfiguration(partial)).toEqual(partial);
    expect(normalizeLspConfiguration({ ...configuration, languages: [] }))
      .toEqual({ ...configuration, languages: [] });
  });

  it("rejects configuration for a language the same response does not describe", () => {
    // Such an entry has no label, no server, and no platform applicability to render, so it is
    // refused here rather than surfacing as a control the UI cannot describe.
    expect(() => normalizeLspConfiguration({
      ...configuration,
      languages: [...configuration.languages, {
        language: "go",
        enabled: true,
        executableOverride: null,
        startupArguments: null,
        initializationOptions: {},
      }],
    })).toThrow("invalid LSP response");
    expect(() => normalizeLspConfiguration({
      ...configuration,
      descriptors: [descriptors[0], descriptors[0]],
    })).toThrow("invalid LSP response");
  });

  it("rejects malformed identifiers, duplicates, and non-object language configurations", () => {
    for (const language of ["", "Rust", "c++", "rust.analyzer", " rust", "a".repeat(65)]) {
      expect(() => normalizeLspConfiguration({
        ...configuration,
        languages: [{ ...configuration.languages[0], language }],
        descriptors: [{ ...descriptors[0], language }],
      }), language).toThrow("invalid LSP response");
    }
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

  it("rejects duplicate languages and inconsistent discovery states", () => {
    expect(() => normalizeLspServerDiscoveries([discoveries[0], discoveries[0]]))
      .toThrow("invalid LSP response");
    expect(() => normalizeLspServerDiscoveries([{ ...discoveries[0], server: "Rust Analyzer" }]))
      .toThrow("invalid LSP response");
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
    // A well-formed id the frontend does not recognise is no longer rejectable here -- which
    // languages exist is a backend fact. Only the id's shape is checkable, and the backend refuses
    // an unregistered one. Malformed shapes still fail.
    expect(normalizeLspServerTestInput({ language: "python" })).toEqual({ language: "python" });
    expect(() => normalizeLspServerTestInput({ language: "Python" })).toThrow("invalid LSP response");
    expect(() => normalizeLspServerTestInput({ language: "" })).toThrow("invalid LSP response");
    expect(() => normalizeLspServerTestResult({
      ...serverTest,
      phases: serverTest.phases.slice(0, 3),
    })).toThrow("invalid LSP response");
    expect(() => normalizeLspServerTestResult({
      ...serverTest,
      phases: serverTest.phases.map((phase) => (
        phase.phase === "initialize" ? { ...phase, status: "waiting" } : phase
      )),
    })).toThrow("invalid LSP response");
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
