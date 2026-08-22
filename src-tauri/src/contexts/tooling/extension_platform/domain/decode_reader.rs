// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! A consuming reader over one mapping in the parsed document.
//!
//! Every field is taken by name, and `finish` rejects whatever was left. That is what makes "no
//! blanket deserialization" mechanical rather than a habit: a field nobody read is a field the
//! author wrote and this build ignored, which for a security-relevant key is the difference
//! between refusing a package and quietly running it with unreviewed intent.

use super::{DecodeReason, ManifestDecodeError};
use vanehub_bounded_yaml::BoundedYamlValue;

pub(crate) struct MappingReader<'a> {
    /// Dotted path to this mapping, used to locate every diagnostic it produces.
    path: String,
    entries: &'a [(String, BoundedYamlValue)],
    consumed: Vec<bool>,
}

impl<'a> MappingReader<'a> {
    /// Opens a reader over a value that must be a mapping.
    pub(crate) fn open(
        path: impl Into<String>,
        value: &'a BoundedYamlValue,
    ) -> Result<Self, ManifestDecodeError> {
        let path = path.into();
        let Some(entries) = value.as_mapping() else {
            return Err(ManifestDecodeError::new(
                path,
                DecodeReason::ExpectedMapping,
            ));
        };
        Ok(Self {
            path,
            entries,
            consumed: vec![false; entries.len()],
        })
    }

    /// `<this mapping>.<field>`, or just `<field>` at the document root.
    pub(crate) fn child_path(&self, field: &str) -> String {
        if self.path.is_empty() {
            field.to_string()
        } else {
            format!("{}.{field}", self.path)
        }
    }

    fn take(&mut self, field: &str) -> Option<&'a BoundedYamlValue> {
        let index = self.entries.iter().position(|(key, _)| key == field)?;
        self.consumed[index] = true;
        Some(&self.entries[index].1)
    }

    pub(crate) fn optional_value(&mut self, field: &str) -> Option<&'a BoundedYamlValue> {
        self.take(field)
    }

    pub(crate) fn required_value(
        &mut self,
        field: &str,
    ) -> Result<&'a BoundedYamlValue, ManifestDecodeError> {
        self.take(field)
            .ok_or_else(|| ManifestDecodeError::new(self.child_path(field), DecodeReason::Missing))
    }

    pub(crate) fn optional_scalar(
        &mut self,
        field: &str,
    ) -> Result<Option<&'a str>, ManifestDecodeError> {
        let Some(value) = self.take(field) else {
            return Ok(None);
        };
        value
            .as_scalar()
            .ok_or_else(|| {
                ManifestDecodeError::new(self.child_path(field), DecodeReason::ExpectedScalar)
            })
            .map(Some)
    }

    pub(crate) fn required_scalar(&mut self, field: &str) -> Result<&'a str, ManifestDecodeError> {
        let path = self.child_path(field);
        let value = self.required_value(field)?;
        let scalar = value
            .as_scalar()
            .ok_or_else(|| ManifestDecodeError::new(&path, DecodeReason::ExpectedScalar))?;
        if scalar.is_empty() {
            return Err(ManifestDecodeError::new(path, DecodeReason::Empty));
        }
        Ok(scalar)
    }

    /// A list of plain values, such as `capabilities` or `activation_events`.
    ///
    /// An absent field is an empty list rather than an error: every list in this manifest is
    /// optional, and `key:` with nothing under it parses as an empty mapping, which callers
    /// reasonably mean as "none".
    pub(crate) fn scalar_sequence(
        &mut self,
        field: &str,
    ) -> Result<Vec<&'a str>, ManifestDecodeError> {
        let Some(value) = self.take(field) else {
            return Ok(Vec::new());
        };
        match value {
            BoundedYamlValue::Sequence(items) => Ok(items.iter().map(String::as_str).collect()),
            BoundedYamlValue::Mapping(entries) if entries.is_empty() => Ok(Vec::new()),
            _ => Err(ManifestDecodeError::new(
                self.child_path(field),
                DecodeReason::ExpectedScalarSequence,
            )),
        }
    }

    /// An id-keyed collection: `<field>: { <id>: { .. }, .. }`.
    ///
    /// A sequence here is the list-of-records shape, reported as such rather than as a type error
    /// — it is what an author coming from another extension ecosystem writes first, and "expects a
    /// mapping" would not tell them what to do about it.
    pub(crate) fn keyed_collection(
        &mut self,
        field: &str,
    ) -> Result<Vec<(&'a str, &'a BoundedYamlValue)>, ManifestDecodeError> {
        let path = self.child_path(field);
        let Some(value) = self.take(field) else {
            return Ok(Vec::new());
        };
        match value {
            BoundedYamlValue::Mapping(entries) => Ok(entries
                .iter()
                .map(|(key, child)| (key.as_str(), child))
                .collect()),
            BoundedYamlValue::Sequence(_) => {
                Err(ManifestDecodeError::new(path, DecodeReason::ListOfRecords))
            }
            BoundedYamlValue::Scalar(_) => Err(ManifestDecodeError::new(
                path,
                DecodeReason::ExpectedMapping,
            )),
        }
    }

    /// Rejects every key that no `required_*`/`optional_*` call claimed.
    pub(crate) fn finish(self) -> Result<(), ManifestDecodeError> {
        for (index, (key, _)) in self.entries.iter().enumerate() {
            if !self.consumed[index] {
                return Err(ManifestDecodeError::new(
                    self.child_path(key),
                    DecodeReason::UnknownField,
                ));
            }
        }
        Ok(())
    }
}

/// Enforces a per-collection ceiling. The node budget already bounds the document; this gives the
/// author a diagnostic that names the collection instead of the whole file.
pub(crate) fn bound<T>(
    field: &str,
    items: Vec<T>,
    limit: usize,
) -> Result<Vec<T>, ManifestDecodeError> {
    if items.len() > limit {
        return Err(ManifestDecodeError::new(
            field,
            DecodeReason::TooMany { limit },
        ));
    }
    Ok(items)
}
