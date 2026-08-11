use crate::contexts::code_intelligence::domain::models::{NormalizedRange, PositionEncoding};
use lsp_types::{Position, Range};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AgentPosition {
    pub(crate) line: u32,
    pub(crate) column: u32,
}

impl AgentPosition {
    pub(crate) const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PositionConversionError {
    #[error("position coordinates must be one-based")]
    ZeroBasedAgentPosition,
    #[error("position line is outside the document")]
    LineOutOfRange,
    #[error("position column is outside the line")]
    ColumnOutOfRange,
    #[error("position does not fall on a character boundary")]
    InvalidCharacterBoundary,
    #[error("position range is reversed")]
    ReversedRange,
    #[error("position coordinate exceeds the protocol range")]
    CoordinateOverflow,
}

pub(crate) struct PositionConverter<'a> {
    lines: Vec<&'a str>,
    encoding: PositionEncoding,
}

impl<'a> PositionConverter<'a> {
    pub(crate) fn new(text: &'a str, encoding: PositionEncoding) -> Self {
        Self {
            lines: text
                .split('\n')
                .map(|line| line.strip_suffix('\r').unwrap_or(line))
                .collect(),
            encoding,
        }
    }

    pub(crate) fn agent_to_lsp(
        &self,
        position: AgentPosition,
    ) -> Result<Position, PositionConversionError> {
        if position.line == 0 || position.column == 0 {
            return Err(PositionConversionError::ZeroBasedAgentPosition);
        }
        let line_index = usize::try_from(position.line - 1)
            .map_err(|_| PositionConversionError::CoordinateOverflow)?;
        let line = self
            .lines
            .get(line_index)
            .ok_or(PositionConversionError::LineOutOfRange)?;
        let scalar_index = usize::try_from(position.column - 1)
            .map_err(|_| PositionConversionError::CoordinateOverflow)?;
        if scalar_index > line.chars().count() {
            return Err(PositionConversionError::ColumnOutOfRange);
        }
        let units = line
            .chars()
            .take(scalar_index)
            .try_fold(0_usize, |total, character| {
                total.checked_add(units_for(character, self.encoding))
            })
            .ok_or(PositionConversionError::CoordinateOverflow)?;
        Ok(Position::new(
            u32::try_from(line_index).map_err(|_| PositionConversionError::CoordinateOverflow)?,
            u32::try_from(units).map_err(|_| PositionConversionError::CoordinateOverflow)?,
        ))
    }

    pub(crate) fn lsp_to_agent(
        &self,
        position: Position,
    ) -> Result<AgentPosition, PositionConversionError> {
        let line_index = usize::try_from(position.line)
            .map_err(|_| PositionConversionError::CoordinateOverflow)?;
        let line = self
            .lines
            .get(line_index)
            .ok_or(PositionConversionError::LineOutOfRange)?;
        let target = usize::try_from(position.character)
            .map_err(|_| PositionConversionError::CoordinateOverflow)?;
        let mut units = 0_usize;
        let mut scalar_index = 0_usize;
        for character in line.chars() {
            if target == units {
                return agent_position(line_index, scalar_index);
            }
            let next = units
                .checked_add(units_for(character, self.encoding))
                .ok_or(PositionConversionError::CoordinateOverflow)?;
            if target < next {
                return Err(PositionConversionError::InvalidCharacterBoundary);
            }
            units = next;
            scalar_index += 1;
        }
        if target == units {
            agent_position(line_index, scalar_index)
        } else {
            Err(PositionConversionError::ColumnOutOfRange)
        }
    }

    pub(crate) fn range_to_normalized(
        &self,
        range: Range,
    ) -> Result<NormalizedRange, PositionConversionError> {
        if (range.end.line, range.end.character) < (range.start.line, range.start.character) {
            return Err(PositionConversionError::ReversedRange);
        }
        let start = self.lsp_to_agent(range.start)?;
        let end = self.lsp_to_agent(range.end)?;
        NormalizedRange::new(start.line, start.column, end.line, end.column)
            .map_err(|_| PositionConversionError::ReversedRange)
    }
}

const fn units_for(character: char, encoding: PositionEncoding) -> usize {
    match encoding {
        PositionEncoding::Utf8 => character.len_utf8(),
        PositionEncoding::Utf16 => character.len_utf16(),
    }
}

fn agent_position(
    line_index: usize,
    scalar_index: usize,
) -> Result<AgentPosition, PositionConversionError> {
    Ok(AgentPosition::new(
        u32::try_from(line_index + 1).map_err(|_| PositionConversionError::CoordinateOverflow)?,
        u32::try_from(scalar_index + 1).map_err(|_| PositionConversionError::CoordinateOverflow)?,
    ))
}
