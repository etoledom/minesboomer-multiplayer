mod serializable_board;
mod serializable_point;
use minesweeper_core::{Cell, Difficulty};

pub use serializable_board::*;
pub use serializable_point::*;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SerializableCell {
    pub number: i8,
    pub cleared: bool,
    pub flagged: bool,
    pub coordinates: SerializablePoint,
}

impl From<Cell> for SerializableCell {
    fn from(cell: Cell) -> Self {
        SerializableCell {
            number: cell.number,
            cleared: cell.cleared,
            flagged: cell.flagged,
            coordinates: cell.coordinates.into(),
        }
    }
}

impl From<SerializableCell> for Cell {
    fn from(cell: SerializableCell) -> Self {
        Cell {
            number: cell.number,
            cleared: cell.cleared,
            flagged: cell.flagged,
            coordinates: cell.coordinates.into(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct GameDefinition {
    pub name: String,
    pub id: String,
    pub difficulty: String,
}

impl GameDefinition {
    pub fn new(id: impl Into<String>, name: impl Into<String>, difficulty: Difficulty) -> Self {
        GameDefinition {
            name: name.into(),
            id: id.into(),
            difficulty: difficulty.to_string(),
        }
    }
}
