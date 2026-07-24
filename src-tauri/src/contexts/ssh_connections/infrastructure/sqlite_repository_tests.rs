use super::sqlite_repository::SqliteSshConnectionRepository;
use crate::contexts::ssh_connections::application::SshConnectionRepository;
use crate::contexts::ssh_connections::domain::{
    SshAuthMode, SshConnectionProfile, SshConnectionTestStatus, SshHostTrustMetadata,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

#[test]
fn repository_round_trips_profiles_without_password_plaintext() {
    let directory = TempDirectory::new("ssh-connections-repository");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteSshConnectionRepository::new(database.clone());
    let profile = SshConnectionProfile {
        id: "ssh-fixture".to_string(),
        name: "Fixture".to_string(),
        host: "dev.example.com".to_string(),
        port: 2222,
        user: "cdavid".to_string(),
        default_path: "/work/app".to_string(),
        auth_mode: SshAuthMode::Password,
        key_path: None,
        credential_ref: Some("ssh-connection/ssh-fixture".to_string()),
        revision: 3,
        host_trust: Some(SshHostTrustMetadata {
            host: "dev.example.com".to_string(),
            port: 2222,
            algorithm: "ssh-ed25519".to_string(),
            fingerprint: "SHA256:fixture".to_string(),
            confirmed_at: "2026-07-21T00:00:00.000Z".to_string(),
        }),
        test_status: SshConnectionTestStatus::NotTested,
        last_connected_at: None,
        last_error: None,
        created_at: "2026-07-21T00:00:00.000Z".to_string(),
        updated_at: "2026-07-21T00:00:00.000Z".to_string(),
    };

    repository.insert(&profile).expect("insert");
    let loaded = repository
        .find("ssh-fixture")
        .expect("find")
        .expect("profile");

    assert_eq!(loaded, profile);
    let raw = database.connection().expect("connection").query_row(
        "SELECT group_concat(COALESCE(name, '') || COALESCE(host, '') || COALESCE(credential_ref, ''), '|') FROM ssh_connections",
        [],
        |row| row.get::<_, String>(0),
    ).expect("raw");
    assert!(!raw.contains("private-password"));
}
