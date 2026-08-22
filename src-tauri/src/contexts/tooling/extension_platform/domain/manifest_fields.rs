// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! The fields a manifest may contain, at every path.
//!
//! One checked-in list, cross-checked against two things that would otherwise drift apart: the
//! decoder, which refuses anything not in it, and the published JSON Schema, which tells an
//! editor the same story. Neither is generated from this — the decoder stays explicit and the
//! schema stays hand-written and readable — but a test compares all three, so adding a field in
//! one place and forgetting the others fails rather than ships.
//!
//! `required` is the decoder's notion: a field it errors on when absent. Optional here means the
//! decoder has a documented default, not that the field is meaningless.

/// One mapping shape in the manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldSet {
    /// Dotted path, with `*` standing for an id key in a keyed collection.
    pub(crate) path: &'static str,
    pub(crate) required: &'static [&'static str],
    pub(crate) optional: &'static [&'static str],
}

impl FieldSet {
    pub(crate) fn contains(&self, field: &str) -> bool {
        self.required.contains(&field) || self.optional.contains(&field)
    }

    pub(crate) fn all(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.required.iter().chain(self.optional.iter()).copied()
    }
}

pub(crate) const MANIFEST_FIELDS: &[FieldSet] = &[
    FieldSet {
        path: "",
        required: &[
            "schema_version",
            "id",
            "display_name",
            "publisher",
            "version",
            "min_vanehub_version",
        ],
        optional: &[
            "description",
            "license",
            "runtime",
            "activation_events",
            "requires",
            "permissions",
            "contributes",
        ],
    },
    FieldSet {
        path: "runtime",
        required: &["kind"],
        optional: &["entry", "trust_profile"],
    },
    FieldSet {
        path: "requires",
        required: &[],
        optional: &["extensions", "skills"],
    },
    FieldSet {
        path: "requires.extensions.*",
        required: &["version"],
        optional: &["optional"],
    },
    FieldSet {
        path: "requires.skills.*",
        required: &["version"],
        optional: &["optional"],
    },
    FieldSet {
        path: "permissions",
        required: &[],
        optional: &["filesystem", "network", "process", "secrets"],
    },
    FieldSet {
        path: "permissions.filesystem",
        required: &[],
        optional: &["read", "write"],
    },
    FieldSet {
        path: "permissions.network",
        required: &[],
        optional: &["origins"],
    },
    FieldSet {
        path: "contributes",
        required: &[],
        optional: &[
            "tools",
            "skills",
            "mcp_definitions",
            "modes",
            "hooks",
            "authorization_rules",
            "connectors",
            "configuration",
            "transforms",
        ],
    },
    FieldSet {
        path: "contributes.tools.*",
        required: &["display_name", "handler"],
        optional: &["description", "input_schema", "output_schema"],
    },
    FieldSet {
        path: "contributes.skills.*",
        required: &["path"],
        optional: &[],
    },
    FieldSet {
        path: "contributes.mcp_definitions.*",
        required: &["display_name", "transport"],
        optional: &[],
    },
    FieldSet {
        path: "contributes.mcp_definitions.*.transport",
        required: &["kind"],
        // Read per transport kind: `command`/`args`/`env_keys` for stdio, `url`/`header_keys` for
        // http. Listed together because the reader takes them from one mapping, and a field
        // belonging to the other kind is refused by `finish` rather than silently ignored.
        optional: &["command", "args", "env_keys", "url", "header_keys"],
    },
    FieldSet {
        path: "contributes.modes.*",
        required: &["display_name", "strategy"],
        optional: &[
            "default_policy_template",
            "required_tool_groups",
            "required_skills",
            "required_hooks",
        ],
    },
    FieldSet {
        path: "contributes.hooks.*",
        required: &["event", "handler"],
        optional: &["matcher", "failure_mode", "priority"],
    },
    FieldSet {
        path: "contributes.hooks.*.handler",
        required: &["kind"],
        optional: &["entry", "tool"],
    },
    FieldSet {
        path: "contributes.authorization_rules.*",
        required: &["operation", "effect", "risk"],
        optional: &["matcher", "allowed_scopes"],
    },
    FieldSet {
        path: "contributes.connectors.*",
        required: &["display_name", "type", "driver", "auth_strategy"],
        optional: &["capabilities"],
    },
    FieldSet {
        path: "contributes.configuration.*",
        required: &["schema"],
        optional: &[],
    },
    FieldSet {
        path: "contributes.transforms.*",
        required: &["event", "handler"],
        optional: &[],
    },
];

pub(crate) fn field_set(path: &str) -> Option<&'static FieldSet> {
    MANIFEST_FIELDS.iter().find(|set| set.path == path)
}
