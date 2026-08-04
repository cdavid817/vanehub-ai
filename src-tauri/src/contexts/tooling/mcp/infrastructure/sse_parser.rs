use crate::contexts::tooling::mcp::application::McpRuntimeError;
use crate::contexts::tooling::mcp::domain::McpFailureCode;

const MAXIMUM_FIELD_OVERHEAD: usize = 8 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct SseEvent {
    pub(super) event_type: Option<String>,
    pub(super) data: Vec<u8>,
}

pub(super) struct BoundedSseParser {
    maximum: usize,
    line: Vec<u8>,
    event_type: Option<String>,
    data: Vec<u8>,
}

impl BoundedSseParser {
    pub(super) fn new(maximum: usize) -> Self {
        Self {
            maximum,
            line: Vec::new(),
            event_type: None,
            data: Vec::new(),
        }
    }

    pub(super) fn feed(&mut self, bytes: &[u8]) -> Result<Vec<SseEvent>, McpRuntimeError> {
        let mut events = Vec::new();
        for byte in bytes {
            if *byte == b'\n' {
                let mut line = std::mem::take(&mut self.line);
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
                if let Some(event) = self.process_line(&line)? {
                    events.push(event);
                }
            } else {
                self.line.push(*byte);
                if self.line.len() > self.maximum.saturating_add(MAXIMUM_FIELD_OVERHEAD)
                    || self.pending_data_length_exceeds_limit()
                {
                    return Err(McpRuntimeError::new(McpFailureCode::LimitExceeded));
                }
            }
        }
        Ok(events)
    }

    fn pending_data_length_exceeds_limit(&self) -> bool {
        let Some(value) = self.line.strip_prefix(b"data:") else {
            return false;
        };
        let value = value.strip_prefix(b" ").unwrap_or(value);
        let separator = usize::from(!self.data.is_empty());
        self.data
            .len()
            .saturating_add(separator)
            .saturating_add(value.len())
            > self.maximum
    }

    fn process_line(&mut self, line: &[u8]) -> Result<Option<SseEvent>, McpRuntimeError> {
        if line.is_empty() {
            if self.event_type.is_none() && self.data.is_empty() {
                return Ok(None);
            }
            return Ok(Some(SseEvent {
                event_type: self.event_type.take(),
                data: std::mem::take(&mut self.data),
            }));
        }
        if line.starts_with(b":") {
            return Ok(None);
        }
        let (field, value) = if let Some(separator) = line.iter().position(|byte| *byte == b':') {
            let value = &line[separator + 1..];
            (
                &line[..separator],
                value.strip_prefix(b" ").unwrap_or(value),
            )
        } else {
            (line, &[] as &[u8])
        };
        match field {
            b"event" => {
                self.event_type = Some(
                    std::str::from_utf8(value)
                        .map_err(|_| McpRuntimeError::new(McpFailureCode::Protocol))?
                        .to_string(),
                );
            }
            b"data" => {
                let separator = usize::from(!self.data.is_empty());
                let new_length = self
                    .data
                    .len()
                    .saturating_add(separator)
                    .saturating_add(value.len());
                if new_length > self.maximum {
                    return Err(McpRuntimeError::new(McpFailureCode::LimitExceeded));
                }
                if separator == 1 {
                    self.data.push(b'\n');
                }
                self.data.extend_from_slice(value);
            }
            _ => {}
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_handles_incremental_crlf_endpoint_and_message_events() {
        let mut parser = BoundedSseParser::new(128);
        assert!(parser.feed(b"event: end").expect("partial").is_empty());
        let events = parser
            .feed(b"point\r\ndata: /messages?id=1\r\n\r\ndata: {\"ok\":true}\n\n")
            .expect("events");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type.as_deref(), Some("endpoint"));
        assert_eq!(events[0].data, b"/messages?id=1");
        assert_eq!(events[1].data, b"{\"ok\":true}");
    }

    #[test]
    fn parser_accepts_exact_data_limit_and_rejects_limit_plus_one() {
        let mut exact = BoundedSseParser::new(8);
        assert_eq!(
            exact.feed(b"data: 12345678\n\n").expect("exact")[0].data,
            b"12345678"
        );

        let mut oversized = BoundedSseParser::new(8);
        let error = oversized
            .feed(b"data: 123456789\n\n")
            .expect_err("limit plus one");
        assert_eq!(error.code(), McpFailureCode::LimitExceeded);
    }

    #[test]
    fn parser_bounds_unterminated_lines_before_an_event_is_allocated() {
        let mut parser = BoundedSseParser::new(8);
        let line = vec![b'x'; 8 + MAXIMUM_FIELD_OVERHEAD + 1];
        let error = parser.feed(&line).expect_err("unterminated line limit");
        assert_eq!(error.code(), McpFailureCode::LimitExceeded);
    }
}
