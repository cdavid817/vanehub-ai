import { pathWithoutCompetingAgents, prepareManagedCliFixtures } from "./wdio-cli-fixture.mjs";
import { createDesktopConfig } from "./wdio-shared.mjs";

// The managed fixture Agents, the way the smoke layer wires them, rather than the dialogs layer's
// bare fixture PATH: this layer creates a second session after the initial CLI refresh has
// finished, and from then on the launch target comes from the stored discovery snapshot. Only
// the managed fixture root is something that snapshot recognizes; a stub merely prepended to
// PATH is found by the pre-scan live lookup and refused once the scan has concluded. Competing
// real Agents are removed from PATH for the same reason as in the smoke layer: a real one would
// start a real model session in the worktree under test.
const agentFixtureDir = await prepareManagedCliFixtures();
const fixturePath = await pathWithoutCompetingAgents(agentFixtureDir);

export const config = await createDesktopConfig({
  specDirectory: "specs-session-deletion",
  environment: {
    PATH: fixturePath,
    VANEHUB_CLI_FIXTURE_ROOT: agentFixtureDir,
  },
});
