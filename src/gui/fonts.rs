use eframe::egui::epaint::text::VariationCoords;
use eframe::egui::{FontData, FontDefinitions, FontFamily};
use fastframe_fonts::{FontSetup, Weight};
use std::sync::Arc;

const FIGTREE: &[u8] = include_bytes!("../../assets/fonts/Figtree-Variable.ttf");

// fastframe-fonts names its interface face "inter" whatever it draws, so the
// Figtree data goes under those keys and the families are completed here,
// in case the system face it normally starts from was not found.
pub fn definitions() -> FontDefinitions {
    let mut fonts = FontSetup::default().definitions();
    for weight in Weight::ALL {
        let mut data = FontData::from_static(FIGTREE);
        data.tweak.coords = VariationCoords::new([(b"wght", weight.value())]);
        fonts.font_data.insert(weight.name().to_owned(), Arc::new(data));
    }

    let regular = Weight::Regular.name().to_owned();
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    if proportional.first() != Some(&regular) {
        proportional.insert(0, regular.clone());
    }
    let behind: Vec<String> = proportional.iter().skip(1).cloned().collect();
    for weight in Weight::ALL.into_iter().skip(1) {
        let family = fonts.families.entry(weight.family()).or_default();
        if family.first().map(String::as_str) != Some(weight.name()) {
            family.insert(0, weight.name().to_owned());
        }
        if family.len() == 1 {
            family.extend(behind.iter().cloned());
        }
    }
    fonts
}
