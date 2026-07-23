use super::*;
use std::io::{Cursor, Write};

#[test]
fn relay_configuration_round_trip_preserves_literal_json_rpc_arguments() {
    let literal = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"arguments":{"quoted":"a b \\\" c"}}}"#;
    let configuration = RelayConfiguration {
        target: RelayTarget::Stdio {
            command: "node".to_string(),
            args: vec!["fixture.cjs".to_string(), literal.to_string()],
            env: BTreeMap::new(),
        },
        traceparent: "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string(),
        observation: None,
        timeout_ms: 25,
    };
    let encoded = serde_json::to_vec(&configuration).expect("encode");
    let decoded = serde_json::from_slice::<RelayConfiguration>(&encoded).expect("decode");
    let RelayTarget::Stdio { args, .. } = decoded.target else {
        panic!("stdio target expected");
    };
    assert_eq!(args[1], literal);
}

#[test]
fn configuration_never_invents_an_unbounded_timeout() {
    assert_eq!(default_timeout_ms(), DEFAULT_TIMEOUT_MS);
}

#[test]
fn stdio_forwarding_is_byte_transparent_for_json_rpc_and_protocol_errors() {
    let input = br#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"arguments":{"literal":"a b ; & \"quoted\""}}}
not-json-but-still-forwarded
"#;
    let mut output = Vec::new();

    forward_stdio_input(Cursor::new(input), &mut output, None).expect("forward");

    assert_eq!(output, input);
}

#[test]
fn stdio_forwarding_propagates_a_bounded_writer_failure() {
    struct FailingWriter;
    impl Write for FailingWriter {
        fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "cancelled",
            ))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let result = forward_stdio_input(
        Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"ping"}
"#,
        ),
        &mut FailingWriter,
        None,
    );

    assert!(result.is_err());
}
