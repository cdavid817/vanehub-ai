## Why

Utility Skills are classified and discoverable by the effective Skill runtime but remain unavailable because VaneHub AI has no bounded child-Agent delegation path. Activating them requires a permission-aware execution model that preserves parent context, cancellation, observability, and user control without injecting Utility instructions as conversational roles.

## What Changes

- Add a fixed-schema `delegate_skill` tool for native API Agents to invoke an enabled, bound, trusted, effective Utility Skill by canonical id or alias.
- Execute each accepted delegation in an isolated child-Agent attempt using the parent's provider interface and model snapshot, the Utility's effective Overlay-applied instructions, a bounded task contract, and explicitly selected context rather than the full parent conversation.
- Activate parent-principal relationships in the unified permission system for Utility child principals, with explicit-Deny-first parent ceilings and independent evaluation of every child tool action.
- Require approval to start a delegation by default, show the Utility, task, capability ceiling, parent Agent, workspace, and risk, and keep remembered approval scopes within existing permission policy semantics.
- Restrict child tools to the intersection of platform limits, parent permission mode, Utility declarations, trust, and policy; default undeclared Utility capabilities to read-only.
- Prohibit recursive delegation, arbitrary script loading, MCP tools by default, hidden context expansion, and direct access to parent secrets or complete transcripts.
- Bound depth, active children, attempts, model/tool rounds, context, output, duration, and cancellation; return a structured summary and evidence references rather than an unbounded child transcript.
- Persist Utility usage, delegation attempts, child results, permission decisions, and correlated execution topology through existing message, permission, session, and observability boundaries.
- Add Utility eligibility, declared capability, assignment, unavailable-reason, usage, and delegation-history presentation to Skills settings, and render delegated work as a collapsible child activity in chat.
- Provide matching Web/mock service behavior without native model, process, or filesystem side effects.
- Keep direct CLI-originated invocation out of scope until an individual CLI adapter can call VaneHub's native delegation contract; CLI Agents may consume resulting project changes but do not receive the native `delegate_skill` tool in this change.

## Capabilities

### New Capabilities

- `delegated-utility-skills`: Defines Utility eligibility, delegation contracts, isolated child execution, context and tool ceilings, recursion prevention, bounds, results, cancellation, persistence, and runtime scope.

### Modified Capabilities

- `skill-management`: Adds Utility delegation metadata, API-Agent assignment eligibility, capability declarations, unavailable reasons, usage, and history summaries.
- `agent-tool-execution`: Adds the fixed `delegate_skill` tool and routes child tool calls through the existing bounded tool and approval loop.
- `agent-chat-configuration`: Defines how parent permission modes constrain Utility delegation and keeps delegation unavailable in Plan mode unless the Utility is read-only.
- `permissions-core`: Activates parent principals for bounded Utility delegation and enforces parent-chain permission ceilings.
- `permissions-approval`: Adds delegation context to start and child-action approval presentation and keeps pending approvals cancellable with their parent generation.
- `agent-execution-observability`: Requires native parent/child spans, delegation and attempt identities, bounded metadata, and cancellation outcomes for Utility execution.
- `chat-experience`: Displays live and persisted delegated Utility activity and structured results without merging child output into the parent as a separate speaker.
- `settings-skill-management-ui`: Presents Utility availability, declared and effective capabilities, assignments, usage, delegation history, and security restrictions.

## Impact

- Depends on `establish-effective-skill-runtime` and `add-skill-overlay-governance`; implementation must sequence both first so delegation uses the same effective identity, trust, instructions, resources, pin state, and usage counters.
- Affects native API-agent tool catalog and execution loop, provider adapters, session/generation lifecycle, permissions principals and evaluation, approval broker, execution observability, SQLite persistence, unified logging, and cancellation.
- Affects shared frontend service contracts, Tauri and Web/mock adapters, chat tool rendering, approval presentation, Skills settings, and localization in desktop and Web runtimes.
- Does not add a new state-management library, direct React-to-Tauri calls, dynamic Skill script execution, recursive agents, a second permission engine, or native delegation tools inside third-party CLI runtimes.
