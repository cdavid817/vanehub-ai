## Slow Operation Audit

### Frontend

- CLI management refresh and install already use `OperationTask` via `AgentService`, Tauri/Web adapters, and operation polling in the providers page.
- SDK install/update/rollback/uninstall previously waited for a final `SdkOperationResult`; this change migrates those service methods to return `OperationTask` and lets the SDK page poll status while preserving loaded SDK cards.
- MCP connection testing previously waited for a final `McpTestResult`; this change migrates `testConnection` to return `OperationTask` and lets the MCP page poll status while preserving the server list.
- SDK/MCP refresh buttons already keep existing query data visible during `isFetching`; no blank replacement flow was found there.
- Session creation previously returned a final `Session`; this change migrates it to return `OperationTask` so optional Git worktree creation can run in the background while the dialog shows running state.
- Web SDK and MCP adapters now simulate operation state through the shared Web operation client.
- Web session creation now simulates operation state and stores the created `Session` in the terminal operation result.

### Native

- CLI detection and CLI package operations already return an operation id before detection/npm work completes and run background work through `spawn_blocking`.
- SDK package operations previously created a task but still ran npm synchronously inside the Tauri command; this change moves the SDK operation body into background execution and stores terminal results on the task.
- MCP connection tests previously used async execution but still awaited completion inside the command; this change starts the test in the background and stores terminal results on the task.
- MCP test diagnostics now write operation-associated entries through unified logging.
- Session creation, including optional `git worktree add`, now starts a workspace operation and runs the create flow in background execution; the created `Session` is stored in the terminal task result.
- Cached reads and bounded writes such as listing settings, listing cached MCP servers, updating server configuration, and reading operation status remain direct request/response operations.

### Compatibility Notes

- Existing operation status APIs are reused rather than introducing a second task model.
- SDK, MCP, and session creation service method return types changed from final result to `OperationTask`; callers must observe `operationService` for terminal results.
- Terminal SDK and MCP results are stored in `OperationTask.result` for pages that need result-specific notices.
- Terminal session creation results are stored in `OperationTask.result` as a `Session`.
