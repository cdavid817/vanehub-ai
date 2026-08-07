//! How long a resolved approval decision should be remembered.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Scope {
    Once,
    Session,
    Project,
    Global,
}

impl Scope {
    /// Only `Session`/`Project`/`Global` resolutions persist a grant
    /// (`permissions-core`'s "Remembered grants are consulted before falling back to
    /// templates"); `Once` never does.
    pub(crate) fn is_remembered(self) -> bool {
        !matches!(self, Scope::Once)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn once_is_not_remembered() {
        assert!(!Scope::Once.is_remembered());
    }

    #[test]
    fn session_project_and_global_are_remembered() {
        assert!(Scope::Session.is_remembered());
        assert!(Scope::Project.is_remembered());
        assert!(Scope::Global.is_remembered());
    }
}
