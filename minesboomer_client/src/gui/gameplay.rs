use super::mine_image::MineImage;
use minesweeper_multiplayer::serializables::*;
use minesweeper_multiplayer::*;

use eframe::egui;
use egui::{Button, Color32, Label, RichText, TextStyle, Ui, WidgetText};
use futures::channel::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::protocol::Message;

#[derive(Clone)]
pub struct OpenGame {
    pub name: String,
    pub difficulty: String,
    pub game_id: String,
}

pub struct MinesBoomer {
    pub game: Multiplayer,
    mine: MineImage,
    sender: UnboundedSender<Message>,
    is_active: bool,
    show_games_list: Option<Vec<OpenGame>>,
    pub waiting_for_enemy: bool,
    show_game_name_popup: bool,
    game_creation_view: GameCreationView,
    game_name: String,
}

impl MinesBoomer {
    pub fn new(sender: UnboundedSender<Message>, game: Multiplayer) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.

        MinesBoomer {
            game,
            mine: MineImage::default(),
            sender,
            is_active: false,
            show_games_list: None,
            waiting_for_enemy: false,
            show_game_name_popup: false,
            game_creation_view: GameCreationView::default(),
            game_name: "".to_owned(),
        }
    }

    fn draw_cell(&mut self, cell: &Cell, ui: &mut Ui) {
        let color = get_color_for_cell(cell);
        let text = get_text_for_cell(cell);

        if cell.is_mine() && cell.cleared {
            self.mine.ui(ui);
        } else if ui.add_sized([50., 50.], Button::new(text).fill(color)).clicked() {
            self.on_cell_tapped(cell);
        }
    }

    fn get_copied_cell_at(&self, coordinates: Point) -> Option<Cell> {
        self.game.get_board().cell_at(coordinates).copied()
    }

    fn draw_board(&mut self, ui: &mut Ui) {
        let dimentions = self.game.get_board_dimentions();
        ui.horizontal(|ui| {
            for x in 0..dimentions.width {
                ui.vertical(|ui| {
                    for y in 0..dimentions.height {
                        let Some(cell) = self.get_copied_cell_at(Point { x, y }) else {
                            continue;
                        };
                        self.draw_cell(&cell, ui);
                    }
                });
            }
        });
    }

    fn draw_gui(&mut self, ui: &mut Ui) {
        if let Some(winner) = self.game.winner() {
            ui.vertical_centered_justified(|ui| {
                ui.heading("WINNER!");
                ui.heading(winner.name.to_string());
            });
            return;
        }

        let remining_mines = self.game.game.remaining_mines();
        let mines_to_win = self.game.remaining_to_win();
        let winning = self.game.player_winning();
        let is_active = self.is_active;

        ui.vertical_centered_justified(|ui| {
            if is_active {
                ui.heading("Is YOUR tourn!");
            } else {
                ui.heading("Your enemy is playing");
            }
            // ui.heading(current_player);
            ui.label(format!("Mines left: {}", remining_mines));
            if mines_to_win <= 5 {
                let Some(winning) = winning else {
                    return
                };
                ui.separator();
                ui.label(format!("{} is winning!", winning.name));
                ui.label(format!("{} mines to go", mines_to_win));
            }
        });
    }

    fn draw_game_list(&mut self, ui: &mut Ui, game_list: &[OpenGame]) {
        ui.vertical_centered(|ui| {
            let title = WidgetText::RichText(RichText::new("MinesBooMer!").size(50.));
            let title_label = Label::new(title);
            ui.add(title_label);
            ui.separator();
            ui.add_space(10.);
            ui.label("Chose a game to join or create a new one.");
            ui.add_space(10.);
            ui.vertical_centered(|ui| {
                if ui.button("New game").clicked() {
                    self.show_game_name_popup = true;
                }
                if !game_list.is_empty() {
                    ui.add_space(10.);
                    ui.label("Current games:");
                    ui.add_space(10.);
                }
                game_list.iter().for_each(|game| {
                    if ui.add_sized([150., 30.], Button::new(&game.name)).clicked() {
                        self.send_join_game_message(&game.game_id);
                    }
                });
            });
        });

        if self.show_game_name_popup {
            self.show_game_creation_window(ui);
        }
    }

    fn show_game_creation_window(&mut self, ui: &Ui) {
        let closed = self.game_creation_view.show(ui.ctx(), |name| {
            self.game_name = name;
            self.show_game_name_popup = false;
        });

        if !self.game_name.is_empty() {
            self.send_create_new_game_message(&self.game_name);
            self.game_name = "".to_owned();
        }

        self.show_game_name_popup = !closed;
    }

    fn draw_waiting_screen(&self, ui: &mut Ui) {
        ui.heading("Waiting for your enemy to connect...");
    }

    fn on_cell_tapped(&mut self, cell: &Cell) {
        if !self.is_active {
            return;
        }
        if self.game.winner().is_none() {
            self.game.player_selected(cell.coordinates);

            self.send_selected_message(cell);
        }
    }

    pub fn set_is_active(&mut self, is_active: bool) {
        self.is_active = is_active;
    }

    pub fn remote_player_selected(&mut self, coordinates: Point) {
        self.game.player_selected(coordinates);
    }

    pub fn set_board(&mut self, board: Board) {
        self.game.game.board = board
    }

    pub fn present_open_games_menu(&mut self, games: Vec<OpenGame>) {
        self.show_games_list = Some(games);
    }

    pub fn close_open_games_menu(&mut self) {
        self.show_games_list = None;
    }
}

