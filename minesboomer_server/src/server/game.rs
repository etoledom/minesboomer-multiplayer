use minesweeper_multiplayer::{Board, Difficulty, Multiplayer, Point};
use std::net::SocketAddr;
use uuid::Uuid;

pub struct Player {
    id: String,
    name: String,
    game_id: String,
    address: SocketAddr,
}

impl Player {
    pub fn new(name: String, game_id: impl Into<String>, address: SocketAddr) -> Self {
        Player {
            id: Uuid::new_v4().to_string(),
            name,
            game_id: game_id.into(),
            address,
        }
    }

    pub fn game_id(&self) -> String {
        self.game_id.clone()
    }

    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    pub fn get_address(&self) -> SocketAddr {
        self.address
    }
}

pub struct Game {
    id: String,
    host: Player,
    client: Option<Player>,
    multi_game: Multiplayer,
}

impl Game {
    pub fn new(player: Player, id: impl Into<String>) -> Self {
        Game {
            host: player,
            client: None,
            multi_game: Multiplayer::new(["", ""], Difficulty::Easy),
            id: id.into(),
        }
    }

    pub fn generate_multi_game(&mut self) {
        let mut multi_game = Multiplayer::new([&self.host.name, &self.client.as_ref().unwrap().name], Difficulty::Easy);
        multi_game.players[0].id = self.host.get_id();
        multi_game.players[1].id = self.client.as_ref().unwrap().get_id();
        self.multi_game = multi_game;
    }

    pub fn get_board(&self) -> &Board {
        self.multi_game.get_board()
    }

    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    pub fn set_client(&mut self, client: Player) {
        self.client = Some(client);
    }

    pub fn has_client(&self) -> bool {
        self.client.is_some()
    }

    pub fn player_selected(&mut self, coordinates: Point) {
        self.multi_game.player_selected(coordinates);
    }

    pub fn is_player_active(&self, player_id: impl Into<String>) -> bool {
        self.multi_game.current_player().id == player_id.into()
    }

    pub fn get_players(&self) -> Vec<&Player> {
        let mut players = vec![&self.host];
        if let Some(client) = &self.client {
            players.push(client);
        }
        players
    }
}
