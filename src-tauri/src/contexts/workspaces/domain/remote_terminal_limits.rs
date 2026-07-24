pub(crate) const REMOTE_TERMINAL_POOL_CAPACITY: usize = 8;
pub(crate) const REMOTE_TERMINAL_IDLE_TIMEOUT_SECONDS: u64 = 5 * 60;
pub(crate) const REMOTE_TERMINAL_DRAIN_TIMEOUT_SECONDS: u64 = 30;
pub(crate) const REMOTE_TERMINAL_CONNECT_TIMEOUT_SECONDS: u64 = 15;
pub(crate) const REMOTE_TERMINAL_KEEPALIVE_SECONDS: u64 = 30;

pub(crate) const TERMINAL_CAPTURE_QUEUE_CHUNKS: usize = 256;
pub(crate) const TERMINAL_CAPTURE_CHUNK_BYTES: usize = 32 * 1024;
pub(crate) const TERMINAL_CAPTURE_BATCH_CHUNKS: usize = 32;
pub(crate) const TERMINAL_CAPTURE_RETENTION_DAYS: i64 = 30;
pub(crate) const TERMINAL_CAPTURE_CAPACITY_BYTES: i64 = 512 * 1024 * 1024;
pub(crate) const REMOTE_TERMINAL_TRANSCRIPT_BYTES: usize = 1024 * 1024;

pub(crate) const TERMINAL_SEARCH_DEFAULT_PAGE_SIZE: usize = 50;
pub(crate) const TERMINAL_SEARCH_MAX_PAGE_SIZE: usize = 100;
pub(crate) const TERMINAL_SEARCH_MAX_QUERY_BYTES: usize = 512;
pub(crate) const TERMINAL_SEARCH_MAX_CURSOR_BYTES: usize = 512;

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn connection_limits_are_bounded_and_allow_graceful_draining() {
        assert!((1..=32).contains(&REMOTE_TERMINAL_POOL_CAPACITY));
        assert!(REMOTE_TERMINAL_CONNECT_TIMEOUT_SECONDS > 0);
        assert!(REMOTE_TERMINAL_DRAIN_TIMEOUT_SECONDS < REMOTE_TERMINAL_IDLE_TIMEOUT_SECONDS);
        assert!(REMOTE_TERMINAL_KEEPALIVE_SECONDS < REMOTE_TERMINAL_IDLE_TIMEOUT_SECONDS);
    }

    #[test]
    fn capture_limits_bound_memory_and_storage_work() {
        assert!(TERMINAL_CAPTURE_BATCH_CHUNKS > 0);
        assert!(TERMINAL_CAPTURE_BATCH_CHUNKS <= TERMINAL_CAPTURE_QUEUE_CHUNKS);
        assert!((4 * 1024..=64 * 1024).contains(&TERMINAL_CAPTURE_CHUNK_BYTES));
        assert!(TERMINAL_CAPTURE_RETENTION_DAYS > 0);
        assert!(TERMINAL_CAPTURE_CAPACITY_BYTES >= 64 * 1024 * 1024);
        assert!(REMOTE_TERMINAL_TRANSCRIPT_BYTES <= 2 * 1024 * 1024);
    }

    #[test]
    fn search_limits_reject_unbounded_requests() {
        assert!(TERMINAL_SEARCH_DEFAULT_PAGE_SIZE > 0);
        assert!(TERMINAL_SEARCH_DEFAULT_PAGE_SIZE <= TERMINAL_SEARCH_MAX_PAGE_SIZE);
        assert!(TERMINAL_SEARCH_MAX_PAGE_SIZE <= 200);
        assert!(TERMINAL_SEARCH_MAX_QUERY_BYTES <= 1024);
        assert!(TERMINAL_SEARCH_MAX_CURSOR_BYTES <= 1024);
    }
}
