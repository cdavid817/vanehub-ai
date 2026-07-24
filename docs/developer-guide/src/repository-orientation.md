# Repository orientation

The same React application runs behind two runtime adapters:

```text
React components
      |
frontend service interfaces
      |
      +-- Web/mock adapters
      |
      +-- Tauri adapters --> Rust commands --> bounded contexts --> SQLite / CLI / OS
```

Important roots:

| Path | Responsibility |
| --- | --- |
| `src/components`, `src/main-layout`, `src/settings` | React presentation and interaction |
| `src/services` | Frontend runtime-independent contracts and adapters |
| `src/types`, `src/contracts` | Transport-independent TypeScript contracts |
| `src-tauri/src/commands` | Thin Tauri command and DTO mapping boundary |
| `src-tauri/src/contexts` | Native domain, application, and infrastructure ownership |
| `src-tauri/src/platform` | Shared platform adapters such as database, process, and logging |
| `openspec/specs` | Confirmed behavior requirements |
| `openspec/changes` | Active and archived change evidence |
| `tests/e2e` | Playwright user-visible regression paths |

Start with `AGENTS.md` and `openspec/project.md`. They are normative contributor rules and take precedence over explanatory examples in this guide.

The detailed native module inventory is maintained in [`src-tauri/ARCHITECTURE.md`](../reference/native-architecture.md) and the repository source. The assembled guide copies that checked-in Markdown as a reference so it cannot diverge from the repository file.
