mod gui;
mod networking;

use eframe::{App, Frame};
use minesboomer_utils::*;
use minesweeper_multiplayer::{Difficulty, Multiplayer, Point};
use networking::*;
use std::sync::{Arc, Mutex};
use std::thread;

// use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::channel::mpsc;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt};
use gui::gameplay::MinesBoomer;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

struct AppThreadsafeWrapper {
    boomer: Arc<Mutex<MinesBoomer>>,
}

impl App for AppThreadsafeWrapper {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        self.boomer.lock().unwrap().update(ctx, frame);
    }
}

fn main() {
    // Internal game->ws-client communication.
    let (game_sender, game_receiver) = mpsc::unbounded::<Message>();
    // Web-Sockets client<->server communication
    let (socket_sender, socket_receiver) = tokio::sync::mpsc::unbounded_channel::<Message>();

    let game = Multiplayer::new(["Player 1", "Player 2"], Difficulty::Easy);
    let boomer = MinesBoomer::new(game_sender, game);
    let boomer_multithread = Arc::new(Mutex::new(boomer));
    let boomer_multithread_clone = Arc::clone(&boomer_multithread);

    thread::spawn(move || {
        let client = Arc::new(WSClient::new(socket_sender, boomer_multithread_clone));
        client.start_listening(socket_receiver, game_receiver);
    });

    let app = AppThreadsafeWrapper { boomer: boomer_multithread };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("MinesBooMer", native_options, Box::new(|_| Box::new(app)));
}
