use crate::contexts::desktop::application::{DesktopLocalePort, DesktopSettingsApplicationError};
use crate::contexts::desktop::domain::ApplicationLanguage;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub(crate) struct DesktopLocaleBridge {
    target: Arc<RwLock<Option<Arc<dyn DesktopLocalePort>>>>,
}

impl DesktopLocaleBridge {
    pub(crate) fn attach(&self, target: Arc<dyn DesktopLocalePort>) -> Result<(), String> {
        let mut slot = self
            .target
            .write()
            .map_err(|_| "desktop-locale-bridge-poisoned".to_string())?;
        *slot = Some(target);
        Ok(())
    }
}

impl DesktopLocalePort for DesktopLocaleBridge {
    fn apply(&self, language: ApplicationLanguage) -> Result<(), DesktopSettingsApplicationError> {
        let target = self
            .target
            .read()
            .map_err(|_| {
                DesktopSettingsApplicationError::NativeLocale(
                    "desktop-locale-bridge-poisoned".to_string(),
                )
            })?
            .clone();
        match target {
            Some(target) => target.apply(language),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct RecordingLocalePort(Mutex<Vec<ApplicationLanguage>>);

    impl DesktopLocalePort for RecordingLocalePort {
        fn apply(
            &self,
            language: ApplicationLanguage,
        ) -> Result<(), DesktopSettingsApplicationError> {
            self.0.lock().expect("recording locale").push(language);
            Ok(())
        }
    }

    #[test]
    fn forwards_locale_changes_after_attachment() {
        let bridge = DesktopLocaleBridge::default();
        let target = Arc::new(RecordingLocalePort(Mutex::new(Vec::new())));
        bridge.attach(target.clone()).expect("attach");

        bridge
            .apply(ApplicationLanguage::Japanese)
            .expect("apply locale");

        assert_eq!(
            target.0.lock().expect("recording locale").as_slice(),
            &[ApplicationLanguage::Japanese]
        );
    }
}
