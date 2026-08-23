// The runtime that produces these lands with Task Groups 4 and 5; see the domain's `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Emitting runtime diagnostics through the unified logging service.
//!
//! No new store. Diagnostics are bounded, high-volume, and interesting for minutes rather than
//! forever; the unified logging service already has rotation, levels, and redaction, and a second
//! store would be a second retention policy to get wrong.
//!
//! Everything written here is host-owned: a code from a closed set, the subject's ids, and integer
//! measurements. `redact_log_fields` still runs over it — not because anything here could carry a
//! secret, but because a sink that skipped redaction would be the one someone later extended with
//! a field that could.

use crate::contexts::tooling::extension_platform::application::RuntimeDiagnosticSink;
use crate::contexts::tooling::extension_platform::domain::{
    DiagnosticMeasure, SafeExtensionRuntimeDiagnostic,
};
use crate::platform::logging::{redact_log_fields, write_message, LogLevel};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The log category extension-runtime diagnostics are filed under.
pub(crate) const RUNTIME_DIAGNOSTIC_CATEGORY: &str = "extension-runtime";

pub(crate) struct LoggingRuntimeDiagnosticSink {
    log_dir: PathBuf,
}

impl LoggingRuntimeDiagnosticSink {
    pub(crate) fn new(log_dir: PathBuf) -> Self {
        Self { log_dir }
    }
}

impl RuntimeDiagnosticSink for LoggingRuntimeDiagnosticSink {
    fn emit(&self, diagnostic: &SafeExtensionRuntimeDiagnostic) -> Result<(), String> {
        let mut context = BTreeMap::new();
        context.insert(
            "extension".to_string(),
            diagnostic.extension().as_str().to_string(),
        );
        if let Some(snapshot) = diagnostic.snapshot() {
            context.insert("snapshot".to_string(), snapshot.as_str().to_string());
        }
        for (measure, value) in diagnostic.measurements() {
            context.insert(measure.as_str().to_string(), value.to_string());
        }

        // The message is the code and nothing else. There is no interpolated detail, because there
        // is no detail: the diagnostic type cannot carry a string the extension chose.
        let (message, context) = redact_log_fields(diagnostic.code().as_str(), context);
        let level = if diagnostic.code().is_failure() {
            LogLevel::Warn
        } else {
            LogLevel::Debug
        };

        write_message(
            &self.log_dir,
            level,
            RUNTIME_DIAGNOSTIC_CATEGORY,
            &message,
            context,
        )
        .map_err(|error| error.to_string())
    }
}

/// Every measure this sink knows how to render, in a fixed order.
///
/// Present so a reader of a log line can be told which keys are possible without having to find
/// the domain enum; the ordering matches `BTreeMap`'s, which is what the log actually writes.
pub(crate) fn rendered_measure_keys() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> =
        crate::contexts::tooling::extension_platform::domain::ALL_DIAGNOSTIC_MEASURES
            .iter()
            .map(|measure: &DiagnosticMeasure| measure.as_str())
            .collect();
    keys.sort_unstable();
    keys
}
