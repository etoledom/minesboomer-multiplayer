mod gui;
mod networking;

use eframe::{App, Frame};
use minesweeper_multiplayer::{Difficulty, Multiplayer};
use networking::*;
use std::sync::{Arc, Mutex};
use std::thread;

use futures::channel::mpsc::unbounded;
use gui::gameplay::MinesBoomer;
use tokio_tungstenite::tungstenite::protocol::Message;

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
    let (game_sender, game_receiver) = unbounded::<Message>();

    let game = Multiplayer::new(["Player 1", "Player 2"], Difficulty::Easy);
    let boomer = MinesBoomer::new(game_sender, game);
    let boomer_multithread = Arc::new(Mutex::new(boomer));
    let boomer_multithread_clone = Arc::clone(&boomer_multithread);

    thread::spawn(move || {
        let client = WSClient::new(boomer_multithread_clone);
        client.start_listening(game_receiver);
    });

    let app = AppThreadsafeWrapper { boomer: boomer_multithread };
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("MinesBooMer", native_options, Box::new(|_| Box::new(app)));
}
