//! Adapts `desktop::api::DesktopSettingsApi` (a published cross-context facade) to
//! `DefaultTemplatePort`, `permissions`' own dependency-inversion boundary onto the global
//! default-template setting (design.md D2, `add-permissions-settings-ui`).

use crate::contexts::desktop::api::DesktopSettingsApi;
use crate::contexts::permissions::application::DefaultTemplatePort;
use crate::contexts::permissions::domain::PolicyTemplateName;

#[derive(Clone)]
pub(crate) struct DesktopDefaultTemplateAdapter {
    desktop_settings: DesktopSettingsApi,
}

impl DesktopDefaultTemplateAdapter {
    pub(crate) fn new(desktop_settings: DesktopSettingsApi) -> Self {
        Self { desktop_settings }
    }
}

impl DefaultTemplatePort for DesktopDefaultTemplateAdapter {
    fn default_template(&self) -> PolicyTemplateName {
        let stored = self.desktop_settings.get_settings().ok();
        resolve_default_template(
            stored
                .as_ref()
                .map(|view| view.settings.default_policy_template()),
        )
    }
}

/// Any read failure, an absent setting, or an unparseable stored value all resolve to `Standard`
/// here — the one place that fallback decision is made, so callers never need their own notion
/// of "the default is unavailable." A pure function so this resolution logic is testable without
/// constructing a full `DesktopSettingsApi`.
fn resolve_default_template(stored: Option<&str>) -> PolicyTemplateName {
    stored
        .and_then(PolicyTemplateName::from_str)
        .unwrap_or(PolicyTemplateName::Standard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_setting_falls_back_to_standard() {
        assert_eq!(resolve_default_template(None), PolicyTemplateName::Standard);
    }

    #[test]
    fn valid_stored_value_is_used() {
        assert_eq!(
            resolve_default_template(Some("trusted")),
            PolicyTemplateName::Trusted
        );
    }

    #[test]
    fn unparseable_stored_value_falls_back_to_standard() {
        assert_eq!(
            resolve_default_template(Some("not-a-real-template")),
            PolicyTemplateName::Standard
        );
    }
}
