use super::models::ConnectorKind;
use crate::AppError;
use std::collections::HashMap;
use std::sync::Mutex;
use zeroize::Zeroizing;

const SERVICE_NAME: &str = "io.vanehub.ai.im";

pub trait CredentialStore: Send + Sync {
    fn set(&self, account: &str, secret: &str) -> Result<(), AppError>;
    fn get(&self, account: &str) -> Result<Option<Zeroizing<String>>, AppError>;
    fn delete(&self, account: &str) -> Result<(), AppError>;
}

pub fn credential_account(kind: ConnectorKind, profile: &str) -> String {
    let normalized = profile
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("{}/{normalized}", kind.as_str())
}

pub fn get_connector_credential(
    store: &dyn CredentialStore,
    kind: ConnectorKind,
    profile: &str,
) -> Result<Option<Zeroizing<String>>, AppError> {
    let account = credential_account(kind, profile);
    if let Some(secret) = store.get(&account)? {
        return Ok(Some(secret));
    }
    if kind != ConnectorKind::WeChat {
        return Ok(None);
    }

    let legacy = account.replacen("weixin/", "wechat/", 1);
    let Some(secret) = store.get(&legacy)? else {
        return Ok(None);
    };
    store.set(&account, secret.as_str())?;
    store.delete(&legacy)?;
    Ok(Some(secret))
}

pub fn delete_connector_credential(
    store: &dyn CredentialStore,
    kind: ConnectorKind,
    profile: &str,
) -> Result<(), AppError> {
    let account = credential_account(kind, profile);
    store.delete(&account)?;
    if kind == ConnectorKind::WeChat {
        store.delete(&account.replacen("weixin/", "wechat/", 1))?;
    }
    Ok(())
}

pub struct OsCredentialStore;

impl CredentialStore for OsCredentialStore {
    fn set(&self, account: &str, secret: &str) -> Result<(), AppError> {
        keyring::Entry::new(SERVICE_NAME, account)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|error| AppError::Storage(format!("credential store write failed: {error}")))
    }

    fn get(&self, account: &str) -> Result<Option<Zeroizing<String>>, AppError> {
        let entry = keyring::Entry::new(SERVICE_NAME, account)
            .map_err(|error| AppError::Storage(format!("credential store open failed: {error}")))?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(AppError::Storage(format!(
                "credential store read failed: {error}"
            ))),
        }
    }

    fn delete(&self, account: &str) -> Result<(), AppError> {
        let entry = keyring::Entry::new(SERVICE_NAME, account)
            .map_err(|error| AppError::Storage(format!("credential store open failed: {error}")))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(AppError::Storage(format!(
                "credential store delete failed: {error}"
            ))),
        }
    }
}

#[derive(Default)]
pub struct MemoryCredentialStore {
    values: Mutex<HashMap<String, String>>,
}

impl CredentialStore for MemoryCredentialStore {
    fn set(&self, account: &str, secret: &str) -> Result<(), AppError> {
        self.values
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?
            .insert(account.to_string(), secret.to_string());
        Ok(())
    }

    fn get(&self, account: &str) -> Result<Option<Zeroizing<String>>, AppError> {
        Ok(self
            .values
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?
            .get(account)
            .cloned()
            .map(Zeroizing::new))
    }

    fn delete(&self, account: &str) -> Result<(), AppError> {
        self.values
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?
            .remove(account);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_round_trip_and_stable_account() {
        let store = MemoryCredentialStore::default();
        let account = credential_account(ConnectorKind::WeCom, "Default Bot");
        assert_eq!(account, "wecom/default-bot");
        store.set(&account, "secret-value").unwrap();
        let secret = store.get(&account).unwrap().unwrap();
        assert_eq!(secret.as_str(), "secret-value");
        store.delete(&account).unwrap();
        assert!(store.get(&account).unwrap().is_none());
    }

    #[test]
    fn migrates_legacy_wechat_account_to_weixin() {
        let store = MemoryCredentialStore::default();
        store.set("wechat/default", "secret-value").unwrap();
        let secret = get_connector_credential(&store, ConnectorKind::WeChat, "default")
            .unwrap()
            .unwrap();
        assert_eq!(secret.as_str(), "secret-value");
        assert!(store.get("wechat/default").unwrap().is_none());
        assert!(store.get("weixin/default").unwrap().is_some());
    }
}
