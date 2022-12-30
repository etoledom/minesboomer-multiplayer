use crate::gui::gameplay::{MinesBoomer, OpenGame};
use futures::channel::mpsc::UnboundedReceiver;
use futures::pin_mut;
use futures_util::{future, StreamExt};
use minesweeper_multiplayer::messages::*;
use std::sync::{Arc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub struct WSClient {
    game: Arc<Mutex<MinesBoomer>>,
}

impl WSClient {
    pub fn new(game: Arc<Mutex<MinesBoomer>>) -> Self {
        WSClient { game }
    }

    #[tokio::main]
    pub async fn start_listening(&self, game_receiver: UnboundedReceiver<Message>) {
        let connect_addr = "ws://0.0.0.0:8080";

        let url = url::Url::parse(connect_addr).unwrap();

        println!("connecting...");
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
        println!("WebSocket handshake has been successfully completed");

        let (sender, receiver) = ws_stream.split();

        // Get message from game and forward it to remote.
        let game_to_remote = game_receiver.map(Ok).forward(sender);

        // Receive message from remote and handle it.
        let remote_to_game = {
            receiver.for_each(|message| async {
                self.receive_message(message.unwrap()).await;
            })
        };

        pin_mut!(game_to_remote, remote_to_game);
        future::select(game_to_remote, remote_to_game).await;
    }

    async fn receive_message(&self, message: Message) {
        let string = message.to_string();
        if let Ok(msg) = serde_json::from_str::<GameStartMessage>(&string) {
            println!("-> GameStartMessage. active: {}", msg.is_active);
            let board = msg.get_board();
            let mut game = self.game.lock().unwrap();
            game.set_board(board);
            game.set_is_active(msg.is_active);
            game.waiting_for_enemy = false;
            game.close_open_games_menu();
            println!("Ok.");
        } else if let Ok(msg) = serde_json::from_str::<CellSelectedMessage>(&string) {
            println!("-> CellSelectedMessage: {}", msg.to_json_string());
            let mut game = self.game.lock().unwrap();
            game.remote_player_selected(msg.coordinates.into());
            game.set_is_active(msg.is_active_player);
            println!("Ok.");
        } else if let Ok(msg) = OpenGamesMessage::new_from_json(&string) {
            println!("-> OpenGamesMessage: {}", msg.to_json_string());
            let mut game = self.game.lock().unwrap();
            let games = msg
                .games
                .iter()
                .map(|game| OpenGame {
                    name: game.name.clone(),
                    difficulty: game.difficulty.clone(),
                    game_id: game.id.clone(),
                })
                .collect();
            game.present_open_games_menu(games);
            println!("Ok.");
        } else if let Ok(simple_msg) = serde_json::from_str::<SimpleMessage>(&string) {
            println!("-> SimpleMessage: {}", simple_msg.name);
            if simple_msg.name == "identify" {
                let game = self.game.lock().unwrap();
                game.request_open_games();
                game.request_user_id();
            } else if simple_msg.name == "waiting_enemy" {
                let mut game = self.game.lock().unwrap();
                game.waiting_for_enemy = true;
                game.close_open_games_menu();
            } else if simple_msg.name == "client_disconnected" {
                let mut game = self.game.lock().unwrap();
                game.waiting_for_enemy = true;
            } else if simple_msg.name == "host_disconnected" {
                let mut game = self.game.lock().unwrap();
                game.present_open_games_menu(vec![]);
                game.request_open_games();
            }
            println!("Ok.");
        }
    }
}
