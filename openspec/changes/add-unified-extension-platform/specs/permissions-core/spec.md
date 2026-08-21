## ADDED Requirements

### Requirement: Permissions consumes compiled rule outcomes before template fallback

For a normalized operation, Permissions SHALL apply immutable safety floors, consume the current compiled AuthorizationRule outcome, use the existing policy template/PDP only as fallback when no stronger rule decides, apply only monotonic Hook strengthening, then consult remembered grants and the approval broker for a remaining Ask.

#### Scenario: Rule escalates Trusted template

* WHEN the assigned policy template would Allow but a matching rule requires Ask
* THEN the final pending decision is Ask before grant/user approval processing

#### Scenario: No rule matches

* WHEN no safety floor or compiled rule decides
* THEN existing policy-template/PDP behavior remains authoritative

### Requirement: Decision audit includes rules and Hooks

Permission audit SHALL include normalized operation, risk, safety-floor result, rule-set generation and matched rule ids/sources, template fallback, Hook strengthening, grant lookup, approval scope, and final outcome using redacted bounded fields.

#### Scenario: Operation is denied by extension rule

* WHEN an extension-contributed Deny rule matches
* THEN the audit identifies extension id/version/hash and rule id without exposing secret operation arguments

### Requirement: Existing hard floors remain intact during migration

Feature flags, shadow comparison, migration, or compatibility adapters SHALL NOT remove current explicit-Deny-first behavior, MCP approval floors, timeout fail-closed behavior, grant scope constraints, or Claude permission-hook offline safety.

#### Scenario: Rule engine feature is disabled

* WHEN compiled rule enforcement is disabled
* THEN current Permissions behavior remains available and no operation becomes more permissive due solely to the disabled feature
