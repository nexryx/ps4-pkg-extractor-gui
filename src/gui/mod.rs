mod app;
mod extract;
mod fonts;
mod i18n;
mod icons;
mod menu;
mod palette;
mod settings;
mod widgets;
mod window_icon;

use app::{App, Inbox, Incoming};
use eframe::egui;
use fastframe_instance::{Claim, Slot};
use settings::{AppDirs, Settings};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const APP_ID: &str = "dev.afkarxyz.pkgextract";
const WINDOW_SIZE: [f32; 2] = [440.0, 384.0];

pub fn run(initial_pkg: Option<PathBuf>) -> eframe::Result {
    let dirs = AppDirs::discover();

    let _ = fastframe_log::Logging::new("pkgextract", env!("CARGO_PKG_VERSION"))
        .filter("warn,pkgextract=info")
        .file(dirs.log_file())
        .panic_log(dirs.panic_log())
        .init();

    let inbox: Inbox = Arc::new(Mutex::new(Vec::new()));
    let wake: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));

    let request = match &initial_pkg {
        Some(path) => format!("open {}", path.display()),
        None => "show".to_owned(),
    };
    let _guard = match Slot::new(APP_ID).claim(&request, {
        let inbox = Arc::clone(&inbox);
        let wake = Arc::clone(&wake);
        move |request| {
            let incoming = if request == "show" {
                Incoming::Show
            } else {
                Incoming::Open(PathBuf::from(request.strip_prefix("open ")?))
            };
            if let Ok(mut queue) = inbox.lock() {
                queue.push(incoming);
            }
            if let Ok(ctx) = wake.lock()
                && let Some(ctx) = ctx.as_ref()
            {
                ctx.request_repaint();
            }
            Some("ok".to_owned())
        }
    }) {
        Claim::First(guard) => guard,
        Claim::Running(_) | Claim::Declined | Claim::Unanswered => return Ok(()),
    };

    let settings = Settings::load(&dirs.settings_file());
    let viewport = egui::ViewportBuilder::default()
        .with_title("PS4 PKG Extractor")
        .with_app_id(APP_ID)
        .with_icon(window_icon::icon())
        .with_inner_size(WINDOW_SIZE)
        .with_min_inner_size(WINDOW_SIZE)
        .with_max_inner_size(WINDOW_SIZE)
        .with_resizable(false)
        .with_maximize_button(false);

    eframe::run_native(
        "PS4 PKG Extractor",
        eframe::NativeOptions {
            viewport,
            centered: true,
            ..Default::default()
        },
        Box::new(move |cc| {
            if let Ok(mut slot) = wake.lock() {
                *slot = Some(cc.egui_ctx.clone());
            }
            Ok(Box::new(App::new(cc, dirs, settings, initial_pkg, inbox)))
        }),
    )
}
