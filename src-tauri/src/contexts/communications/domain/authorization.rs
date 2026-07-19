use super::CommunicationsDomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthorizationStatus {
    Waiting,
    Scanned,
    Confirmed,
    Expired,
    Error,
    Cancelled,
}

impl AuthorizationStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Scanned => "scanned",
            Self::Confirmed => "confirmed",
            Self::Expired => "expired",
            Self::Error => "error",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Confirmed | Self::Expired | Self::Error | Self::Cancelled
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AuthorizationObservation {
    Waiting,
    Scanned,
    Confirmed,
    Expired,
    Failed(String),
}

impl AuthorizationObservation {
    fn status(&self) -> AuthorizationStatus {
        match self {
            Self::Waiting => AuthorizationStatus::Waiting,
            Self::Scanned => AuthorizationStatus::Scanned,
            Self::Confirmed => AuthorizationStatus::Confirmed,
            Self::Expired => AuthorizationStatus::Expired,
            Self::Failed(_) => AuthorizationStatus::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthorizationAttempt {
    status: AuthorizationStatus,
    expires_at_millis: i64,
    safe_error_code: Option<String>,
}

impl AuthorizationAttempt {
    pub(crate) fn begin(
        started_at_millis: i64,
        expires_at_millis: i64,
    ) -> Result<Self, CommunicationsDomainError> {
        if expires_at_millis <= started_at_millis {
            return Err(CommunicationsDomainError::InvalidAuthorizationDeadline);
        }
        Ok(Self {
            status: AuthorizationStatus::Waiting,
            expires_at_millis,
            safe_error_code: None,
        })
    }

    pub(crate) fn current_status(&self) -> AuthorizationStatus {
        self.status
    }

    pub(crate) fn safe_error_code(&self) -> Option<&str> {
        self.safe_error_code.as_deref()
    }

    pub(crate) fn expire_if_due(&mut self, now_millis: i64) -> bool {
        if !self.status.is_terminal() && now_millis >= self.expires_at_millis {
            self.status = AuthorizationStatus::Expired;
            self.safe_error_code = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn observe(
        &mut self,
        now_millis: i64,
        observation: AuthorizationObservation,
    ) -> Result<AuthorizationStatus, CommunicationsDomainError> {
        if self.expire_if_due(now_millis) {
            return Ok(self.status);
        }
        if self.status.is_terminal() {
            return Err(CommunicationsDomainError::InvalidAuthorizationTransition {
                from: self.status.as_str(),
                to: observation.status().as_str(),
            });
        }
        match observation {
            AuthorizationObservation::Waiting if self.status == AuthorizationStatus::Scanned => {}
            AuthorizationObservation::Waiting => self.status = AuthorizationStatus::Waiting,
            AuthorizationObservation::Scanned => self.status = AuthorizationStatus::Scanned,
            AuthorizationObservation::Confirmed => self.status = AuthorizationStatus::Confirmed,
            AuthorizationObservation::Expired => self.status = AuthorizationStatus::Expired,
            AuthorizationObservation::Failed(code) => {
                if code.trim().is_empty() {
                    return Err(CommunicationsDomainError::AuthorizationErrorCodeRequired);
                }
                self.safe_error_code = Some(code);
                self.status = AuthorizationStatus::Error;
            }
        }
        Ok(self.status)
    }

    pub(crate) fn cancel(&mut self) -> Result<(), CommunicationsDomainError> {
        if self.status.is_terminal() {
            return Err(CommunicationsDomainError::InvalidAuthorizationTransition {
                from: self.status.as_str(),
                to: AuthorizationStatus::Cancelled.as_str(),
            });
        }
        self.status = AuthorizationStatus::Cancelled;
        self.safe_error_code = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_progress_is_monotonic_and_terminal_once() {
        let mut attempt = AuthorizationAttempt::begin(100, 200).expect("attempt");
        assert_eq!(attempt.current_status(), AuthorizationStatus::Waiting);
        assert_eq!(
            attempt
                .observe(120, AuthorizationObservation::Scanned)
                .expect("scanned"),
            AuthorizationStatus::Scanned
        );
        assert_eq!(
            attempt
                .observe(130, AuthorizationObservation::Waiting)
                .expect("stale waiting"),
            AuthorizationStatus::Scanned
        );
        attempt
            .observe(140, AuthorizationObservation::Confirmed)
            .expect("confirmed");
        assert!(attempt
            .observe(150, AuthorizationObservation::Waiting)
            .is_err());
    }

    #[test]
    fn deadline_and_failures_produce_explicit_terminal_states() {
        assert_eq!(
            AuthorizationAttempt::begin(100, 100),
            Err(CommunicationsDomainError::InvalidAuthorizationDeadline)
        );

        let mut expired = AuthorizationAttempt::begin(100, 200).expect("attempt");
        assert!(expired.expire_if_due(200));
        assert_eq!(expired.current_status(), AuthorizationStatus::Expired);

        let mut failed = AuthorizationAttempt::begin(100, 200).expect("attempt");
        failed
            .observe(
                150,
                AuthorizationObservation::Failed("wechat-api-invalid".to_string()),
            )
            .expect("failure");
        assert_eq!(failed.current_status(), AuthorizationStatus::Error);
        assert_eq!(failed.safe_error_code(), Some("wechat-api-invalid"));
    }

    #[test]
    fn cancellation_is_explicit_and_cannot_replace_a_terminal_result() {
        let mut attempt = AuthorizationAttempt::begin(100, 200).expect("attempt");
        attempt.cancel().expect("cancel");
        assert_eq!(attempt.current_status(), AuthorizationStatus::Cancelled);
        assert!(attempt.cancel().is_err());
    }
}
