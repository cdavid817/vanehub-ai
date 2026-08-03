## Verification Evidence

### Task 10.3 failure-injection matrix

| Boundary | Injected failure | Terminal-state evidence |
|---|---|---|
| Relay artifact creation | Relay root is a file, invocation directory disappears before guard acquisition, exclusive file creation collides, provider configuration exceeds its serialized limit after a server artifact exists | `directory_and_guard_creation_failures_leave_no_owned_artifacts`, `artifact_creation_failure_remains_owned_by_idempotent_cleanup`, `aborted_preparation_drops_every_partially_written_artifact`, and `provider_configuration_failure_cleans_previously_created_server_artifacts` prove no invocation directory or provider file remains. |
| Session startup | Missing stdio executable, cancelled Streamable HTTP before send, legacy SSE redirect/deadline/cancellation during endpoint negotiation | `stdio_session_startup_failure_creates_no_owned_task_or_child`, `cancellation_before_send_opens_no_connection`, and the `legacy_sse_tests` failure cases prove startup fails before an unowned child, connection, or task can remain. |
| Protocol phases | Cancellation during initialize/discovery/invocation, invalid frames, redirect/status failures, disconnect, timeout, oversized JSON/SSE/body, stdio pump failure and open-parent-stdin races | `connection_adapter_tests`, `relay_stdio::tests`, both relay `failure_tests` modules, and the independent fixture contract tests assert bounded typed failures correlated to the originating request. |
| Persistence | Import insert failure, connection-outcome persistence failure, and logging-port persistence failure with injected private database/sink details | `native_import_reports_validation_and_storage_failures_per_entry`, `failed_connection_preserves_the_prior_valid_tool_cache`, and `persistence_and_logging_failures_leave_only_a_safe_operation_diagnostic` prove terminal operations continue with fixed safe text and no raw storage detail. |
| Logging | Normal sink receives a secret-bearing command/error; normal sink is unavailable and emergency fallback is used | `normal_sink_receives_only_safe_command_and_failure_metadata` and `emergency_sink_receives_only_a_fixed_already_safe_classification` assert the only remaining diagnostic artifact is `vanehub.log` and it contains no raw args, env, headers, bodies, schemas, tool data, stderr, or relay configuration. |
| Cancellation | Owned HTTP task is pending, stdio descendant tree is running, HTTP request is in flight, stdio relay parent input remains open | `managed_cancellation_drops_and_joins_the_owned_operation_task` asserts active task count returns to zero; managed-session, connection-adapter, HTTP, and stdio-relay cancellation tests assert terminal resource release before return. |
| Cleanup | Remote success followed by cleanup failure, hanging/failing Streamable HTTP `DELETE`, parent EOF with hanging stdio child, partial provider preparation | Managed-session and connection-adapter cleanup tests convert false success to `cleanup`; relay cleanup tests remain wall-clock bounded; relay guards remove every owned artifact idempotently. |

### No-residual-resource evidence

- Secret-bearing files: `injected_protocol_failure_reaps_descendants_and_leaves_no_raw_secret_artifact` runs the real VaneHub helper with a secret relay configuration, secret stderr, an invalid frame, and a descendant process. It asserts the helper consumes the relay file, scans every remaining invocation artifact for injected secrets, then verifies invocation cleanup.
- Child and descendant processes: the same process-level test and `managed_stdio_cancellation_terminates_the_owned_descendant_tree` verify reported descendant PIDs are no longer alive before the operation is considered terminal.
- Tasks: `managed_cancellation_drops_and_joins_the_owned_operation_task` uses a drop guard and asserts the active count is zero after cancellation returns.
- Raw diagnostics: runtime logging tests enumerate all injected secrets in normal and emergency sinks, while the real-helper failure test also checks protocol stdout, native stderr, filenames, and file contents.
- Web/mock native side effects: `web-mcp-tool-simulation.test.ts` exercises catalog, arguments, and rendered-result limit-plus-one inputs. Each branch returns only a failed tool event with safe code `limit_exceeded`; no approval/completion event or native process/network lifecycle claim is produced.

### Commands recorded for task 10.3

Executed on 2026-08-03 in the dedicated `harden-mcp-runtime-reliability` worktree:

| Command group | Result |
|---|---|
| Private relay filesystem failure tests | 9 passed |
| Provider partial-preparation cleanup test | 1 passed |
| Managed MCP session failure tests | 7 passed |
| Runtime logging failure tests | 3 passed |
| Persistence/logging port failure test | 1 passed |
| Connection adapter lifecycle tests | 10 passed |
| Stdio relay failure/supervisor tests | 5 passed |
| Streamable HTTP relay failure tests | 2 passed |
| Legacy SSE relay failure tests | 2 passed |
| Streamable HTTP session failure tests | 4 passed |
| Legacy SSE session deadline/cancellation tests | 4 passed |
| Real helper secret/descendant failure injection | 1 passed; complete provider matrix 3/3 passed |

### Commands recorded for task 10.4

Executed on 2026-08-03 in the dedicated `harden-mcp-runtime-reliability` worktree:

| Command | Result |
|---|---|
| `npm run lint` | Passed with zero ESLint errors (exit code 0). |
| `npm run test` | 119 test files passed; 453 tests passed (exit code 0). |
| `npm run contracts:check` | 1 contract test file passed; 2 tests passed (exit code 0). |
| `npm run build` | TypeScript compilation, Vite production build, and frontend chunk-budget check passed (exit code 0); 16 lazy chunks verified and the main static closure was 103.9 KiB gzip. Vite emitted informational plugin-timing and large-chunk warnings, but no build or project chunk-budget failure. |

No TypeScript or lint rule was weakened. The final Web/mock parity review added shared MCP catalog/result validation, bounded simulation planning, and direct limit-plus-one coverage for catalog, arguments, and rendered results; the full frontend suite passed after the form-error mapping was exhaustively narrowed for the expanded validation-field union.

### Commands recorded for task 10.5

Executed on 2026-08-03 in the dedicated `harden-mcp-runtime-reliability` worktree:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed from `src-tauri/`, the directory containing the Rust workspace manifest (exit code 0). |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Passed: 1,154 library tests, 11 architecture tests, 3 MCP fixture-contract tests, and 3 relay-provider integration tests; 9 process-fixture entry points remained intentionally ignored (exit code 0). |
| `cargo check --manifest-path src-tauri/Cargo.toml` | Passed with zero warnings (exit code 0). |
| `cargo clippy --manifest-path src-tauri/Cargo.toml` | Passed with zero warnings (exit code 0). |
| `openspec validate harden-mcp-runtime-reliability --strict` | Change is valid (exit code 0). |
| `openspec validate --specs --strict` | 84 main specifications passed; 0 failed (exit code 0). |

The final validation run found and corrected three quality issues before the successful rerun: the HTTP timeout fixture now bounds its own connection accept so an already-expired request cannot leave the test blocked in `join`; the runtime-I/O architecture gate explicitly recognizes the private relay filesystem as a shared platform adapter while the external-process fixture is marked test-only; and seven Rust/Clippy warnings were removed, including boxing the large stdio session resource to reduce the managed-session enum size.
