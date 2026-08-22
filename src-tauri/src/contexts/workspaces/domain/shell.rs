#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalDimensions {
    rows: u16,
    cols: u16,
}

impl TerminalDimensions {
    pub(crate) fn bounded(rows: u16, cols: u16) -> Self {
        Self {
            rows: rows.clamp(1, 500),
            cols: cols.clamp(1, 500),
        }
    }

    pub(crate) fn rows(self) -> u16 {
        self.rows
    }

    pub(crate) fn cols(self) -> u16 {
        self.cols
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellHost {
    Windows,
    Unix,
}

/// What a Session Shell runtime can actually do. The capabilities travel with the variant so a
/// caller cannot ask a simulated shell to resize a PTY it does not have, and a remote shell
/// carries the witnesses that identify which connection it belongs to — the previous
/// `&'static str` capability could name `remote` without being able to say remote *what*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShellRuntimeDescriptor {
    Native,
    Remote {
        connection_id: String,
        profile_revision: i64,
        supports_reconnect: bool,
    },
    Simulated,
    Unavailable {
        reason_code: &'static str,
        remediation: Option<String>,
    },
}

impl ShellRuntimeDescriptor {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Remote { .. } => "remote",
            Self::Simulated => "simulated",
            Self::Unavailable { .. } => "unavailable",
        }
    }

    /// Only a runtime with a real terminal can honour a resize request.
    pub(crate) fn supports_resize(&self) -> bool {
        matches!(self, Self::Native | Self::Remote { .. })
    }

    /// Retained output is a registry concern, so every runtime that exists at all can replay it.
    pub(crate) fn supports_replay(&self) -> bool {
        !matches!(self, Self::Unavailable { .. })
    }

    pub(crate) fn supports_reconnect(&self) -> bool {
        match self {
            Self::Remote {
                supports_reconnect, ..
            } => *supports_reconnect,
            _ => false,
        }
    }
}

pub(crate) fn reset_directory_command(root: &str, host: ShellHost) -> String {
    match host {
        ShellHost::Windows => format!("cd /d \"{root}\"\r\n"),
        ShellHost::Unix => {
            let escaped = root.replace('\'', "'\"'\"'");
            format!("cd '{escaped}'\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_dimensions_keep_the_existing_safety_bounds() {
        assert_eq!(TerminalDimensions::bounded(0, 0).rows(), 1);
        assert_eq!(TerminalDimensions::bounded(800, 900).cols(), 500);
        assert_eq!(TerminalDimensions::bounded(24, 80).cols(), 80);
    }

    #[test]
    fn reset_directory_commands_preserve_platform_escaping() {
        assert_eq!(
            reset_directory_command("C:\\folder with spaces", ShellHost::Windows),
            "cd /d \"C:\\folder with spaces\"\r\n"
        );
        assert_eq!(
            reset_directory_command("/work/it's here", ShellHost::Unix),
            "cd '/work/it'\"'\"'s here'\n"
        );
    }
}
