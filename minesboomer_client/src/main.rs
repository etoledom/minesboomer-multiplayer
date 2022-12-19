mod gui;
use eframe::{App, Frame};
use minesboomer_utils::*;
use minesweeper_multiplayer::{Difficulty, Multiplayer, Point};
use std::sync::{Arc, Mutex};
use std::thread;

// use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::channel::mpsc;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt};
use gui::gameplay::MinesBoomer;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

struct AppWrapper {
    boomer: Arc<Mutex<MinesBoomer>>,
}

impl App for AppWrapper {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        self.boomer.lock().unwrap().update(ctx, frame);
    }
}

fn main() {
    let (selected_send_tx, selected_send_rx) = mpsc::unbounded::<Message>();
    let (selected_received_tx, selected_received_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    let game = Multiplayer::new(["Player 1", "Player 2"], Difficulty::Easy);
    let boomer = MinesBoomer::new(selected_send_tx, game);
    let boomer_multithread = Arc::new(Mutex::new(boomer));
    let boomer_multithread_clone = Arc::clone(&boomer_multithread);
    let app = AppWrapper { boomer: boomer_multithread };

    // let game_clone = Arc::clone(&game);
    thread::spawn(move || {
        run_ws_clien(selected_send_rx, selected_received_tx, selected_received_rx, boomer_multithread_clone);
    });

    // let mines = MinesBoomer::new(selected_send_tx, game);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native("MinesBooMer", native_options, Box::new(|cc| Box::new(app)));
}

async fn receive_message(rx: tokio::sync::mpsc::UnboundedReceiver<Message>, game: Arc<Mutex<MinesBoomer>>) {
    let mut mut_rx = rx;

    while let Some(message) = mut_rx.recv().await {
        let string = message.to_string();

        if let Ok(s_point) = serde_json::from_str::<SerializablePoint>(&string) {
            let point: Point = s_point.into();
            println!("-> Selected from remote");
            let mut game = game.lock().unwrap();
            game.game.game.selected_at(point);
        } else if let Ok(msg) = serde_json::from_str::<GameStartMessage>(&string) {
            println!("-> New message");
            let board = msg.get_board();
            let mut game = game.lock().unwrap();
            game.game.game.board = board;
        } else if let Ok(simple_msg) = serde_json::from_str::<SimpleMessage>(&string) {
            if simple_msg.name == "identify" {
                game.lock().unwrap().request_user_id();
            }
        }
    }
}

#[tokio::main]
async fn run_ws_clien(
    rx_selected: UnboundedReceiver<Message>,
    tx_selected: tokio::sync::mpsc::UnboundedSender<Message>,
    rx: tokio::sync::mpsc::UnboundedReceiver<Message>,
    game: Arc<Mutex<MinesBoomer>>,
) {
    let connect_addr = "ws://127.0.0.1:8000";

    let url = url::Url::parse(connect_addr).unwrap();

    tokio::spawn(receive_message(rx, game));

    println!("connecting...");
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (write, read) = ws_stream.split();

    let stdin_to_ws = rx_selected.map(Ok).forward(write);

    let ws_to_stdout = {
        read.for_each(|message| async {
            let string = message.unwrap().into_text().unwrap();
            println!("Received {}", string);

            match tx_selected.send(Message::Text(string)) {
                Result::Ok(some) => some,
                Err(err) => println!("Error {}", err.to_string()),
            }
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);

    future::select(stdin_to_ws, ws_to_stdout).await;
}
