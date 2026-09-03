use super::{dto, mapper};
use crate::contexts::execution_observability::EvaluationApi;
use tauri::State;

/// Mirrors `contexts::operations::application::mission_control`'s own cursor-shaped pagination:
/// the cursor is honestly just the offset re-exposed as an opaque string -- the repository beneath
/// `EvaluationApi::list` is plain SQL OFFSET/LIMIT, not a keyset cursor -- but every other paginated
/// list surface already reachable from the frontend (`MissionControlPage`, artifact listing,
/// evidence, personalization memory, skill overlay history) reads `{ items, nextCursor }` to its
/// caller. Matching that keeps Evaluation consistent with the rest of the app instead of being the
/// one list surface that leaks a raw offset/limit pair a caller has to reconstruct paging from
/// itself. `DEFAULT_LIMIT`/`MAX_LIMIT` copy Mission Control's own constants verbatim.
const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 50;

#[tauri::command]
pub(crate) fn list_evaluation_arenas(
    api: State<'_, EvaluationApi>,
    cursor: Option<String>,
    limit: Option<usize>,
) -> Result<dto::EvaluationArenaPage, String> {
    let offset = parse_offset(cursor.as_deref())?;
    let bounded_limit = resolve_limit(limit);
    // Fetches one extra row to learn whether a next page exists without a separate COUNT query --
    // the same trick Mission Control's own `scoped` closure uses. `MAX_LIMIT` (50) stays well under
    // the repository's own hard `MAX_PAGE` (100, `evaluation_repository.rs`), so this `+ 1` request
    // is never silently clamped back down before `has_more` gets to see it.
    let arenas = api
        .list(offset, bounded_limit + 1)
        .map_err(mapper::safe_error)?;
    let (page, has_more) = paginate(arenas, bounded_limit);
    Ok(dto::EvaluationArenaPage {
        items: page.into_iter().map(mapper::arena).collect(),
        next_cursor: has_more.then(|| (offset + bounded_limit).to_string()),
    })
}

fn parse_offset(cursor: Option<&str>) -> Result<usize, String> {
    cursor
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| "invalid evaluation cursor".to_string())
        })
        .transpose()
        .map(|offset| offset.unwrap_or(0))
}

fn resolve_limit(limit: Option<usize>) -> usize {
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

fn paginate<T>(mut items: Vec<T>, limit: usize) -> (Vec<T>, bool) {
    let has_more = items.len() > limit;
    items.truncate(limit);
    (items, has_more)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_offset_defaults_to_zero_when_no_cursor_is_given() {
        assert_eq!(parse_offset(None), Ok(0));
    }

    #[test]
    fn parse_offset_reads_a_valid_cursor() {
        assert_eq!(parse_offset(Some("40")), Ok(40));
    }

    #[test]
    fn parse_offset_rejects_a_non_numeric_cursor_with_a_safe_message() {
        assert_eq!(
            parse_offset(Some("not-a-number")),
            Err("invalid evaluation cursor".to_string())
        );
    }

    #[test]
    fn resolve_limit_defaults_and_clamps_into_bounds() {
        assert_eq!(resolve_limit(None), DEFAULT_LIMIT);
        assert_eq!(resolve_limit(Some(0)), 1);
        assert_eq!(resolve_limit(Some(1_000)), MAX_LIMIT);
        assert_eq!(resolve_limit(Some(35)), 35);
    }

    #[test]
    fn paginate_reports_has_more_only_when_the_probe_row_was_returned() {
        let (page, has_more) = paginate(vec![1, 2, 3], 2);
        assert_eq!(page, vec![1, 2]);
        assert!(has_more);

        let (page, has_more) = paginate(vec![1, 2], 2);
        assert_eq!(page, vec![1, 2]);
        assert!(!has_more);
    }
}
