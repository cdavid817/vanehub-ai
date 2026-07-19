use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DesktopLifecycleApplicationError {
    Runtime(String),
}

impl fmt::Display for DesktopLifecycleApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for DesktopLifecycleApplicationError {}
