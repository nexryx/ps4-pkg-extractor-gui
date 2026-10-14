use eframe::egui::{ColorImage, IconData};

const WINDOW_PNG: &[u8] = include_bytes!("../../assets/icon-256.png");
const LOGO_PNG: &[u8] = include_bytes!("../../assets/icon-128.png");

pub fn icon() -> IconData {
    eframe::icon_data::from_png_bytes(WINDOW_PNG).unwrap_or_default()
}

pub fn logo() -> ColorImage {
    let data = eframe::icon_data::from_png_bytes(LOGO_PNG).unwrap_or_default();
    ColorImage::from_rgba_unmultiplied(
        [data.width as usize, data.height as usize],
        &data.rgba,
    )
}
