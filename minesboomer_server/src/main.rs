use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use minesboomer_utils::*;
use minesweeper_multiplayer::*;

use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;
type MultiGames = Arc<Mutex<Vec<Game>>>;
type Players = Arc<Mutex<HashMap<SocketAddr, String>>>;

async fn handle_connection(peer_map: PeerMap, games: MultiGames, raw_stream: TcpStream, players: Players, addr: SocketAddr) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream).await.expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr, tx);

    request_identification(&peer_map, addr);

    let (outgoing, incoming) = ws_stream.split();

    let handle_received = incoming.try_for_each(|msg| {
        println!("Received a message from {}: {}", addr, msg.to_text().unwrap());
        handle_received_message(msg, Arc::clone(&games), addr, Arc::clone(&players), &peer_map);
        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(handle_received, receive_from_others);
    future::select(handle_received, receive_from_others).await;

    println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
}

fn request_identification(peer_map: &PeerMap, addr: SocketAddr) {
    let message = Message::Text(SimpleMessage::new("identify").to_json_string());
    let peer_guard = peer_map.lock().unwrap();
    let tx = peer_guard.get(&addr);
    tx.unwrap().unbounded_send(message).unwrap();
}

fn handle_received_message(msg: Message, games: MultiGames, addr: SocketAddr, players: Players, peer_map: &PeerMap) {
    let message_string = msg.to_text().unwrap();
    if let Ok(message) = IdentificationMessage::new_from_json(message_string) {
        println!("Identification received for {}", message.name);
        handle_identification_message(&games, message, addr, &players, peer_map);
    } else if let Ok(message) = CellSelectedMessage::new_from_json(message_string) {
        let game_id = players.lock().unwrap().get(&addr).unwrap().clone();
        let mut games = games.lock().unwrap();
        let game = games.iter_mut().find(|game| game.id == game_id).unwrap();

        game.multi_game.as_mut().unwrap().player_selected(message.coordinates.into());

        send_to_all(Message::text(message.coordinates.to_json_string()), peer_map);
    }
}

fn handle_identification_message(games: &MultiGames, message: IdentificationMessage, addr: SocketAddr, players: &Players, peer_map: &PeerMap) {
    let mut games_guard = games.lock().unwrap();
    if let Some(game) = games_guard.iter_mut().find(|game| game.client.is_none()) {
        println!("-> Game found. Adding player to game");
        let player = Player {
            name: message.name,
            address: addr,
            game_id: game.id.clone(),
        };
        players.lock().unwrap().insert(addr, player.game_id.clone());
        game.client = Some(player);
        game.generate_multi_game();
        let board: SerializableBoard = game.multi_game.as_ref().unwrap().game.board.clone().into();
        send_to_all(Message::Text(GameStartMessage::new(board).to_json_string()), peer_map)
    } else {
        println!("-> No game found. Creating new game");
        let game_id = Uuid::new_v4().to_string();
        let player = Player {
            name: message.name,
            address: addr,
            game_id: game_id.clone(),
        };
        players.lock().unwrap().insert(addr, player.game_id.clone());
        let game = Game::new(player, game_id);
        games_guard.push(game);
        println!("Game added");
    }
}

fn send_to_all(msg: Message, peer_map: &PeerMap) {
    let peers = peer_map.lock().unwrap();

    // We want to broadcast the message to everyone except ourselves.
    let broadcast_recipients = peers.iter().map(|(_, ws_sink)| ws_sink);

    println!("Sending message to all");
    for recp in broadcast_recipients {
        recp.unbounded_send(msg.clone()).unwrap();
    }
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8000".to_string();

    let multi_games: MultiGames = Arc::new(Mutex::new(vec![]));

    let state = PeerMap::new(Mutex::new(HashMap::new()));
    let players = Players::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), Arc::clone(&multi_games), stream, Arc::clone(&players), addr));
    }
}

struct Player {
    name: String,
    // tx: &Tx,
    address: SocketAddr,
    game_id: String,
}

struct Game {
    id: String,
    host: Player,
    client: Option<Player>,
    multi_game: Option<Multiplayer>,
}

impl Game {
    fn new(player: Player, id: impl Into<String>) -> Self {
        Game {
            host: player,
            client: None,
            multi_game: None,
            id: id.into(),
        }
    }

    fn generate_multi_game(&mut self) {
        let multi_game = Multiplayer::new([&self.host.name, &self.client.as_ref().unwrap().name], Difficulty::Easy);
        self.multi_game = Some(multi_game);
    }
}
