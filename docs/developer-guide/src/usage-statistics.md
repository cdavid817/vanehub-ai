# Usage statistics

VaneHub records per-response usage for VaneHub-managed assistant responses and summarizes it in the settings center. There is no external billing integration; this is first-version local usage accounting.

## Reported tokens vs estimated characters

Two categories are kept strictly separate:

- **Reported tokens** — fresh-input, output, cache-read, cache-creation, and total, taken from provider-reported usage. Reported total equals the sum of those four categories.
- **Estimated characters** — input, output, and total, derived from character counting when provider-reported usage is unavailable. Estimated characters are never added to any reported token total.

Statistics also return reported/estimated/total counted response counts, counted sessions, daily trend points, per-Agent breakdown rows keyed by stable Agent id, and the percentage of counted responses backed by reported usage. A range with no records returns zero-valued totals and empty arrays instead of failing.

## Time ranges

Supported ranges are today, last seven days, last thirty days, and all time, computed on the active runtime's user-local calendar.

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the spec.

- [openspec/specs/usage-statistics](../../../openspec/specs/usage-statistics/spec.md)

Usage persistence sits in the `sessions` bounded context; see [Native bounded contexts](native-contexts.md).
