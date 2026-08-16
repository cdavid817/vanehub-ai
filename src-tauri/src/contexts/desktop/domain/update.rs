use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum UpdateChannel {
    Stable,
    Preview,
}

impl UpdateChannel {
    pub(crate) fn default_for(version: &str) -> Self {
        if version.contains('-') {
            Self::Preview
        } else {
            Self::Stable
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Version {
    numbers: [u64; 3],
    prerelease: Vec<String>,
}

impl Version {
    fn parse(value: &str) -> Option<Self> {
        let value = value.strip_prefix('v').unwrap_or(value);
        let (core, suffix) = value.split_once('-').unwrap_or((value, ""));
        let parts = core.split('.').collect::<Vec<_>>();
        if parts.len() != 3
            || parts
                .iter()
                .any(|part| part.is_empty() || (part.len() > 1 && part.starts_with('0')))
        {
            return None;
        }
        let numbers = [
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        ];
        let prerelease = if suffix.is_empty() {
            Vec::new()
        } else {
            suffix.split('.').map(str::to_owned).collect()
        };
        if prerelease.iter().any(|part| {
            part.is_empty()
                || (part.len() > 1
                    && part.starts_with('0')
                    && part.chars().all(|ch| ch.is_ascii_digit()))
                || !part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        }) {
            return None;
        }
        Some(Self {
            numbers,
            prerelease,
        })
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.numbers.cmp(&other.numbers).then_with(|| {
            match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                _ => compare_prerelease(&self.prerelease, &other.prerelease),
            }
        })
    }
}

fn compare_prerelease(left: &[String], right: &[String]) -> Ordering {
    for (left_part, right_part) in left.iter().zip(right) {
        let ordering = match (left_part.parse::<u64>(), right_part.parse::<u64>()) {
            (Ok(left_number), Ok(right_number)) => left_number.cmp(&right_number),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => left_part.cmp(right_part),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

pub(crate) fn admits_update(current: &str, candidate: &str, channel: UpdateChannel) -> bool {
    let Some(current) = Version::parse(current) else {
        return false;
    };
    let Some(candidate) = Version::parse(candidate) else {
        return false;
    };
    if channel == UpdateChannel::Stable && !candidate.prerelease.is_empty() {
        return false;
    }
    candidate.compare(&current) == Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_downgrade_and_stable_prerelease() {
        assert!(!admits_update("1.2.0", "1.1.9", UpdateChannel::Preview));
        assert!(!admits_update(
            "1.0.0",
            "1.1.0-preview.1",
            UpdateChannel::Stable
        ));
        assert!(admits_update(
            "1.0.0-preview.1",
            "1.0.0-preview.2",
            UpdateChannel::Preview
        ));
        assert!(admits_update(
            "1.0.0-preview.2",
            "1.0.0-preview.10",
            UpdateChannel::Preview
        ));
    }

    #[test]
    fn derives_channel_without_persistence_migration() {
        assert_eq!(
            UpdateChannel::default_for("0.1.0-preview.1"),
            UpdateChannel::Preview
        );
        assert_eq!(UpdateChannel::default_for("1.0.0"), UpdateChannel::Stable);
    }
}
