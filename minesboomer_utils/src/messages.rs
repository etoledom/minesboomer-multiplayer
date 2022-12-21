use minesweeper_core::Board;
use serde::{Deserialize, Serialize};

use crate::serializables::*;

macro_rules! new_from_and_to_json {
    () => {
        pub fn new_from_json(str: &str) -> Result<Self, serde_json::Error> {
            serde_json::from_str(str)
        }

        pub fn to_json_string(&self) -> String {
            serde_json::to_string(self).unwrap()
        }
    };
}

#[derive(Serialize, Deserialize)]
pub struct GameStartMessage {
    pub name: String,
    board: SerializableBoard,
    pub is_active: bool,
}

impl GameStartMessage {
    pub fn new(board: SerializableBoard, is_active: bool) -> Self {
        GameStartMessage {
            name: "start".to_owned(),
            board,
            is_active,
        }
    }

    pub fn get_board(&self) -> Board {
        self.board.clone().into()
    }

    new_from_and_to_json!();
}

#[derive(Serialize, Deserialize)]
pub struct SimpleMessage {
    pub name: String,
}

impl SimpleMessage {
    pub fn new(name: impl Into<String>) -> Self {
        SimpleMessage { name: name.into() }
    }

    new_from_and_to_json!();
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

    new_from_and_to_json!();
}

#[derive(Serialize, Deserialize)]
pub struct CellSelectedMessage {
    pub name: String,
    pub is_active_player: bool,
    pub coordinates: SerializablePoint,
}

impl CellSelectedMessage {
    pub fn new(coordinates: SerializablePoint, is_active_player: bool) -> Self {
        CellSelectedMessage {
            name: "cell_selected".to_owned(),
            is_active_player,
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
