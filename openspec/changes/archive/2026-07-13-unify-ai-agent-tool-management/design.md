## Context

Developers may use several AI coding agents in the same day, each with different launch commands, authentication assumptions, UI surfaces, and session behavior. The current project does not yet define a shared model for registering these agents, switching between them, or deciding whether a workflow should run through a browser session or a native desktop window.

This design introduces a thin orchestration layer that treats each agent as a registered tool with declared capabilities and supported interaction modes. The layer should keep product behavior consistent while allowing each agent integration to preserve its own execution details.

## Goals / Non-Goals

**Goals:**

- Provide one registry model for known AI coding agents such as Claude Code, OpenCode, Codex CLI, and Gemini CLI.
- Let users select an active agent for a development workflow without manually remembering launch details.
- Represent web browser and native desktop window modes as first-class interaction choices.
- Keep agent-specific launch, health check, and session behavior behind adapter boundaries.
- Allow future additions such as credentials, workspace profiles, routing rules, and automation without changing the core capability model.

**Non-Goals:**

- Implementing full automation for every supported agent in this change.
- Replacing the native behavior, authentication, or configuration model of each agent.
- Defining a shared prompt format across all agents.
- Building cross-agent conversation synchronization.
- Guaranteeing feature parity between browser and desktop modes.

## Decisions

### Use Tauri, React, TypeScript, Rust, Playwright, and SQLite

The application will use Tauri for the desktop shell, React and TypeScript for the frontend, Tailwind CSS and shadcn/ui for UI components, Rust Tauri commands for native/backend operations, SQLite for local structured state, and Playwright for browser-mode automation.

Rationale: the product needs a polished desktop app, local process/window integration, browser automation, and strongly typed UI development. Tauri keeps the native layer small while still allowing Rust access to local system capabilities. React and TypeScript fit the agent management UI, SQLite supports durable local registry and preference state, and Playwright gives a proven browser automation path.

Alternative considered: build a pure web app. That would simplify distribution but would not cover native desktop window integration or local CLI process management well.

Alternative considered: build an Electron app. Electron would provide a mature desktop stack, but Tauri is a better fit for a lighter native shell with Rust-backed local operations.

### Use a Registry Plus Adapter Model

Each agent will be represented by a registry entry with stable metadata: identifier, display name, provider, launch type, supported interaction modes, availability status, and capability tags. Runtime behavior will be handled by an agent adapter.

Rationale: registry metadata is needed for discovery and UI selection, while launch and session behavior varies enough that it should remain agent-specific.

Alternative considered: hard-code each agent directly into the UI. This is simpler initially but makes adding tools and handling per-agent differences brittle.

### Isolate Frontend Services From Runtime Backends

The React UI will depend on frontend service interfaces rather than directly calling Tauri commands. Runtime-specific details will live behind adapters: a Tauri adapter for desktop/native execution and a Web adapter for browser preview, mock data, or a future HTTP API backend.

Rationale: the same UI should run in both the Tauri client and a browser page. Keeping `invoke()` and native assumptions out of components allows future expansion to a remote Web backend without rewriting the UI.

Alternative considered: call Tauri commands directly from React components or shared UI helpers. This is fast initially but tightly couples the frontend to the desktop runtime and makes Web deployment harder.

### Treat Interaction Mode as a Runtime Selection

Browser and native desktop modes should be modeled as interaction modes supported by an agent, not as separate agents. A user selects an agent and a compatible mode; the system then routes launch and session control through the correct adapter behavior.

Rationale: Claude Code in one surface and the same agent in another surface should remain the same tool from a user perspective.

Alternative considered: create separate entries such as `codex-browser` and `codex-desktop`. That would simplify launch routing but duplicate configuration and confuse switching.

### Separate Availability Checks From Launch

The system should distinguish whether an agent is registered, installed or reachable, authenticated, and currently launchable. Availability checks should be explicit and should not require starting an interactive session.

Rationale: users need to understand why a tool cannot be selected before a task starts.

Alternative considered: attempt launch first and report failures afterward. That produces poorer UX and makes automated switching harder.

### Keep Session State Agent-Scoped

Session state should be tracked per selected agent and interaction mode. The orchestration layer can expose common lifecycle states such as idle, starting, running, failed, and stopped, but it should not assume the internal session model is identical across agents.

Rationale: CLI agents, browser workflows, and desktop windows have different lifecycle semantics.

Alternative considered: force all agents into one generic session abstraction. That would hide important differences and likely leak implementation details later.

## Risks / Trade-offs

- [Agent behavior differs widely] -> Keep adapters small and require each registry entry to declare only supported modes and capabilities.
- [Desktop control may be OS-specific] -> Isolate native window operations behind an interaction-mode adapter and avoid baking OS assumptions into the registry.
- [Browser automation may require user profile or login state] -> Model authentication and availability separately from launch.
- [Playwright browser automation may increase install size and setup complexity] -> Keep browser mode behind an adapter and document runtime requirements clearly.
- [Tauri native commands can become a catch-all boundary] -> Keep Rust commands coarse-grained and map them to registry, availability, launch, and lifecycle use cases.
- [Frontend can drift into runtime-specific logic] -> Require UI components to use service interfaces and keep Tauri/Web details inside runtime adapters.
- [SQLite schema may change as registry needs evolve] -> Start with a minimal schema and add migrations when persistent state expands.
- [Too much abstraction before implementation] -> Start with minimal registry fields and lifecycle states required for switching and launch.
- [Users may expect mode parity] -> Surface unsupported modes clearly in the registry and selection UI.

## Migration Plan

No existing specs or implementation need migration. Introduce the registry, switching, and interaction-mode specs first, then implement adapters incrementally for each supported agent.

Rollback is straightforward while the feature is additive: remove the new registry and switching UI paths, leaving existing agent-specific workflows untouched.

## Open Questions

- Should agent registry entries be static configuration, user-editable configuration, or both?
- Which agents must be supported in the first implementation slice?
- Should browser mode use an embedded browser, external browser, or both?
- Which operating systems are required for native desktop window mode?
- How should credentials and authentication state be represented without storing secrets in the registry?
