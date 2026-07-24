use super::dto;
use crate::contexts::ssh_connections::api::{
    HostKeyChallengeKind, SaveSshConnectionRequest, SshAuthMode, SshConnectionTestStatus,
};
use crate::contexts::ssh_connections::domain::runtime::HostKeyChallenge;
use crate::contexts::ssh_connections::domain::SshConnectionProfile;

pub(crate) fn connection_to_dto(profile: SshConnectionProfile) -> dto::SshConnection {
    let has_password = profile.has_password();
    let host_trust = profile.host_trust.map(|trust| dto::SshHostTrustMetadata {
        host: trust.host,
        port: trust.port,
        algorithm: trust.algorithm,
        fingerprint: trust.fingerprint,
        confirmed_at: trust.confirmed_at,
    });
    dto::SshConnection {
        id: profile.id,
        name: profile.name,
        host: profile.host,
        port: profile.port,
        user: profile.user,
        default_path: profile.default_path,
        auth_mode: profile.auth_mode.as_str().to_string(),
        key_path: profile.key_path,
        has_password,
        revision: profile.revision,
        host_trust,
        test_status: profile.test_status.as_str().to_string(),
        last_connected_at: profile.last_connected_at,
        last_error: profile.last_error,
        created_at: profile.created_at,
        updated_at: profile.updated_at,
    }
}

pub(crate) fn mutation_from_dto(
    input: dto::SaveSshConnectionInput,
) -> Result<SaveSshConnectionRequest, crate::contexts::ssh_connections::api::SshConnectionsError> {
    Ok(SaveSshConnectionRequest {
        name: input.name,
        host: input.host,
        port: input.port,
        user: input.user,
        default_path: input.default_path,
        auth_mode: SshAuthMode::parse(&input.auth_mode)?,
        key_path: input.key_path,
        password: input.password,
    })
}

pub(crate) fn test_result_to_dto(
    result: crate::contexts::ssh_connections::application::SshConnectionTestResult,
) -> dto::SshConnectionTestResult {
    dto::SshConnectionTestResult {
        status: test_status(result.status).to_string(),
        message: result.message,
        tested_at: result.tested_at,
    }
}

fn test_status(status: SshConnectionTestStatus) -> &'static str {
    status.as_str()
}

pub(crate) fn host_key_challenge_to_dto(challenge: HostKeyChallenge) -> dto::SshHostKeyChallenge {
    dto::SshHostKeyChallenge {
        connection_id: challenge.connection_key.connection_id,
        revision: challenge.connection_key.revision,
        host: challenge.host,
        port: challenge.port,
        kind: match challenge.kind {
            HostKeyChallengeKind::FirstSeen => "first-seen",
            HostKeyChallengeKind::Changed => "changed",
        }
        .to_string(),
        algorithm: challenge.evidence.algorithm,
        fingerprint: challenge.evidence.fingerprint,
        previous_fingerprint: challenge.previous_fingerprint,
    }
}
