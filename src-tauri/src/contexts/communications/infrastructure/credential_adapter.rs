use crate::contexts::communications::application::{
    CommunicationsApplicationError, CommunicationsCredentialPort, ConnectorCredential,
};
use crate::contexts::communications::domain::ConnectorKind;
use crate::platform::credentials::OsCredentialStore;
use crate::platform::error::InfrastructureError;
use std::fmt;
use std::sync::Arc;
use zeroize::Zeroizing;

const SERVICE_NAME: &str = "io.vanehub.ai.im";
const DEFAULT_PROFILE: &str = "default";

pub(crate) trait SecureCredentialStore: Send + Sync {
    fn set(&self, account: &str, secret: &str) -> Result<(), InfrastructureError>;
    fn get(&self, account: &str) -> Result<Option<Zeroizing<String>>, InfrastructureError>;
    fn delete(&self, account: &str) -> Result<(), InfrastructureError>;
}

impl SecureCredentialStore for OsCredentialStore {
    fn set(&self, account: &str, secret: &str) -> Result<(), InfrastructureError> {
        OsCredentialStore::set(self, account, secret)
    }

    fn get(&self, account: &str) -> Result<Option<Zeroizing<String>>, InfrastructureError> {
        OsCredentialStore::get(self, account)
    }

    fn delete(&self, account: &str) -> Result<(), InfrastructureError> {
        OsCredentialStore::delete(self, account)
    }
}

#[derive(Clone)]
pub(crate) struct CommunicationsCredentialAdapter {
    store: Arc<dyn SecureCredentialStore>,
}

impl CommunicationsCredentialAdapter {
    pub(crate) fn new() -> Self {
        Self::with_store(Arc::new(OsCredentialStore::new(SERVICE_NAME)))
    }

    fn with_store(store: Arc<dyn SecureCredentialStore>) -> Self {
        Self { store }
    }
}

impl fmt::Debug for CommunicationsCredentialAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommunicationsCredentialAdapter")
            .finish_non_exhaustive()
    }
}

impl CommunicationsCredentialPort for CommunicationsCredentialAdapter {
    fn load(
        &self,
        kind: ConnectorKind,
    ) -> Result<Option<ConnectorCredential>, CommunicationsApplicationError> {
        let account = credential_account(kind, DEFAULT_PROFILE);
        if let Some(secret) = self.store.get(&account).map_err(|_| read_error())? {
            return Ok(Some(ConnectorCredential {
                reference: account,
                secret,
            }));
        }
        if kind != ConnectorKind::WeChat {
            return Ok(None);
        }

        let legacy_account = legacy_wechat_account(&account);
        let Some(secret) = self.store.get(&legacy_account).map_err(|_| read_error())? else {
            return Ok(None);
        };
        self.store
            .set(&account, secret.as_str())
            .map_err(|_| write_error())?;
        self.store
            .delete(&legacy_account)
            .map_err(|_| delete_error())?;
        Ok(Some(ConnectorCredential {
            reference: account,
            secret,
        }))
    }

    fn store(
        &self,
        kind: ConnectorKind,
        secret: &str,
    ) -> Result<ConnectorCredential, CommunicationsApplicationError> {
        let account = credential_account(kind, DEFAULT_PROFILE);
        self.store
            .set(&account, secret)
            .map_err(|_| write_error())?;
        Ok(ConnectorCredential {
            reference: account,
            secret: Zeroizing::new(secret.to_string()),
        })
    }

    fn delete(&self, kind: ConnectorKind) -> Result<(), CommunicationsApplicationError> {
        let account = credential_account(kind, DEFAULT_PROFILE);
        self.store.delete(&account).map_err(|_| delete_error())?;
        if kind == ConnectorKind::WeChat {
            self.store
                .delete(&legacy_wechat_account(&account))
                .map_err(|_| delete_error())?;
            let session_account = credential_account(kind, "session-contexts");
            self.store
                .delete(&session_account)
                .map_err(|_| delete_error())?;
            self.store
                .delete(&legacy_wechat_account(&session_account))
                .map_err(|_| delete_error())?;
        }
        Ok(())
    }
}

