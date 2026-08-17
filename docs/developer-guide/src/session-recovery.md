# Session recovery

Managed generations are durable: a session can be safely resumed after a crash, an interrupted tool-use loop, or a structural inconsistency, without losing evidence or double-executing work. Recovery status is tracked **independently** from the session's lifecycle state.

## Recovery status is orthogonal to lifecycle

Every durable session carries a recovery status of `clean`, `reconciling`, `action_required`, or `quarantined`:

- A `failed` session with recovery `clean` and no active run still accepts a new message.
- An `idle` session with recovery `action_required` rejects new generation work on every managed submission path until an allowed recovery action succeeds.
- A `quarantined` session — a stable structural inconsistency that cannot be reconciled without risking evidence loss — stays readable and exportable but rejects generation or mutation that depends on the inconsistent state.

## Durable execution identity and ownership

Every accepted managed generation has one stable execution run id correlated with its session and persisted messages. Before provider or CLI execution begins, the session atomically claims **at most one** active execution run. A competing claim while a run is active is rejected without starting work.

## Where the design lives

This chapter orients contributors. The authoritative requirements — recovery status, durable execution identity and ownership, and the allowed recovery actions — live in the spec.

- [openspec/specs/session-recovery](../../../openspec/specs/session-recovery/spec.md)

Session durability sits in the `sessions` bounded context; see [Native bounded contexts](native-contexts.md).
