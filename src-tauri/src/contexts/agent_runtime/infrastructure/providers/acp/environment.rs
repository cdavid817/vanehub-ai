//! The environment an ACP agent process is started with.
//!
//! The agent inherits the desktop process environment -- it needs PATH, HOME, proxies, and its
//! own vendor's credential variables -- but not another vendor's secrets. A Copilot process has
//! no business seeing `ANTHROPIC_API_KEY`, and a Qwen process has none seeing `GH_TOKEN`. The
//! scrub is by reviewed variable name, per provider; it is not a heuristic over values.

use std::collections::BTreeMap;

/// Credential-bearing variables each provider is documented to read. Everything on this table
/// that does not belong to the launching provider is removed from the child environment.
const PROVIDER_CREDENTIAL_VARIABLES: &[(&str, &[&str])] = &[
    (
        "claude-code",
        &["ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"],
    ),
    ("codex-cli", &["OPENAI_API_KEY", "CODEX_API_KEY"]),
    ("gemini-cli", &["GEMINI_API_KEY", "GOOGLE_API_KEY"]),
    ("opencode", &["OPENCODE_API_KEY"]),
    ("antigravity-cli", &["ANTIGRAVITY_API_KEY"]),
    (
        "qwen-code",
        &["OPENAI_API_KEY", "DASHSCOPE_API_KEY", "QWEN_API_KEY"],
    ),
    ("kimi-cli", &["KIMI_API_KEY", "MOONSHOT_API_KEY"]),
    ("qoder-cli", &["QODER_PERSONAL_ACCESS_TOKEN"]),
    ("codebuddy-code", &["CODEBUDDY_API_KEY"]),
    (
        "copilot-cli",
        &["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"],
    ),
    ("cursor-agent-cli", &["CURSOR_API_KEY"]),
    ("iflow-cli", &["IFLOW_API_KEY"]),
];

/// Prefixes that carry BYOK provider configuration for one vendor and nothing else.
const PROVIDER_CREDENTIAL_PREFIXES: &[(&str, &[&str])] = &[("copilot-cli", &["COPILOT_PROVIDER_"])];

/// The child environment for `provider_id`: the parent environment minus other providers'
/// credential variables, plus the reviewed adapter-derived entries (an account-environment
/// switch, a trace parent). Adapter entries win over inherited ones.
pub(crate) fn child_environment(
    provider_id: &str,
    parent: impl IntoIterator<Item = (String, String)>,
    adapter_entries: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut environment: BTreeMap<String, String> = parent
        .into_iter()
        .filter(|(key, _)| !belongs_to_another_provider(provider_id, key))
        .collect();
    for (key, value) in adapter_entries {
        environment.insert(key.clone(), value.clone());
    }
    environment
}

fn belongs_to_another_provider(provider_id: &str, key: &str) -> bool {
    let owned_by_launcher = PROVIDER_CREDENTIAL_VARIABLES
        .iter()
        .filter(|(id, _)| *id == provider_id)
        .any(|(_, names)| names.contains(&key))
        || PROVIDER_CREDENTIAL_PREFIXES
            .iter()
            .filter(|(id, _)| *id == provider_id)
            .any(|(_, prefixes)| prefixes.iter().any(|prefix| key.starts_with(prefix)));
    if owned_by_launcher {
        return false;
    }
    PROVIDER_CREDENTIAL_VARIABLES
        .iter()
        .filter(|(id, _)| *id != provider_id)
        .any(|(_, names)| names.contains(&key))
        || PROVIDER_CREDENTIAL_PREFIXES
            .iter()
            .filter(|(id, _)| *id != provider_id)
            .any(|(_, prefixes)| prefixes.iter().any(|prefix| key.starts_with(prefix)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent() -> Vec<(String, String)> {
        [
            ("PATH", "/usr/bin"),
            ("HOME", "/home/user"),
            ("ANTHROPIC_API_KEY", "anthropic-sentinel"),
            ("GH_TOKEN", "github-sentinel"),
            ("COPILOT_PROVIDER_BASE_URL", "https://byok.example"),
            ("CODEBUDDY_API_KEY", "codebuddy-sentinel"),
            ("HTTPS_PROXY", "http://proxy.local"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
    }

    #[test]
    fn other_providers_secrets_are_removed_and_own_ones_are_kept() {
        let copilot = child_environment("copilot-cli", parent(), &BTreeMap::new());
        assert_eq!(
            copilot.get("GH_TOKEN").map(String::as_str),
            Some("github-sentinel")
        );
        assert_eq!(
            copilot.get("COPILOT_PROVIDER_BASE_URL").map(String::as_str),
            Some("https://byok.example")
        );
        assert!(!copilot.contains_key("ANTHROPIC_API_KEY"));
        assert!(!copilot.contains_key("CODEBUDDY_API_KEY"));
        assert_eq!(copilot.get("PATH").map(String::as_str), Some("/usr/bin"));
        assert_eq!(
            copilot.get("HTTPS_PROXY").map(String::as_str),
            Some("http://proxy.local")
        );

        let codebuddy = child_environment("codebuddy-code", parent(), &BTreeMap::new());
        assert_eq!(
            codebuddy.get("CODEBUDDY_API_KEY").map(String::as_str),
            Some("codebuddy-sentinel")
        );
        assert!(!codebuddy.contains_key("GH_TOKEN"));
        assert!(!codebuddy.contains_key("COPILOT_PROVIDER_BASE_URL"));
        assert!(!codebuddy.contains_key("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn adapter_entries_override_inherited_values() {
        let adapter = BTreeMap::from([
            (
                "CODEBUDDY_INTERNET_ENVIRONMENT".to_string(),
                "internal".to_string(),
            ),
            ("HOME".to_string(), "/tmp/scoped".to_string()),
        ]);
        let environment = child_environment("codebuddy-code", parent(), &adapter);
        assert_eq!(
            environment
                .get("CODEBUDDY_INTERNET_ENVIRONMENT")
                .map(String::as_str),
            Some("internal")
        );
        assert_eq!(
            environment.get("HOME").map(String::as_str),
            Some("/tmp/scoped")
        );
    }

    #[test]
    fn a_shared_variable_stays_when_the_launcher_owns_it_too() {
        // OPENAI_API_KEY is read by both Codex and Qwen (OpenAI-compatible endpoints); launching
        // Qwen keeps it, launching Kimi drops it.
        let parent = vec![("OPENAI_API_KEY".to_string(), "x".to_string())];
        assert!(
            child_environment("qwen-code", parent.clone(), &BTreeMap::new())
                .contains_key("OPENAI_API_KEY")
        );
        assert!(
            !child_environment("kimi-cli", parent, &BTreeMap::new()).contains_key("OPENAI_API_KEY")
        );
    }
}
