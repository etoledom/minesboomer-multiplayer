mod gui;

use minesweeper_multiplayer::{Difficulty, Multiplayer, Point};
use std::sync::{Arc, Mutex};
use std::thread;

// use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::channel::mpsc;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt};
use gui::gameplay::MinesBoomer;
use serde_json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

fn main() {
    let (selected_send_tx, selected_send_rx) = mpsc::unbounded::<Message>();
    let (selected_received_tx, selected_received_rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let game = Arc::new(Mutex::new(Multiplayer::new(["Player 1", "Player 2"], Difficulty::Easy)));

    let game_clone = Arc::clone(&game);
    thread::spawn(move || {
        println!("Will try to connect");
        run_ws_clien(selected_send_rx, selected_received_tx, selected_received_rx, game_clone);
    });

    println!("Will create UI...");

    let mines = MinesBoomer::new(selected_send_tx, game);

    // let mut shared = Arc::new(mines);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native("MinesBooMer", native_options, Box::new(|cc| Box::new(mines)));
}

async fn receive_message(rx: tokio::sync::mpsc::UnboundedReceiver<Message>, game: Arc<Mutex<Multiplayer>>) {
    let mut mut_rx = rx;

    while let Some(message) = mut_rx.recv().await {
        let string = message.to_string();
        let s_point: SerializablePoint = serde_json::from_str::<SerializablePoint>(&string).unwrap();
        let point: Point = s_point.into();
        println!("SELECTED FROM REMOTE: {:?}", point);
        let mut game = game.lock().unwrap();
        game.game.selected_at(point);
    }
}

#[tokio::main]
async fn run_ws_clien(
    rx_selected: UnboundedReceiver<Message>,
    tx_selected: tokio::sync::mpsc::UnboundedSender<Message>,
    rx: tokio::sync::mpsc::UnboundedReceiver<Message>,
    game: Arc<Mutex<Multiplayer>>,
) {
    let connect_addr = "ws://127.0.0.1:8000";

    let url = url::Url::parse(connect_addr).unwrap();

    // let (stdin_tx, stdin_rx) = mpsc::unbounded();
    // tokio::spawn(read_stdin(stdin_tx));
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

#[derive(serde::Serialize, serde::Deserialize)]
struct SerializablePoint {
    x: usize,
    y: usize,
}

impl From<Point> for SerializablePoint {
    fn from(point: Point) -> SerializablePoint {
        SerializablePoint { x: point.x, y: point.y }
    }
}

impl Into<Point> for SerializablePoint {
    fn into(self) -> Point {
        Point { x: self.x, y: self.y }
    }
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
async fn read_stdin(tx: futures::channel::mpsc::UnboundedSender<Message>) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx.unbounded_send(Message::binary(buf)).unwrap();
    }
}
