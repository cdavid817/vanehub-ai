# Design

## D1: Pin at the adapter, not at call sites

`LC_ALL=C` is set once in `GitAdapter`'s shared request builder rather than by each caller: every consumer that classifies git output by text (today `session_queries.rs`, tomorrow anyone) gets the guarantee for free, and no future call site can silently reintroduce the bug. `LC_ALL` sits above `LC_MESSAGES` and `LANG` in libc's precedence, so pinning that single variable defeats any inherited language configuration.

## D2: Callers can still override

The pin is applied before caller-supplied environment in `execute_with_environment`, so an explicit `LC_ALL` from a caller replaces it (BTreeMap insert semantics). No current caller does; the door stays open for a deliberate future need without weakening the default.

## D3: What is deliberately not changed

Classification stays substring-based on English text — introducing exit-code or `rev-parse`-probe classification would be a larger behavioral change than this fix warrants. Filename handling is untouched: git emits paths as bytes and callers already pass `core.quotepath=false`, so `LC_ALL=C` does not alter path round-tripping.

## D4: Test strategy

Deterministic on every host: one test drives a hostile language environment (`LANG`/`LC_MESSAGES`/`LANGUAGE` = zh_CN) through `execute_with_environment` and asserts the English marker still appears; one asserts `execute` yields the English marker in a non-git directory (vacuously green on English hosts, the real regression on zh_CN hosts); one asserts override precedence purely via `ProcessRequest::command()`'s `get_envs()` without spawning git, avoiding dependence on which locales the host has generated.
