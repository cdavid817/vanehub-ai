## Why

Once the language registry is data-driven, the three languages whose servers behave like the two already supported — a single executable on `PATH` speaking stdio JSON-RPC — can be added without inventing any new mechanism. Go, Python, and C/C++ are the three most common languages in VaneHub workspaces that currently get no semantic code intelligence at all, so the Agent falls back to `read_file` plus inference for them.

## What Changes

- Register `gopls` for Go: project root detected from `go.mod`, stdio with no extra startup arguments.
- Register `pyright-langserver` and `basedpyright-langserver` for Python as two alternative executables of one language, resolved in preference order, started with `--stdio`.
- Register `clangd` for C/C++.
- Declare each new language's project-root markers, and define the precedence rule when several markers appear in one directory. Python has four candidates (`pyproject.toml`, `setup.py`, `setup.cfg`, `requirements.txt`) with genuinely different strength, unlike the existing TypeScript list where any match is equally valid.
- Treat `compile_commands.json` as C/C++'s root signal rather than a package-manifest marker, because `clangd` without a compilation database produces misleading results. A C/C++ workspace with no compilation database reports an explicit unavailable reason instead of starting a server that will answer badly.
- Add one minimal isolated fixture project per new language so the existing bounded initialize-and-shutdown server test works for them.
- Extend the settings language cards and all five locale bundles to cover the new languages.
- Install nothing. Discovery stays "found on `PATH` or pointed at by an absolute executable override"; managed installation is deliberately deferred to `extract-managed-tool-installation`.

## Capabilities

### New Capabilities

None. The registry from `extend-lsp-language-registry` already carries the shape these languages plug into.

### Modified Capabilities

- `lsp-server-management`: gains the three registered languages with their declared executables and startup arguments, a declared precedence rule for languages whose root markers differ in strength, and a distinct compilation-database root rule for C/C++ including the explicit unavailable outcome when no compilation database exists.

**No `settings-center-ui` delta.** An earlier draft of this proposal claimed one, written before `extend-lsp-language-registry` landed. That change made the settings surface render one card per backend-supplied descriptor, so three new languages need no requirement change there at all — which is precisely what it was for. Inventing a delta to match a stale plan would be recording work that is not happening.

Nor does `lsp-code-intelligence` need one. A C/C++ workspace with no compilation database resolves to an unavailable outcome with a safe reason, which its existing "tool outcomes distinguish degradation from no result" requirement already governs.

## Impact

**Runtimes affected: desktop and Web.** The Web/mock adapter gains the new language ids in its deterministic contract; it still starts no process and reads no files.

Frontend/backend isolation is unchanged, and no new adapter boundary appears. If `extend-lsp-language-registry` did its job, the frontend diff here is locale strings plus whatever the data-driven card renderer needs, not new components per language.

Affected code:

- `src-tauri/src/contexts/code_intelligence/` — three new registry entries, three new server-test fixture projects, root-marker precedence, and the C/C++ compilation-database root rule
- `src/i18n/locales/{en,zh-CN,zh-TW,ja,ko}.json`
- `docs/{user,developer}-guide/{en,zh-CN}/src/lsp-code-intelligence.md` — install instructions and per-language limitations

Dependency: this change requires `extend-lsp-language-registry` to land first. Attempting it against the closed enums would triple the very cost that change exists to remove.

Deliberately out of scope: Java. `jdtls` is not a `PATH` executable speaking stdio and does not fit this change's premise; it is handled by `add-lsp-java-jdtls`.
