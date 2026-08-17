# Skill management

Skills are on-demand capability bundles attached to an Agent. The native side owns discovery, mounting, drift reconciliation, and Agent binding; the frontend never touches the filesystem directly.

## Dual scopes

Skills are managed in two isolated scopes:

- **`global`** — stored under the fixed user-home VaneHub Skill directory.
- **`workspace`** — stored under the current workspace directory's VaneHub Skill directory.

The same Skill id may exist in both scopes; their enabled state, source path, Agent bindings, drift state, and deletion are managed independently.

## SKILL.md contract

Every Skill is defined by a `SKILL.md` file with a fixed frontmatter schema: `id`, `name`, `description`, `category`, `version`, and optional `triggers`. The `id` is immutable after creation. A registry record pointing at a directory with no `SKILL.md` (or invalid frontmatter) is reported as drift, not treated as healthy.

## Where the design lives

This chapter orients contributors. The authoritative requirements — dual scopes, the `SKILL.md` schema, drift, Agent binding, and the built-in seeding/reconciliation contract — live in the specs.

- [openspec/specs/skill-management](../../../openspec/specs/skill-management/spec.md)
- [openspec/specs/agent-skill-injection](../../../openspec/specs/agent-skill-injection/spec.md)

The `tooling` bounded context that owns this is described in [Native bounded contexts](native-contexts.md).
