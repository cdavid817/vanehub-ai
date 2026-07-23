use super::ExecutionDomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ExecutionRunId(String);

impl ExecutionRunId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, ExecutionDomainError> {
        let value = value.into();
        let bytes = value.as_bytes();
        let hyphens = [8, 13, 18, 23];
        let valid = bytes.len() == 36
            && bytes.iter().enumerate().all(|(index, byte)| {
                if hyphens.contains(&index) {
                    *byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }
            });
        if !valid {
            return Err(ExecutionDomainError::InvalidRunId);
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

macro_rules! telemetry_id {
    ($name:ident, $length:expr, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn parse(value: impl Into<String>) -> Result<Self, ExecutionDomainError> {
                let value = value.into();
                if value.len() != $length
                    || value.bytes().any(|byte| !byte.is_ascii_hexdigit())
                    || value.bytes().all(|byte| byte == b'0')
                {
                    return Err(ExecutionDomainError::InvalidTelemetryId {
                        kind: $kind,
                        expected: $length,
                    });
                }
                Ok(Self(value.to_ascii_lowercase()))
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

telemetry_id!(TraceId, 32, "trace id");
telemetry_id!(SpanId, 16, "span id");
