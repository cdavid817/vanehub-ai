use super::position_conversion::{AgentPosition, PositionConversionError, PositionConverter};
use crate::contexts::code_intelligence::domain::models::PositionEncoding;
use lsp_types::{Position, Range};

const SOURCE: &str = "plain\na😀e\u{301}z\nlast";

#[test]
fn one_based_agent_coordinates_convert_to_zero_based_lsp_coordinates() {
    let utf16 = PositionConverter::new(SOURCE, PositionEncoding::Utf16);
    let utf8 = PositionConverter::new(SOURCE, PositionEncoding::Utf8);

    assert_eq!(
        utf16.agent_to_lsp(AgentPosition::new(2, 3)),
        Ok(Position::new(1, 3))
    );
    assert_eq!(
        utf8.agent_to_lsp(AgentPosition::new(2, 3)),
        Ok(Position::new(1, 5))
    );
}

#[test]
fn surrogate_pairs_and_combining_characters_preserve_scalar_boundaries() {
    let utf16 = PositionConverter::new(SOURCE, PositionEncoding::Utf16);

    assert_eq!(
        utf16.agent_to_lsp(AgentPosition::new(2, 5)),
        Ok(Position::new(1, 5))
    );
    assert_eq!(
        utf16.lsp_to_agent(Position::new(1, 5)),
        Ok(AgentPosition::new(2, 5))
    );
    assert_eq!(
        utf16.lsp_to_agent(Position::new(1, 2)),
        Err(PositionConversionError::InvalidCharacterBoundary)
    );
}

#[test]
fn lsp_ranges_become_one_based_ranges_with_exclusive_end_positions() {
    let converter = PositionConverter::new(SOURCE, PositionEncoding::Utf16);

    let normalized = converter
        .range_to_normalized(Range::new(Position::new(1, 1), Position::new(1, 3)))
        .expect("normalized range");

    assert_eq!(normalized.start_line, 2);
    assert_eq!(normalized.start_column, 2);
    assert_eq!(normalized.end_line, 2);
    assert_eq!(normalized.end_column, 3);
}

#[test]
fn utf8_offsets_must_land_on_code_point_boundaries() {
    let converter = PositionConverter::new(SOURCE, PositionEncoding::Utf8);

    assert_eq!(
        converter.lsp_to_agent(Position::new(1, 2)),
        Err(PositionConversionError::InvalidCharacterBoundary)
    );
    assert_eq!(
        converter.lsp_to_agent(Position::new(1, 5)),
        Ok(AgentPosition::new(2, 3))
    );
}

#[test]
fn invalid_agent_positions_and_reversed_ranges_fail_closed() {
    let converter = PositionConverter::new(SOURCE, PositionEncoding::Utf16);

    for position in [
        AgentPosition::new(0, 1),
        AgentPosition::new(1, 0),
        AgentPosition::new(9, 1),
        AgentPosition::new(1, 99),
    ] {
        assert!(converter.agent_to_lsp(position).is_err());
    }
    assert_eq!(
        converter.range_to_normalized(Range::new(Position::new(1, 3), Position::new(1, 1),)),
        Err(PositionConversionError::ReversedRange)
    );
}

#[test]
fn end_of_line_and_final_line_positions_are_supported() {
    let converter = PositionConverter::new(SOURCE, PositionEncoding::Utf16);

    assert_eq!(
        converter.agent_to_lsp(AgentPosition::new(1, 6)),
        Ok(Position::new(0, 5))
    );
    assert_eq!(
        converter.agent_to_lsp(AgentPosition::new(3, 5)),
        Ok(Position::new(2, 4))
    );
}
