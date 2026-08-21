# Design references

Research date: 2026-08-22.

This file records the external and repository references used to derive the change. The requirements in the delta specs remain authoritative; references are informative and must not be used to bypass VaneHub's current contracts.

## VaneHub AI current contracts

* Repository and architecture overview: https://github.com/cdavid817/vanehub-ai
* Repository implementation rules: https://github.com/cdavid817/vanehub-ai/blob/main/AGENTS.md
* OpenSpec project rules: https://github.com/cdavid817/vanehub-ai/blob/main/openspec/project.md
* Active changes: https://github.com/cdavid817/vanehub-ai/tree/main/openspec/changes
* Tooling context: https://github.com/cdavid817/vanehub-ai/tree/main/src-tauri/src/contexts/tooling
* Current main specs: https://github.com/cdavid817/vanehub-ai/tree/main/openspec/specs
* Remote Skill Registry change: https://github.com/cdavid817/vanehub-ai/tree/main/openspec/changes/add-remote-skill-registry-and-supply-chain-governance

Applied conclusions:

* Preserve React service/Tauri adapter/Web adapter boundaries.
* Keep Rust bounded-context ownership and use published APIs/ports.
* Do not duplicate the active Skill Registry, Skill configuration, Skill Tool sandbox, Permissions PDP, MCP runtime, or IM connector runtime.
* Keep production TS/TSX files at or below 300 physical lines and follow the exact repository validation commands.

## Visual Studio Code extension architecture

* Extension Host: https://code.visualstudio.com/api/advanced-topics/extension-host
* Contribution Points: https://code.visualstudio.com/api/references/contribution-points
* Activation Events: https://code.visualstudio.com/api/references/activation-events
* Extension Manifest: https://code.visualstudio.com/api/references/extension-manifest

Applied conclusions:

* Use declarative contribution points in a manifest.
* Index contributions before executable activation.
* Activate lazily from explicit events.
* Isolate extension execution so startup/UI stability is not coupled to extension code.

## Claude Code plugins and Hooks

* Plugins overview: https://code.claude.com/docs/en/plugins
* Plugins reference: https://code.claude.com/docs/en/plugins-reference
* Features overview: https://code.claude.com/docs/en/features-overview
* Hooks reference: https://code.claude.com/docs/en/hooks
* Hooks guide: https://code.claude.com/docs/en/hooks-guide
* Plugin marketplaces: https://code.claude.com/docs/en/plugin-marketplaces
* Skills: https://code.claude.com/docs/en/skills

Applied conclusions:

* Treat a plugin as a packaging layer that can bundle/refer to multiple capability types.
* Keep Hooks as explicit lifecycle interception points with typed event input and decisions.
* Version the Claude compatibility catalog rather than hard-code an assumed permanent event count.
* Namespace contributed capabilities to avoid collisions.

## Dify plugin packaging and distribution

* Plugin overview: https://docs.dify.ai/en/develop-plugin/getting-started/getting-started-dify-plugin
* Local `.difypkg` packaging: https://docs.dify.ai/en/develop-plugin/publishing/marketplace-listing/release-by-file
* GitHub distribution: https://docs.dify.ai/en/develop-plugin/publishing/marketplace-listing/release-to-individual-github-repo
* Agent Strategy plugin: https://docs.dify.ai/en/develop-plugin/dev-guides-and-walkthroughs/agent-strategy-plugin

Applied conclusions:

* Use a portable package with a manifest and install-time permission review.
* Treat signature verification and requested authority as separate gates.
* Support local developer packaging without equating local installation with production trust.

## Model Context Protocol

* Tools specification (current version at research time): https://modelcontextprotocol.io/specification/2026-07-28/server/tools
* Authorization specification: https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization
* Transports: https://modelcontextprotocol.io/specification/2026-07-28/basic/transports
* Security best practices: https://modelcontextprotocol.io/specification/2026-07-28/basic/security_best_practices

Applied conclusions:

* Keep users able to inspect exposed tools and deny invocation.
* Preserve human-in-the-loop approval for sensitive tool operations.
* Keep MCP credentials/authorization in the MCP/credential subsystem rather than extension package content.
* Apply origin/audience binding, PKCE, and no token passthrough to remote connector authorization.

## Tauri security boundaries

* Security overview: https://v2.tauri.app/security/
* Capabilities: https://v2.tauri.app/security/capabilities/
* Permissions: https://v2.tauri.app/security/permissions/
* Command scopes: https://v2.tauri.app/security/scope/
* Runtime Authority: https://v2.tauri.app/security/runtime-authority/

Applied conclusions:

* Do not treat arbitrary code inside the application process as constrained merely because Tauri command permissions exist.
* Keep the WebView behind explicit IPC/service boundaries.
* Apply capability and scope concepts to extension host calls, while recognizing that sidecar ambient OS access needs a separate sandbox provider.

## Interpretation rule

Where an external project differs from VaneHub's current architecture, this change uses the external project as a design reference rather than a compatibility target. VaneHub's main specs, current approved changes, security floors, and repository rules take precedence.
