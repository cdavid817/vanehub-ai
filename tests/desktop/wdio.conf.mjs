import process from "node:process";
import { pathWithoutCompetingAgents, prepareManagedCliFixtures } from "./wdio-cli-fixture.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";
import { requiredSpecFiles } from "./spec-manifest.mjs";

// The core contract, for a caller that wants the launch/IPC/shutdown check without the sweep.
const coreSmokeOnly = process.env.VANEHUB_DESKTOP_CORE_SMOKE === "1";
// One named required spec, for isolating a failure. Diagnosis only: it selects from the required
// set and cannot introduce a spec the manifest does not classify.
const onlySpec = process.env.VANEHUB_DESKTOP_SPEC;

function selectedSpecs() {
  if (coreSmokeOnly) return ["smoke.e2e.mjs"];
  if (!onlySpec) return requiredSpecFiles();
  const match = requiredSpecFiles().filter((spec) => spec === onlySpec);
  if (match.length === 0) throw new Error(`VANEHUB_DESKTOP_SPEC names no required spec: ${onlySpec}`);
  return match;
}

// Fixture Agents ahead of the inherited PATH, so every managed Agent resolves to the stub whether
// or not the machine has the real one. This is what makes the gate a gate: it used to need a real
// `codex` installed, which no hosted runner has, so the sweep could only pass on a developer's
// laptop and failed identically on Windows, macOS and Linux CI.
const agentFixtureDir = await prepareManagedCliFixtures();
// First is not enough: a real installation left reachable elsewhere on PATH makes two installations
// from different sources, and the launch resolver refuses that as PATH shadowing. The gate has to
// be independent of what the developer happens to have installed.
const fixturePath = await pathWithoutCompetingAgents(agentFixtureDir);

export const config = await createDesktopConfig({
  specDirectory: "specs",
  specFiles: selectedSpecs(),
  environment: {
    PATH: fixturePath,
    VANEHUB_CLI_FIXTURE_ROOT: agentFixtureDir,
  },
});
