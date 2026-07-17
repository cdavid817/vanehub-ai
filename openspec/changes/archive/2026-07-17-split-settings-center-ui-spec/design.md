## Context

This change executes the approved analysis mapping. Every moved Requirement block includes its scenarios and normative terms unchanged.

## Goals / Non-Goals

**Goals:** reduce `settings-center-ui` to shared concerns and establish seven discoverable domain UI specs.

**Non-Goals:** alter behavior, remove requirements, or change archived history.

## Decisions

- Generate delta specs mechanically from the approved source-to-target mapping.
- Use ADDED blocks for focused capabilities and REMOVED blocks in `settings-center-ui` with migration destinations.
- Validate source and target requirement counts before archive.

## Risks / Trade-offs

- [Mapping omission] -> compare every source requirement against retained or moved targets before archive.
