# Agent reference

The single source of truth for what each agent gets from VaneHub AI. README, the user guide, and the developer guide link here instead of repeating agent counts or per-agent capability claims.

| File | Type | Purpose |
| --- | --- | --- |
| [capability-matrix.md](capability-matrix.md) | *generated* | One row per agent: release status, transport, install source, auth probe, provider configuration, permission projection, MCP relay, evaluation eligibility, usage reporting, qualification evidence, upstream source and audited version |
| `capability-matrix.json` | *generated* | The same rows as a machine-readable snapshot |
| `capability-matrix.source.json` | curated | The only file to edit when an agent's documented capability changes |

## Regenerate

```bash
npm run docs:agents:generate
```

`npm run docs:agents:check` (part of `npm run docs:check`) regenerates in memory and fails when the committed matrix is stale, so a hand edit to the generated files never survives CI.

## What the generator refuses

The source is curated because transport, install sources, the relay allowlist, and usage capability are Rust literals with no JSON projection. Curation is safe only because every value the code can contradict is re-read from the code at generation time. `scripts/agent-capability-matrix-validate.mjs` fails, naming the agent and field, when:

- the CLI id set or order differs from `managedCliAgentIds` in `src/types/agent.ts`;
- the agents marked `relayed` differ from the `matches!` allowlist in `src-tauri/src/bootstrap/managed_mcp_relay.rs`;
- the runtime-flag permission set differs from `RUNTIME_POLICY_AGENT_IDS`, or the twelve differ from `POLICY_TEMPLATE_GOVERNED_AGENT_IDS`, in `providers/invocation.rs`;
- the agents with a managed provider configuration differ from `SUPPORTED_AGENT_IDS` in `tooling/cli_config/domain/mod.rs`;
- an agent marked `legacy` is not tagged `legacy` in the registry seed (`agent_runtime/infrastructure/schema.rs`), or an evaluation flag contradicts that tag;
- usage reporting or transport differ from `ProviderUsageCapability` / `ProviderTransport` in `providers/definitions.rs`;
- an agent in `catalog.v2.json` has no row;
- an upstream version or review date is curated rather than read from `src/contracts/fixtures/cli-parameter-source-audit.json`.

Adding an agent therefore fails CI until its release status, permission, MCP, evaluation, and documentation metadata are all filled in.

## Upstream sources and freshness

The matrix's last section lists, per CLI, the official documentation URL, the binary version the parameter catalog was audited against, and the review date, all read from `cli-parameter-source-audit.json`. Treat installation scripts, login flows, and quotas as volatile: user-guide pages describe stable product behaviour and link here for the upstream detail. Confirm any `curl | sh` installer against the official source before running it; the in-app installer only drives vendor scripts whose hosts are allowlisted in the tooling registry.

Live-qualification records are deliberately sparse: a platform appears only when a real-CLI run on that operating system is recorded in the repository (an OpenSpec change's tasks, `docs/desktop-release-verification.md`, or a desktop test evidence directory). A named live test mode is not a record of a run.