impl eframe::App for MinesBoomer {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                if self.show_games_list.is_some() {
                    let list = self.show_games_list.as_ref().unwrap();
                    self.draw_game_list(ui, &list.clone());
                    return;
                }
                if self.waiting_for_enemy {
                    self.draw_waiting_screen(ui);
                    return;
                }
                self.draw_board(ui);
                self.draw_gui(ui);
            });
        });
    }
}

// Messages

impl MinesBoomer {
    pub fn request_user_id(&self) {
        println!("<- Sending player identification");
        let name = "Player".to_owned();
        let message = IdentificationMessage::new(name);

        self.send_message(message.to_json_string());
    }

    pub fn request_open_games(&self) {
        println!("<- Sending games request");
        let message = SimpleMessage::new("games_request");
        self.send_message(message.to_json_string());
    }

    pub fn send_selected_message(&self, cell: &Cell) {
        println!("<- Sending cell selected");
        let serializable: SerializablePoint = cell.coordinates.into();
        let message = CellSelectedMessage::new(serializable, false);
        self.send_message(message.to_json_string());
    }

    fn send_join_game_message(&self, game_id: impl Into<String>) {
        println!("<- Sending joing game");
        let message = JoinGameMessage::new(game_id, "Great Player");
        self.send_message(message.to_json_string());
    }

    fn send_message(&self, json: String) {
        self.sender.unbounded_send(Message::Text(json)).unwrap();
    }
}

fn get_color_for_cell(cell: &Cell) -> Color32 {
    if cell.is_mine() && cell.cleared {
        Color32::from_rgba_premultiplied(150, 29, 27, 100)
    } else if cell.cleared {
        Color32::GRAY
    } else {
        Color32::from_gray(55)
    }
}

fn get_text_for_cell(cell: &Cell) -> WidgetText {
    let text = |cell: &Cell| {
        if cell.cleared && !cell.is_mine() && cell.number > 0 {
            cell.number.to_string()
        } else {
            "".to_string()
        }
    };

    WidgetText::RichText(RichText::new(text(cell)).size(20.).color(Color32::BLACK).text_style(TextStyle::Button))
}

pub trait MessageSender {
    fn send_create_new_game_message(&self, name: &str);
}

impl MessageSender for MinesBoomer {
    fn send_create_new_game_message(&self, game_name: &str) {
        println!("<- Sending create new game");
        let message = CreateGameMessage::new(game_name, Difficulty::Easy);
        self.send_message(message.to_json_string());
    }
}

#[derive(Default, Debug)]
struct GameCreationView {
    name: String,
}

impl GameCreationView {
    fn show(&mut self, ctx: &egui::Context, on_send: impl FnMut(String)) -> bool {
        let mut closed = false;
        egui::Window::new("New Game").resizable(true).default_width(280.0).show(ctx, |ui| {
            self.ui(ui, on_send, &mut closed);
        });
        closed
    }

    fn ui(&mut self, ui: &mut egui::Ui, mut on_send: impl FnMut(String), close: &mut bool) {
        ui.label("The name for the new game:");
        ui.text_edit_singleline(&mut self.name);

        if ui.button(format!("Create game: '{}'", self.name)).clicked() {
            on_send(self.name.clone());
        }

        if ui.button("Cancel").clicked() {
            *close = true;
            // self.show_game_name_popup = false;
            // self.send_create_new_game_message(name);
        }
    }
}
