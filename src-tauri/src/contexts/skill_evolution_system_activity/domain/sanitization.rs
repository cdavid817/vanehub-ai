use super::{ActivityEnvelopeError, ActivityNavigation, ActivityPayloadV1};
use unicode_normalization::UnicodeNormalization;

pub(crate) fn sanitize_text(
    value: &str,
    field: &'static str,
    max_chars: usize,
) -> Result<String, ActivityEnvelopeError> {
    let normalized: String = value.trim().nfc().collect();
    if normalized.is_empty()
        || normalized.chars().count() > max_chars
        || normalized.chars().any(is_prohibited_scalar)
        || !normalized.chars().all(is_safe_identity_scalar)
    {
        return Err(ActivityEnvelopeError::InvalidField(field));
    }
    Ok(normalized)
}

fn is_safe_identity_scalar(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '-' | '_' | '.' | ':' | '@')
}

pub(super) fn sanitize_navigation(
    navigation: &mut ActivityNavigation,
    max_chars: usize,
) -> Result<(), ActivityEnvelopeError> {
    navigation.stable_id = sanitize_text(&navigation.stable_id, "navigation.stable_id", max_chars)?;
    if let Some(child_id) = &mut navigation.child_id {
        *child_id = sanitize_text(child_id, "navigation.child_id", max_chars)?;
    }
    Ok(())
}

pub(super) fn sanitize_payload(
    payload: &mut ActivityPayloadV1,
    max_chars: usize,
) -> Result<(), ActivityEnvelopeError> {
    match payload {
        ActivityPayloadV1::NavigationList { links } => {
            for navigation in links {
                sanitize_navigation(navigation, max_chars)?;
            }
        }
        ActivityPayloadV1::SupersessionNotice { prior_event_id } => {
            *prior_event_id = sanitize_text(prior_event_id, "payload.prior_event_id", max_chars)?;
        }
        ActivityPayloadV1::StatusCard { .. }
        | ActivityPayloadV1::StageTimeline { .. }
        | ActivityPayloadV1::CheckSummary { .. }
        | ActivityPayloadV1::MetricSummary { .. } => {}
    }
    Ok(())
}

fn is_prohibited_scalar(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}
