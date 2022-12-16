use eframe::egui;
use egui::{Color32, Image, Response, Widget};

#[derive(Default)]
pub struct MineImage {
    texture: Option<egui::TextureHandle>,
}

impl MineImage {
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Response {
        let texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
            // Load the texture only once.
            ui.ctx().load_texture("mine", MineImage::load(), Default::default())
        });

        Image::new(texture.id(), [50., 50.]).bg_fill(Color32::from_rgba_premultiplied(150, 29, 27, 100)).ui(ui)
    }

    fn load() -> egui::ColorImage {
        use image::io::Reader;
        let image = Reader::open("assets/mine.png").ok().unwrap().decode().ok().unwrap();
        let size = [image.width() as _, image.height() as _];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();
        egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice())
    }
}