pub(crate) fn credential_account(kind: ConnectorKind, profile: &str) -> String {
    let profile = profile.trim();
    let profile = if profile.is_empty() {
        DEFAULT_PROFILE
    } else {
        profile
    };
    let normalized = profile
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("{}/{normalized}", kind.as_str())
}

fn legacy_wechat_account(account: &str) -> String {
    account.replacen("weixin/", "wechat/", 1)
}

fn read_error() -> CommunicationsApplicationError {
    CommunicationsApplicationError::failure("communications-credential-read-failed")
}

fn write_error() -> CommunicationsApplicationError {
    CommunicationsApplicationError::failure("communications-credential-write-failed")
}

fn delete_error() -> CommunicationsApplicationError {
    CommunicationsApplicationError::failure("communications-credential-delete-failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemorySecureCredentialStore {
        values: Mutex<HashMap<String, String>>,
    }

    impl SecureCredentialStore for MemorySecureCredentialStore {
        fn set(&self, account: &str, secret: &str) -> Result<(), InfrastructureError> {
            self.values
                .lock()
                .map_err(|_| memory_error())?
                .insert(account.to_string(), secret.to_string());
            Ok(())
        }

        fn get(&self, account: &str) -> Result<Option<Zeroizing<String>>, InfrastructureError> {
            Ok(self
                .values
                .lock()
                .map_err(|_| memory_error())?
                .get(account)
                .cloned()
                .map(Zeroizing::new))
        }

        fn delete(&self, account: &str) -> Result<(), InfrastructureError> {
            self.values
                .lock()
                .map_err(|_| memory_error())?
                .remove(account);
            Ok(())
        }
    }

    fn memory_error() -> InfrastructureError {
        InfrastructureError::Credential("memory credential store lock failed".to_string())
    }

    #[test]
    fn memory_store_round_trips_and_deletes_credentials_without_debug_exposure() {
        let store = Arc::new(MemorySecureCredentialStore::default());
        let adapter = CommunicationsCredentialAdapter::with_store(store.clone());
        let credential = adapter
            .store(ConnectorKind::WeCom, "fixture-private-value")
            .expect("store");

        assert_eq!(credential.reference, "wecom/default");
        assert_eq!(credential.secret.as_str(), "fixture-private-value");
        assert_eq!(
            adapter
                .load(ConnectorKind::WeCom)
                .expect("load")
                .expect("credential")
                .secret
                .as_str(),
            "fixture-private-value"
        );
        assert!(!format!("{adapter:?}").contains("fixture-private-value"));

        adapter.delete(ConnectorKind::WeCom).expect("delete");
        assert!(adapter
            .load(ConnectorKind::WeCom)
            .expect("load after delete")
            .is_none());
    }

    #[test]
    fn migrates_and_deletes_legacy_wechat_accounts() {
        let store = Arc::new(MemorySecureCredentialStore::default());
        store
            .set("wechat/default", "legacy-private-value")
            .expect("legacy credential");
        let adapter = CommunicationsCredentialAdapter::with_store(store.clone());

        let migrated = adapter
            .load(ConnectorKind::WeChat)
            .expect("load")
            .expect("migrated credential");
        assert_eq!(migrated.reference, "weixin/default");
        assert_eq!(migrated.secret.as_str(), "legacy-private-value");
        assert!(store.get("wechat/default").expect("legacy read").is_none());
        assert!(store.get("weixin/default").expect("current read").is_some());

        store
            .set("wechat/default", "stale-private-value")
            .expect("stale credential");
        store
            .set("weixin/session-contexts", "private-contexts")
            .expect("session contexts");
        adapter.delete(ConnectorKind::WeChat).expect("delete");
        assert!(store.get("wechat/default").expect("legacy read").is_none());
        assert!(store.get("weixin/default").expect("current read").is_none());
        assert!(store
            .get("weixin/session-contexts")
            .expect("session context read")
            .is_none());
    }

    #[test]
    fn account_names_are_stable_and_empty_profiles_use_default() {
        assert_eq!(
            credential_account(ConnectorKind::DingTalk, "Default Bot"),
            "dingtalk/default-bot"
        );
        assert_eq!(
            credential_account(ConnectorKind::Telegram, "  "),
            "telegram/default"
        );
    }
}
