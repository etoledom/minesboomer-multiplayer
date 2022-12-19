use minesweeper_core::Board;
use serde::{Deserialize, Serialize};

use crate::serializables::*;

#[derive(Serialize, Deserialize)]
pub struct GameStartMessage {
    pub name: String,
    board: SerializableBoard,
}

impl GameStartMessage {
    pub fn new(board: SerializableBoard) -> Self {
        GameStartMessage { name: "start".to_owned(), board }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn get_board(&self) -> Board {
        self.board.clone().into()
    }
}

#[derive(Serialize, Deserialize)]
pub struct SimpleMessage {
    pub name: String,
}

impl SimpleMessage {
    pub fn new(name: impl Into<String>) -> Self {
        SimpleMessage { name: name.into() }
    }

    pub fn new_from_json(str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(str)
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct IdentificationMessage {
    pub name: String,
    pub user_id: String,
}

impl IdentificationMessage {
    pub fn new(user_id: String) -> Self {
        IdentificationMessage {
            name: "user_identification".to_owned(),
            user_id,
        }
    }

    pub fn new_from_json(str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(str)
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct CellSelectedMessage {
    pub name: String,
    pub coordinates: SerializablePoint,
}

impl CellSelectedMessage {
    pub fn new(coordinates: SerializablePoint) -> Self {
        CellSelectedMessage {
            name: "cell_selected".to_owned(),
            coordinates,
        }
    }

    pub fn new_from_json(str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(str)
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
