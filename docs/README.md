# Documentation map

**Which version?** Documentation on `main` describes the next, unreleased version; the current stable download is **v1.5.0** ([release notes](https://github.com/cdavid817/vanehub-ai/releases/tag/v1.5.0) · [docs as released](https://github.com/cdavid817/vanehub-ai/tree/v1.5.0/docs)). Status vocabulary (`stable`, `main` / unreleased, fixture-qualified, live-qualified, experimental, legacy, planned) is defined in [terminology](reference/terminology.md); a "supported" or "delivered" claim must map to a main spec or a release manifest, never only to an active OpenSpec change.

Where each documentation set lives, who it serves, and how it is maintained. Entries marked *generated* must never be edited by hand — change the canonical source and regenerate. Entries marked *historical* describe a past revision and are not current product claims.

| Area | Audience | Type | Maintained from |
| --- | --- | --- | --- |
| [User guide](user-guide/en/src/index.md) ([简体中文](user-guide/zh-CN/src/index.md)) | People using VaneHub AI | Authored narrative | The shipped UI and desktop verification; both languages share one chapter set |
| [Developer guide](developer-guide/src/index.md) ([简体中文](developer-guide/zh-CN/src/index.md)) | Contributors and maintainers | Authored narrative | Source-verified; authoritative requirements stay in the specs |
| [Agent infrastructure](agent-infrastructure/README.md) | Learners of the underlying technologies | External-technology tutorials | Explains protocols and patterns (MCP, LSP, RAG, …) in themselves — **not** a claim that VaneHub implements everything described |
| [Agent capability matrix](reference/agents/README.md) | Everyone | Curated source + *generated* matrix | `capability-matrix.md` is generated from `capability-matrix.source.json` (`npm run docs:agents:generate`) after cross-checking the code registries; README and the guides cite it instead of repeating agent counts |
| [Terminology](reference/terminology.md) | Everyone | Authored reference | One meaning per word: transports, the status vocabulary (stable / main / fixture-qualified / live-qualified / legacy / planned), and supported vs managed vs verified |
| [CLI reference](reference/cli/README.md) | Contributors | Mixed authored + *generated* | `parameter-matrix.md` is generated from the parameter catalog (`npm run docs:matrix:generate`); the built-in CLI reference is a point-in-time audit |
| [Internal provider adapter contract](provider-sdk/contract.md) | Contributors adding a provider | Normative internal contract | Providers are statically compiled in; this is not a shipped third-party plugin SDK |
| [Model provider catalog](model-providers.md) | Users and contributors | Authored reference | Mirrors the built-in provider catalog shipped with the app |
| [Build performance](build-performance.md) · [Runtime performance budgets](runtime-performance-budgets.md) | Contributors and release engineers | Runbook + measurements | Current policy is stated separately from dated measurement records |
| [Release signing](release-signing.md) · [Desktop release verification](desktop-release-verification.md) | Release engineers | Runbook / checklist | The signing phase and verification statuses here are authoritative for release claims |
| [CLI Agent global configuration](cli-agent-global-configuration.md) | Users and contributors | Authored reference | Describes the managed fields VaneHub writes into each CLI's own configuration |
| [OpenSpec main specifications](../openspec/specs/) | Everyone | Normative | The single source of truth for confirmed behavior |
| [Active OpenSpec changes](../openspec/changes/) | Contributors | Proposals in flight | Proposed or in-progress work — never evidence that a capability shipped |
| `../openspec/changes/archive/` | Historians | Immutable archive | Never edited; consult `archive-index.json` first |
| [VaneHub-AI-技术架构深度解析](VaneHub-AI-技术架构深度解析.md) | Historians | *Historical* survey | A snapshot of commit `bb3d28d8` (2026-08); read the guides and specs for current state |
| `../src-tauri/ARCHITECTURE.md` | Contributors | Authored inventory | The native module and decision inventory, copied into the built developer guide as a reference page |

Generated build output lands in `.docs-build/` (`npm run docs:build`); screenshots are inventoried in `user-guide/screenshots.json`, where every entry declares whether it came from the Web/mock runtime or a reviewed desktop run.
