# Proposed Instance-Level Delta

Standard OpenSpec deltas address Requirements by name. Because the invalid source has two identical instances of each affected name, `REMOVED Requirements` cannot safely express which instance to delete.

The reviewable delta is therefore the exact-instance deletion described in `../diff.md`, backed by the one-to-one hashes and coverage mapping. This change intentionally declares no semantic capability delta and sets `skip_specs: true`.
