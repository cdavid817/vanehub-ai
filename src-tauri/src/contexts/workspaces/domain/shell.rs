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
