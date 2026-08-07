//! Streaming text decoding for byte sources that deliver arbitrary chunk boundaries.

/// Drains every byte of `pending` that forms complete UTF-8, leaving an incomplete
/// trailing sequence behind for the next read to finish.
///
/// PTY and pipe reads land on arbitrary byte boundaries, so a multi-byte sequence
/// (中文 / emoji / TUI box-drawing) can be split across two reads. Decoding each read
/// in isolation would emit U+FFFD in the middle of otherwise valid output.
pub(crate) fn take_decodable_utf8(pending: &mut Vec<u8>) -> String {
    let valid_up_to = match std::str::from_utf8(pending) {
        Ok(_) => pending.len(),
        // `error_len() == None` means the bytes after `valid_up_to` are an incomplete
        // sequence at the end of the buffer — keep them for the next read.
        Err(error) if error.error_len().is_none() => error.valid_up_to(),
        Err(_) => {
            let content = String::from_utf8_lossy(pending).to_string();
            pending.clear();
            return content;
        }
    };
    let tail = pending.split_off(valid_up_to);
    let head = std::mem::replace(pending, tail);
    // `head` is guaranteed valid UTF-8 by the check above; fall back rather than panic.
    String::from_utf8(head).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_multibyte_utf8_is_buffered_until_the_sequence_completes() {
        // "好" is E5 A5 BD; a read that ends after E5 A5 must not emit a replacement char.
        let bytes = "已好".as_bytes().to_vec();
        let split = bytes.len() - 1;
        let mut pending = bytes[..split].to_vec();

        let first = take_decodable_utf8(&mut pending);
        assert_eq!(first, "已");
        assert!(!first.contains('\u{FFFD}'));
        assert!(!pending.is_empty(), "incomplete tail is retained");

        pending.extend_from_slice(&bytes[split..]);
        let second = take_decodable_utf8(&mut pending);
        assert_eq!(second, "好");
        assert!(pending.is_empty());
    }

    #[test]
    fn complete_utf8_is_returned_whole_and_drains_pending() {
        let mut pending = "ready ✅".as_bytes().to_vec();

        assert_eq!(take_decodable_utf8(&mut pending), "ready ✅");
        assert!(pending.is_empty());
    }

    #[test]
    fn invalid_bytes_are_replaced_rather_than_retained_forever() {
        // A genuine decode error (not a truncated tail) must not stall the stream.
        let mut pending = vec![0xE5, 0xA5, 0xBD, 0xFF, b'a'];

        let decoded = take_decodable_utf8(&mut pending);

        assert!(decoded.starts_with("好"));
        assert!(decoded.contains('\u{FFFD}'));
        assert!(pending.is_empty());
    }
}
