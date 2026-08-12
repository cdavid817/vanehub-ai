## Context

See `proposal.md` for motivation. The native API adapter classifies each completed tool call immediately before the shared permissions service evaluates it. The fixed OnePiece catalog has grown beyond the original `shell`, `file`, and `remember` tools, but the classifier did not grow with it, so legitimate fixed tools reach the intentional unknown-tool fallback.

Existing permission actions already express the required policy behavior: `file.read` is always allowed, `file.write` follows the assigned template, `shell.exec` follows the assigned template, `memory.write` is always allowed, and `mcp.tool` is always floored at `Ask`.

## Goals / Non-Goals

**Goals:**

- Make the classifier exhaustive over every fixed or conditionally registered OnePiece built-in tool.
- Preserve path-level resources for tools that target one file and workspace-level resources for workspace-wide searches.
- Lock both recognized mappings and the unknown/MCP fail-closed boundaries with focused tests.

**Non-Goals:**

- Changing policy-template semantics, remembered-grant precedence, Plan Mode, or MCP approval behavior.
- Adding new permission-domain action identifiers or database migrations.
- Changing frontend rendering or Web/mock behavior.

## Decisions

### Reuse established permission actions

Map `grep`, `glob`, and `search_code` to `file.read` on the workspace resource; map `recall` to `file.read` on the memory resource; and map `edit` to `file.write` on its requested path. The existing `shell`, `file`, `remember`, LSP, and MCP mappings remain intact.

This restores the pre-unification risk contract without expanding the permission vocabulary. Introducing `code.search` or `memory.read` actions was considered, but would require new template rules and broaden a targeted compatibility fix.

### Test the complete catalog as one contract

Use a table-driven unit test that enumerates all built-in tool constants, representative inputs, and expected action/resource pairs. Keep separate assertions for MCP and an invented tool name because those are deliberate security boundaries rather than built-in mappings.

Testing only the reported `glob` case was rejected because the same omission affects `grep`, `edit`, `recall`, and `search_code`, and future catalog additions could otherwise repeat the regression.

## Risks / Trade-offs

- [Risk] A future built-in tool can be added without updating the classifier. → Keep the full catalog mapping test visibly grouped with the classifier and document the exhaustive invariant.
- [Risk] Reusing `file.read` for memory/code retrieval is less specific in audit output. → Preserve a meaningful resource (`memory` or `workspace`) and avoid a wider permission-domain/schema change in this bug fix.
- [Risk] Making `edit` follow `file.write` changes current accidental prompts. → This is the intended and already specified trusted/readonly/standard behavior; regression tests cover the mapping rather than bypassing policy evaluation.
