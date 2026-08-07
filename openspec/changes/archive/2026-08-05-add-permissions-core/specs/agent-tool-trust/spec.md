## REMOVED Requirements

### Requirement: Persistent per-agent tool trust setting
**Reason**: Superseded by policy template assignment on the unified permission principal.
**Migration**: See `permissions-core`'s "Legacy per-agent trust flag migrates to an equivalent policy assignment" (one-time migration) and "Policy templates provide named, pre-built policy sets" (ongoing replacement mechanism).

### Requirement: A trusted agent's shell and file-write calls skip approval
**Reason**: Superseded by the `trusted` policy template resolving `shell.exec`/`file.write` to `Allow`.
**Migration**: See `permissions-core`'s "Policy templates provide named, pre-built policy sets" and `agent-tool-execution`'s modified "Risk-tiered tool approval".

### Requirement: MCP tool calls remain unconditionally gated regardless of trust
**Reason**: Superseded by a stronger, template-proof invariant — this guarantee no longer depends on the trust setting's absence, it cannot be weakened by any template including `yolo`.
**Migration**: See `permissions-core`'s "MCP-sourced actions are floored at Ask regardless of template or policy".

### Requirement: Plan mode overrides the trust setting unconditionally
**Reason**: Plan Mode's enforcement was, and remains, independent of this capability's trust flag — it lives entirely in `agent-chat-configuration` and is unaffected by this change.
**Migration**: No action needed. See `agent-chat-configuration`'s existing "Plan mode restricts a native API agent to read-only tools" requirement, which continues to apply unchanged regardless of which policy template a principal is assigned.

### Requirement: Enabling the trust setting requires explicit confirmation
**Reason**: Superseded by a generalized confirmation requirement covering every trust-increasing template assignment, not only a single boolean flag.
**Migration**: See `permissions-approval`'s "Increasing a principal's trust requires explicit confirmation; decreasing it does not".

### Requirement: Web runtime trust-setting parity
**Reason**: Superseded by the new capabilities' own Web/mock parity requirements.
**Migration**: See `permissions-core`'s "Web runtime permission evaluation parity" and `permissions-approval`'s "Web runtime approval parity".
