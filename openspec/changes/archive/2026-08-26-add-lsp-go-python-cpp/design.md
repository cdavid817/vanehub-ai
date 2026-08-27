## Context

See `proposal.md` — Why. What `extend-lsp-language-registry` left in place:

- `LANGUAGE_DEFINITIONS` in `domain/registry.rs` declares each language's executables in preference order, root markers, extension-to-`languageId` mappings, default startup arguments, platform applicability, and server-test fixture files.
- `registry_tests.rs` fails a build whose entry is missing any of that, and asserts ids and extensions are unique across the table.
- Discovery already walks `executables` in order and reports which candidate it selected.
- `ProjectRootResolver::resolve` walks ancestors testing `current.join(marker).is_file()` for each of the language's markers, stopping at the canonical session workspace, and **falls back to the session workspace root when no marker is found anywhere**.
- No frontend file names a language. Settings render one card per backend descriptor.

That last fallback is the one existing behavior this change has to make conditional.

## Goals / Non-Goals

**Goals:**

- Three languages added as registry entries plus locale strings, with no new mechanism where the existing one suffices.
- A root-detection rule for C/C++ that refuses rather than guesses, because `clangd` without a compilation database answers confidently and wrongly.
- Multi-marker languages tested for the behavior that is actually observable: each marker identifies a root on its own, and proximity decides between directories.

**Non-Goals:**

- Installing servers. Discovery stays "found on `PATH` or pointed at by an absolute override"; managed installation is `extract-managed-tool-installation`.
- Java. `jdtls` does not fit the executable-plus-arguments launch shape and is `add-lsp-java-jdtls`.
- Any new frontend component. If one turns out to be needed, that is a defect in the previous change, not a task in this one.

## Decisions

### 1. Proximity decides the root; marker order is not a rule

The nearest ancestor holding any of the language's markers wins. That is the existing behavior and it stays.

An earlier draft of this design specified a second level — declared order breaking a tie when one directory holds several markers — and the spec claimed the winning marker would be "reported as the reason". Both were wrong. Once the directory is fixed, which marker matched changes nothing about the resolved path, so the only way that rule could be observable is through a surface that reports it, and no such surface exists. Specifying it would have meant either an untestable requirement or inventing a status field nobody asked for. The spec now states only what is observable: any declared marker identifies a root on its own, and proximity decides between directories.

`root_markers` stays an ordered array because arrays are ordered, but the order carries no meaning and no test asserts one.

Rejected separately: strength-first search, where a further `pyproject.toml` beats a nearer `setup.py`. A nested package with its own `setup.py` is a real project root; skipping past it to the monorepo root would hand the server the wrong scope, which is the failure the workspace-boundary rule already exists to prevent.

### 2. `build/compile_commands.json` is a second marker, not a second mechanism

Root detection already tests `current.join(marker)`, and `Path::join` accepts a relative path with a separator. So C/C++ declares two markers, `compile_commands.json` then `build/compile_commands.json`, and the existing walk finds either.

Rejected: a per-language root-detection strategy enum. It would introduce a branch in the resolver for a case the resolver already handles, and every later language would have to decide which strategy it is.

This is the one place marker order does have a practical effect — `compile_commands.json` is checked before `build/compile_commands.json` within a directory — but the resolved directory is the same either way, so it is a lookup detail rather than a rule.

### 3. A language may require a marker instead of falling back to the workspace root

The resolver's fallback — no marker anywhere, so treat the session workspace as the root — is right for Rust, TypeScript, Go, and Python: those servers degrade gracefully without a manifest. `clangd` does not. Without a compilation database it assumes default flags, and then answers definitions and diagnostics that are confidently wrong rather than absent, which is worse than an unavailable result.

So `LanguageDefinition` gains `requires_root_marker: bool`, true only for C/C++. When it is set and no marker is found, resolution fails with a new `ProjectRootError` that maps to a new safe reason code rather than to the generic project-root-unavailable one — a user who is told "no compilation database" knows what to do, and a user told "project root unavailable" does not.

This is deliberately not "clangd is unavailable". Discovery still reports the executable as available, because it is; the workspace is what cannot be served. Conflating the two would send a user looking for an installation problem that does not exist.

### 4. Python's fork is preferred over the upstream server

Declared order is `basedpyright-langserver` then `pyright-langserver`. Installing a fork is a deliberate act in a way that installing the upstream one is not, so a host carrying both most likely wants the fork. Discovery reports which candidate it selected, so the choice is visible rather than silent.

This is a decision on record rather than an accident of array order, which is why the spec names it.

### 5. `.h` maps to `c`

`.h` is used by both C and C++ projects and the extension cannot tell them apart. `clangd` infers the real dialect from the compilation database and its own heuristics, so the `languageId` here is a hint rather than a determination; `c` is the more conservative of the two. `.hpp`, `.hh`, and `.hxx` are unambiguous and map to `cpp`.

## Risks / Trade-offs

- **The extension-uniqueness test turns a mapping mistake into a build failure** → that is the intent, but it also means `.h` cannot later be claimed by a second language without a decision. Recorded here so the next person meets the reasoning rather than the assertion.
- **`gopls` and `clangd` index eagerly and are slow to become ready on a large repository** → the existing warming state and the distinction between protocol readiness and background indexing already cover it; no new mechanism, but the server-test fixture projects must stay minimal or the bounded initialize deadline will start failing on slow hosts.
- **Adding three languages triples the number of servers a trusted workspace may start** → the existing per-instance keying, idle timeout, and restart budget are unchanged and already bound this; the new risk is only that a user enabling everything sees more memory use, which is their choice to make and the settings page makes visible.
- **The fixture projects are the part most likely to be wrong on a host that has the server installed** → the isolated server test is the only place they run, and a wrong fixture surfaces as an initialize failure that reads like a broken installation. Each fixture is the smallest thing its server accepts, and the server-test phase result names which phase failed.
- **`requires_root_marker` is a second reason resolution can fail** → every caller of `ProjectRootResolver::resolve` already handles an error result; the new variant must map to its own reason code rather than being folded into an existing one, or the actionable message is lost at the boundary.

## Migration Plan

No database migration. Language configuration rows are created on demand and the storage layer stopped constraining language ids in the previous change, so a new language needs no schema change — which was the point of that change and is the first thing this one confirms.

Rollback is code-only. A user who had enabled Go and then downgrades leaves a `go` row in `lsp_language_configurations`; the previous change already specified that an unregistered row is skipped and preserved, so the downgrade is quiet and re-upgrading restores the setting.
