//! SQLite adapters for the authorization-rule store, and the schema they are written against.

mod schema;
mod sqlite_rules;
#[cfg(test)]
mod sqlite_rules_tests;

pub(crate) use schema::apply_authorization_rule_schema;
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_rules::{SqliteActiveRuleSetRepository, SqliteRuleSetRepository};

/// The digest of a rule set's canonical bytes.
///
/// Hashing lives here rather than in the domain, which is the split the rest of this repository
/// uses: the domain decides what the bytes are and infrastructure decides how they are hashed, so
/// the rule model carries no dependency on a hash implementation.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn rule_set_digest(
    rules: &[crate::contexts::permissions::domain::rules::AuthorizationRule],
) -> Result<crate::contexts::permissions::domain::rules::RuleSetDigest, String> {
    let bytes = crate::contexts::permissions::domain::rules::canonical_rule_set_bytes(rules);
    let hex = crate::platform::content_address::sha256_hex(&bytes);
    crate::contexts::permissions::domain::rules::RuleSetDigest::parse(&hex)
        .map_err(|error| error.code().to_string())
}
