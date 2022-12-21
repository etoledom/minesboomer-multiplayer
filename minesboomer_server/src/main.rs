mod server;
use server::*;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::net::TcpListener;

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

    let server = Arc::new(Server::new(state.clone(), Arc::clone(&multi_games), Arc::clone(&players)));

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        let server = Arc::clone(&server);
        tokio::spawn(server.handle_connection(stream, addr));
    }
}
