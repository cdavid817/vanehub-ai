use super::WorkspaceDomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteWorkspace {
    host: String,
    port: u16,
    user: Option<String>,
    path: String,
    display_name: String,
    uri: String,
}

impl RemoteWorkspace {
    pub(crate) fn new(
        host: &str,
        port: Option<u16>,
        user: Option<&str>,
        path: &str,
        display_name: Option<&str>,
    ) -> Result<Self, WorkspaceDomainError> {
        let host = host.trim().to_string();
        let port = port.unwrap_or(22);
        let path = path.trim().to_string();
        let user = user
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if host.is_empty() || path.is_empty() {
            return Err(WorkspaceDomainError::RemoteWorkspaceIncomplete);
        }
        if port == 0 {
            return Err(WorkspaceDomainError::InvalidRemoteWorkspace);
        }
        let contains_control = host
            .chars()
            .chain(path.chars())
            .chain(user.as_deref().unwrap_or("").chars())
            .any(char::is_control);
        if host.contains('/') || host.contains('\\') || contains_control {
            return Err(WorkspaceDomainError::InvalidRemoteWorkspace);
        }
        let authority = user
            .as_ref()
            .map(|user| format!("{user}@{host}"))
            .unwrap_or_else(|| host.clone());
        let display_name = display_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{host}:{}", display_name_for_remote_path(&path)));
        let port_segment = if port == 22 {
            String::new()
        } else {
            format!(":{port}")
        };
        let uri = format!(
            "ssh://{authority}{port_segment}{}{}",
            if path.starts_with('/') { "" } else { "/" },
            path
        );
        Ok(Self {
            host,
            port,
            user,
            path,
            display_name,
            uri,
        })
    }

    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    pub(crate) fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn display_name(&self) -> &str {
        &self.display_name
    }

    pub(crate) fn uri(&self) -> &str {
        &self.uri
    }
}

fn display_name_for_remote_path(path: &str) -> String {
    path.trim_end_matches('/')
        .split('/')
        .rfind(|value| !value.is_empty())
        .unwrap_or(path)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_workspace_normalizes_identity_uri_and_default_name() {
        let workspace =
            RemoteWorkspace::new("  example.com ", None, Some(" dev "), " work/app/ ", None)
                .expect("remote workspace");

        assert_eq!(workspace.host(), "example.com");
        assert_eq!(workspace.port(), 22);
        assert_eq!(workspace.user(), Some("dev"));
        assert_eq!(workspace.path(), "work/app/");
        assert_eq!(workspace.display_name(), "example.com:app");
        assert_eq!(workspace.uri(), "ssh://dev@example.com/work/app/");
    }

    #[test]
    fn absolute_remote_paths_and_custom_names_keep_the_existing_contract() {
        let workspace = RemoteWorkspace::new(
            "example.com",
            None,
            None,
            "/work/app",
            Some("  Example app  "),
        )
        .expect("remote workspace");

        assert_eq!(workspace.user(), None);
        assert_eq!(workspace.display_name(), "Example app");
        assert_eq!(workspace.uri(), "ssh://example.com/work/app");
    }

    #[test]
    fn non_default_ports_are_preserved_in_uri() {
        let workspace =
            RemoteWorkspace::new("example.com", Some(2222), Some("dev"), "/work/app", None)
                .expect("remote workspace");

        assert_eq!(workspace.port(), 2222);
        assert_eq!(workspace.uri(), "ssh://dev@example.com:2222/work/app");
    }

    #[test]
    fn remote_workspace_rejects_missing_fields_separators_and_controls() {
        assert_eq!(
            RemoteWorkspace::new("", None, None, "/work/app", None),
            Err(WorkspaceDomainError::RemoteWorkspaceIncomplete)
        );
        assert_eq!(
            RemoteWorkspace::new("example.com", None, None, " ", None),
            Err(WorkspaceDomainError::RemoteWorkspaceIncomplete)
        );
        for (host, user, path) in [
            ("bad/host", None, "/work/app"),
            ("bad\\host", None, "/work/app"),
            ("example.com", Some("bad\nuser"), "/work/app"),
            ("example.com", None, "/work/\napp"),
        ] {
            assert_eq!(
                RemoteWorkspace::new(host, None, user, path, None),
                Err(WorkspaceDomainError::InvalidRemoteWorkspace)
            );
        }
    }
}
