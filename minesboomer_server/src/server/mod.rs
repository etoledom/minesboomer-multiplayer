mod game;
use game::*;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use minesweeper_multiplayer::messages::*;
use minesweeper_multiplayer::serializables::*;
use uuid::Uuid;

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
        self.remove_player(&addr);
    }

    fn remove_player(&self, addr: &SocketAddr) {
        let mut games = self.games.lock().unwrap();
        if let Some(index) = games.iter().position(|game| game.get_host().get_address() == *addr) {
            let removed_game = games.remove(index);
            if let Some(client) = removed_game.get_client() {
                self.send_message_to(client, SimpleMessage::new("host_disconnected").to_json_string());
            }
        } else {
            let mut games_with_clients = games.iter_mut().filter(|game| game.has_client());
            if let Some(game) = games_with_clients.find(|game| game.get_client().unwrap().get_address() == *addr) {
                game.remove_client();
                self.send_message_to(game.get_host(), SimpleMessage::new("client_disconnected").to_json_string());
            }
        }
        self.players.lock().unwrap().remove(addr);
        self.peer_map.lock().unwrap().remove(addr);
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
        let player = Player::new(message.name, "", addr);
        self.players.lock().unwrap().insert(addr, player.game_id());
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
        } else if let Ok(message) = CreateGameMessage::new_from_json(message_string) {
            let mut games_guard = self.games.lock().unwrap();
            let game_id = Uuid::new_v4().to_string();
            let player = Player::new(message.game.name, &game_id, addr);
            self.players.lock().unwrap().insert(addr, player.game_id());
            let game = Game::new(player, game_id);
            games_guard.push(game);
            self.send_message_to_addr(&addr, SimpleMessage::new("waiting_enemy").to_json_string());
        } else if let Ok(message) = JoinGameMessage::new_from_json(message_string) {
            println!("-> Client joined game");
            let mut games = self.games.lock().unwrap();
            let game = games.iter_mut().find(|game| game.get_id() == message.game_id).unwrap();
            let client = Player::new(message.client_name, game.get_id(), addr);
            self.players.lock().unwrap().insert(addr, client.game_id());
            game.set_client(client);
            game.generate_multi_game();
            self.send_new_game_to_players(game);
        } else if let Ok(message) = SimpleMessage::new_from_json(message_string) {
            if message.name == "games_request" {
                self.send_open_games(addr);
            }
        }
    }

    fn send_open_games(&self, addr: SocketAddr) {
        let games = self.games.lock().unwrap();
        let game_defs = games
            .iter()
            .filter(|game| !game.has_client())
            .map(|game| {
                let name = game.get_host().get_name();
                let id = game.get_id();
                let difficulty = game.get_difficulty();
                GameDefinition::new(id, name, difficulty.clone())
            })
            .collect();
        let message = OpenGamesMessage::new(game_defs);
        let peers = self.peer_map.lock().unwrap();
        let sender = peers.get(&addr).unwrap();
        println!("-> Sending OpenGamesMessage: {}", message.to_json_string());
        sender.unbounded_send(Message::Text(message.to_json_string())).unwrap();
    }

    fn send_selected_to_players(&self, game: &Game, coordinates: SerializablePoint) {
        for player in game.get_players() {
            let is_active = game.is_player_active(player.get_id());
            self.send_message_to(player, CellSelectedMessage::new(coordinates, is_active).to_json_string());
        }
    }

    fn send_new_game_to_players(&self, game: &Game) {
        for player in game.get_players() {
            let is_active = game.is_player_active(player.get_id());
            let board: SerializableBoard = game.get_board().clone().into();
            self.send_message_to(player, GameStartMessage::new(board, is_active).to_json_string());
        }
    }

    fn send_message_to(&self, player: &Player, message_json: String) {
        self.send_message_to_addr(&player.get_address(), message_json);
    }

    fn send_message_to_addr(&self, addr: &SocketAddr, message_json: String) {
        let peers = self.peer_map.lock().unwrap();
        let sender = peers.get(addr);
        sender.unwrap().unbounded_send(Message::Text(message_json)).unwrap();
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
