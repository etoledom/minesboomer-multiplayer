use super::mine_image::MineImage;
use eframe::egui;
use egui::{Button, Color32, RichText, TextStyle, Ui, WidgetText};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use minesboomer_utils::*;
use minesweeper_multiplayer::*;

use std::sync::{Arc, Mutex};
// use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::protocol::Message;

pub struct MinesBoomer {
    pub game: Multiplayer,
    mine: MineImage,
    sender: UnboundedSender<Message>,
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

    fn draw_gui(&self, ui: &mut Ui) {
        if let Some(winner) = self.game.winner() {
            ui.vertical_centered_justified(|ui| {
                ui.heading("WINNER!");
                ui.heading(winner.name.to_string());
            });
            return;
        }

        let current_player = self.game.current_player().name.clone();
        let remining_mines = self.game.game.remaining_mines();
        let mines_to_win = self.game.remaining_to_win();
        let winning = self.game.player_winning();

        ui.vertical_centered_justified(|ui| {
            ui.heading(current_player);
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

    fn on_cell_tapped(&mut self, cell: &Cell) {
        if self.game.winner().is_none() {
            self.game.player_selected(cell.coordinates);

            let serializable: SerializablePoint = cell.coordinates.into();
            let message = CellSelectedMessage::new(serializable);
            self.sender.unbounded_send(Message::Text(message.to_json_string())).unwrap();
        }
    }
}

impl eframe::App for MinesBoomer {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                self.draw_board(ui);
                self.draw_gui(ui);
            });
        });
    }
}

impl MinesBoomer {
    pub fn request_user_id(&self) {
        let name = "Player".to_owned();
        let message = IdentificationMessage::new(name);

        self.sender.unbounded_send(Message::Text(message.to_json_string())).unwrap();
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
