use super::relay_jsonrpc::{
    read_bounded_frame, JsonRpcCorrelation, JsonRpcFrame, PendingRequest, RelayDirection,
};
use super::relay_observer::{RelayObserver, RelayRequest};
use crate::contexts::tooling::mcp::application::McpLimits;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

pub(super) type StdioCorrelation = Arc<Mutex<JsonRpcCorrelation<Option<RelayRequest>>>>;

#[derive(Clone, Default)]
pub(super) struct PumpStop {
    stopped: Arc<AtomicBool>,
}

impl PumpStop {
    pub(super) fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
    }

    #[cfg(test)]
    pub(super) fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }
}

pub(super) struct ClosableWriter<W> {
    inner: Arc<Mutex<Option<W>>>,
}

impl<W> ClosableWriter<W> {
    pub(super) fn new(writer: W) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Some(writer))),
        }
    }

    pub(super) fn close(&self) {
        if let Ok(mut writer) = self.inner.lock() {
            writer.take();
        }
    }
}

impl<W> Clone for ClosableWriter<W> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<W: Write> Write for ClosableWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.inner
            .lock()
            .map_err(|_| io::Error::other("relay writer state is unavailable"))?
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "relay writer is closed"))?
            .write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner
            .lock()
            .map_err(|_| io::Error::other("relay writer state is unavailable"))?
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "relay writer is closed"))?
            .flush()
    }
}

pub(super) enum PumpEvent {
    Ended(RelayDirection, Result<(), String>),
}

pub(super) fn spawn_pump<R, W>(
    source: R,
    mut target: W,
    direction: RelayDirection,
    observer: Option<RelayObserver>,
    correlation: StdioCorrelation,
    events: mpsc::Sender<PumpEvent>,
) -> JoinHandle<()>
where
    R: BufRead + Send + 'static,
    W: Write + Send + 'static,
{
    thread::spawn(move || {
        let result = forward_stdio_frames(
            source,
            &mut target,
            direction,
            observer.as_ref(),
            &correlation,
        );
        let _ = events.send(PumpEvent::Ended(direction, result));
    })
}

pub(super) fn join_if_finished(handle: JoinHandle<()>) {
    if handle.is_finished() {
        let _ = handle.join();
    }
}

pub(super) fn forward_stdio_frames(
    mut source: impl BufRead,
    target: &mut impl Write,
    direction: RelayDirection,
    observer: Option<&RelayObserver>,
    correlation: &StdioCorrelation,
) -> Result<(), String> {
    let mut frame_bytes = Vec::new();
    loop {
        let count = read_bounded_frame(
            &mut source,
            &mut frame_bytes,
            McpLimits::DEFAULT.protocol_message_bytes,
        )
        .map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        let frame = super::relay_jsonrpc::parse_json_rpc_frame(&frame_bytes)?;
        if let JsonRpcFrame::Request { id, method } = &frame {
            let token = observer.and_then(|observer| observer.start_request("stdio", Some(method)));
            let inserted = correlation
                .lock()
                .map_err(|_| "relay correlation state is unavailable".to_string())?
                .insert_request(direction, id.clone(), token);
            if let Err(pending) = inserted {
                finish_pending(
                    observer,
                    pending,
                    false,
                    Some("mcp_stdio_duplicate_request_id"),
                );
                return Err("relay received a duplicate in-flight JSON-RPC id".to_string());
            }
        }
        let result = target
            .write_all(&frame_bytes)
            .and_then(|()| target.flush())
            .map_err(|error| error.to_string());
        if result.is_err() {
            if let JsonRpcFrame::Request { id, .. } = &frame {
                let pending = correlation
                    .lock()
                    .map_err(|_| "relay correlation state is unavailable".to_string())?
                    .abort_request(direction, id);
                if let Some(pending) = pending {
                    finish_pending(observer, pending, false, Some("mcp_stdio_forward_failed"));
                }
            }
            result?;
        }
        if let JsonRpcFrame::Response { id, success } = &frame {
            let pending = correlation
                .lock()
                .map_err(|_| "relay correlation state is unavailable".to_string())?
                .complete_response(direction, id);
            if let Some(pending) = pending {
                finish_pending(
                    observer,
                    pending,
                    *success,
                    (!success).then_some("mcp_stdio_json_rpc_error"),
                );
            }
        }
    }
    Ok(())
}

pub(super) fn finish_pending(
    observer: Option<&RelayObserver>,
    pending: PendingRequest<Option<RelayRequest>>,
    success: bool,
    error_classification: Option<&str>,
) {
    if let (Some(observer), Some(request)) = (observer, pending.token) {
        observer.finish_request(request, success, error_classification);
    }
}
