mod game;
use game::*;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use minesboomer_utils::*;

use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

pub type Tx = UnboundedSender<Message>;
pub type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
pub type MultiGames = Arc<Mutex<Vec<Game>>>;
pub type Players = Arc<Mutex<HashMap<SocketAddr, String>>>;

pub struct Server {
    peer_map: PeerMap,
    games: MultiGames,
    players: Players,
}

impl Server {
    pub fn new(peer_map: PeerMap, games: MultiGames, players: Players) -> Self {
        Server { peer_map, games, players }
    }

    pub async fn handle_connection(self: Arc<Self>, raw_stream: TcpStream, addr: SocketAddr) {
        println!("Incoming TCP connection from: {}", addr);

        let ws_stream = tokio_tungstenite::accept_async(raw_stream).await.expect("Error during the websocket handshake occurred");
        println!("WebSocket connection established: {}", addr);

        let (tx, rx) = unbounded();
        self.peer_map.lock().unwrap().insert(addr, tx);

        self.request_identification(addr);

        let (outgoing, incoming) = ws_stream.split();

        let handle_received = incoming.try_for_each(|msg| {
            println!("Received a message from {}: {}", addr, msg.to_text().unwrap());
            self.handle_received_message(msg, addr);
            future::ok(())
        });

        let receive_from_others = rx.map(Ok).forward(outgoing);

        pin_mut!(handle_received, receive_from_others);
        future::select(handle_received, receive_from_others).await;

        println!("{} disconnected", &addr);
        self.peer_map.lock().unwrap().remove(&addr);
    }

    fn request_identification(&self, addr: SocketAddr) {
        let message = Message::Text(SimpleMessage::new("identify").to_json_string());
        let peer_guard = self.peer_map.lock().unwrap();
        let tx = peer_guard.get(&addr);
        println!("-> Sending identify");
        match tx.unwrap().unbounded_send(message) {
            Ok(_) => println!("Ok."),
            Err(err) => println!("{}", err),
        }
    }

    fn handle_identification_message(&self, message: IdentificationMessage, addr: SocketAddr) {
        let mut games_guard = self.games.lock().unwrap();
        if let Some(game) = games_guard.iter_mut().find(|game| !game.has_client()) {
            println!("-> Game found. Adding player to game");
            let player = Player::new(message.name, game.get_id(), addr);
            self.players.lock().unwrap().insert(addr, player.game_id());
            game.set_client(player);
            game.generate_multi_game();
            self.send_new_game_to_players(game);
        } else {
            let game_id = Uuid::new_v4().to_string();
            let player = Player::new(message.name, &game_id, addr);
            self.players.lock().unwrap().insert(addr, player.game_id());
            let game = Game::new(player, game_id);
            games_guard.push(game);
        }
    }

    fn handle_received_message(&self, msg: Message, addr: SocketAddr) {
        let message_string = msg.to_text().unwrap();
        if let Ok(message) = IdentificationMessage::new_from_json(message_string) {
            println!("Identification received for {}", message.name);
            self.handle_identification_message(message, addr);
        } else if let Ok(message) = CellSelectedMessage::new_from_json(message_string) {
            let game_id = self.players.lock().unwrap().get(&addr).unwrap().clone();
            let mut games = self.games.lock().unwrap();
            let game = games.iter_mut().find(|game| game.get_id() == game_id).unwrap();

            game.player_selected(message.coordinates.into());
            self.send_selected_to_players(game, message.coordinates);
        }
    }

    fn send_selected_to_players(&self, game: &Game, coordinates: SerializablePoint) {
        for player in game.get_players() {
            let is_active = game.is_player_active(player.get_id());
            self.send_message_to(player, Message::Text(CellSelectedMessage::new(coordinates, is_active).to_json_string()));
        }
    }

    fn send_new_game_to_players(&self, game: &Game) {
        for player in game.get_players() {
            let is_active = game.is_player_active(player.get_id());
            let board: SerializableBoard = game.get_board().clone().into();
            self.send_message_to(player, Message::Text(GameStartMessage::new(board, is_active).to_json_string()));
        }
    }

    fn send_message_to(&self, player: &Player, message: Message) {
        let peers = self.peer_map.lock().unwrap();
        let sender = peers.get(&player.get_address());
        sender.unwrap().unbounded_send(message).unwrap();
    }

    fn _send_to_all(&self, msg: Message) {
        let peers = self.peer_map.lock().unwrap();

        let broadcast_recipients = peers.iter().map(|(_, ws_sink)| ws_sink);

        println!("Sending message to all");
        for recp in broadcast_recipients {
            recp.unbounded_send(msg.clone()).unwrap();
        }
    }
}
