## Why

Full strict validation is blocked because two main specifications contain nine pairs of byte-identical Requirement blocks. Requirement names must be unique so future delta application cannot target or replace the wrong instance.

## What Changes

- Remove only the later copy of each byte-identical duplicate Requirement from `agent-skill-injection` and `skill-management`.
- Preserve every unique Requirement, Scenario, `SHALL`, and `MUST` statement unchanged.
- Record instance-level mappings, hashes, coverage, and the exact proposed diff before applying the cleanup.
- Treat this as a specification-integrity repair with no product behavior or capability change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. The retained requirements are semantically and textually identical to the removed duplicate copies, so `.openspec.yaml` sets `skip_specs: true`.

## Impact

- Main specifications only: `agent-skill-injection` and `skill-management` after review approval.
- No frontend, desktop runtime, Web runtime, adapter boundary, database, or API changes.
- Restores `openspec validate --specs --strict` once the reviewed cleanup is applied.
