use crate::gui::gameplay::MinesBoomer;
use futures::{channel::mpsc::UnboundedReceiver, pin_mut};
use futures_util::{future, StreamExt};
use minesboomer_utils::*;
use minesweeper_multiplayer::Point;
use std::sync::{Arc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub struct WSClient {
    socket_sender: tokio::sync::mpsc::UnboundedSender<Message>,
    game: Arc<Mutex<MinesBoomer>>,
}

impl WSClient {
    pub fn new(socket_sender: tokio::sync::mpsc::UnboundedSender<Message>, game: Arc<Mutex<MinesBoomer>>) -> Self {
        WSClient { socket_sender, game }
    }

    #[tokio::main]
    pub async fn start_listening(self: Arc<Self>, socket_receiver: tokio::sync::mpsc::UnboundedReceiver<Message>, game_receiver: UnboundedReceiver<Message>) {
        let connect_addr = "ws://127.0.0.1:8000";

        let url = url::Url::parse(connect_addr).unwrap();

        tokio::spawn({
            let c_self = Arc::clone(&self);
            async move {
                c_self.receive_message(socket_receiver).await;
            }
        });

        println!("connecting...");
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
        println!("WebSocket handshake has been successfully completed");

        let (write, read) = ws_stream.split();

        let stdin_to_ws = game_receiver.map(Ok).forward(write);

        let ws_to_stdout = {
            read.for_each(|message| async {
                let string = message.unwrap().into_text().unwrap();
                println!("Received {}", string);

                match self.socket_sender.send(Message::Text(string)) {
                    Result::Ok(some) => some,
                    Err(err) => println!("Error {}", err),
                }
            })
        };

        pin_mut!(stdin_to_ws, ws_to_stdout);

        future::select(stdin_to_ws, ws_to_stdout).await;
    }

    async fn receive_message(&self, rx: tokio::sync::mpsc::UnboundedReceiver<Message>) {
        let mut mut_rx = rx;

        while let Some(message) = mut_rx.recv().await {
            let string = message.to_string();

            if let Ok(s_point) = serde_json::from_str::<SerializablePoint>(&string) {
                println!("-> Selected from remote");
                let point: Point = s_point.into();
                let mut game = self.game.lock().unwrap();
                game.game.game.selected_at(point);
            } else if let Ok(msg) = serde_json::from_str::<GameStartMessage>(&string) {
                println!("-> New message");
                let board = msg.get_board();
                let mut game = self.game.lock().unwrap();
                game.game.game.board = board;
            } else if let Ok(simple_msg) = serde_json::from_str::<SimpleMessage>(&string) {
                if simple_msg.name == "identify" {
                    println!("-> Identify request received");
                    self.game.lock().unwrap().request_user_id();
                }
            }
        }
    }
}
