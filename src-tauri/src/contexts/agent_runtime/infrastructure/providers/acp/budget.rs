//! Resource budgets for one ACP connection.
//!
//! These are engineering initial values chosen for this host, not protocol constants and not
//! measured performance targets. Every one is enforced by a bound that fails closed with a typed
//! reason; none is a wall-clock assertion a test could flake on.

use std::time::Duration;

/// Largest single newline-delimited frame, UTF-8 encoded. Matches the headless parser's domain
/// maximum so a tool result that fits one transport fits the other.
pub(crate) const MAX_FRAME_BYTES: usize = 1_048_576;
/// Deepest JSON nesting accepted on the wire. `serde_json` already caps recursion at 128; this
/// is the tighter product bound applied before a frame is dispatched.
pub(crate) const MAX_JSON_DEPTH: usize = 64;
/// Outbound requests awaiting a response. Beyond this the connection refuses to send.
pub(crate) const MAX_PENDING_OUTBOUND: usize = 128;
/// Inbound requests and notifications queued for the turn driver. The reader blocks (backpressure)
/// rather than dropping when this fills, so an approval can never be lost to a full queue.
pub(crate) const INBOUND_QUEUE_CAPACITY: usize = 1_024;
/// Tail of the agent's stderr kept for diagnostics, after redaction.
pub(crate) const STDERR_TAIL_BYTES: usize = 64 * 1024;
/// Bound on the bytes read from a proxied terminal, per terminal, unless the agent asks for less.
pub(crate) const TERMINAL_OUTPUT_LIMIT_BYTES: usize = 256 * 1024;
/// Largest file the fs proxy will read or write for the agent.
pub(crate) const FS_PROXY_MAX_BYTES: u64 = 8 * 1024 * 1024;
/// Proxied terminals one connection may hold open at once.
pub(crate) const MAX_PROXY_TERMINALS: usize = 8;

/// `initialize`, `session/new`, and `session/load` each get this long.
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
/// After `session/cancel`, how long the agent gets to answer the original prompt with
/// `cancelled` before the host escalates to owned process-tree termination.
pub(crate) const CANCEL_GRACE: Duration = Duration::from_secs(2);
/// How long a human decision may stay open before the host answers `cancelled` for it. A long
/// wait is not a protocol fault; it is a person thinking.
pub(crate) const INTERACTION_DEADLINE: Duration = Duration::from_secs(30 * 60);
/// Polling cadence of the turn driver when nothing is arriving.
pub(crate) const DRIVER_TICK: Duration = Duration::from_millis(25);
/// How long the process-tree shutdown waits before reporting a leaked child.
pub(crate) const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(5);
