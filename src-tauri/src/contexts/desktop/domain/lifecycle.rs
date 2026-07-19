pub(crate) fn should_hide_main_for_tray(tray_available: bool, quitting: bool) -> bool {
    tray_available && !quitting
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_interception_requires_a_working_tray_and_non_quit_state() {
        assert!(should_hide_main_for_tray(true, false));
        assert!(!should_hide_main_for_tray(false, false));
        assert!(!should_hide_main_for_tray(true, true));
    }
}
