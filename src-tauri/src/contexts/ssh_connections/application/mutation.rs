use super::super::domain::SshConnectionDraft;
use super::SshConnectionMutation;

pub(super) fn draft_from_mutation(mutation: &SshConnectionMutation) -> SshConnectionDraft {
    SshConnectionDraft {
        name: mutation.name.clone(),
        host: mutation.host.clone(),
        port: mutation.port,
        user: mutation.user.clone(),
        default_path: mutation.default_path.clone(),
        auth_mode: mutation.auth_mode,
        key_path: mutation.key_path.clone(),
    }
}

pub(super) fn normalized_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
