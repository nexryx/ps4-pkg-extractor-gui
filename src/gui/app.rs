use crate::gui::extract::{Phase, Shared, default_output_dir, run_extraction};
use crate::gui::i18n::Locale;
use crate::gui::icons::Icon;
use crate::gui::menu::{icon_button, menu_item};
use crate::gui::palette::{self, Colors};
use crate::gui::settings::{AppDirs, DEFAULT_THEME, Settings};
use crate::gui::widgets::{
    BODY_HEIGHT, Kind, Note, button, card, card_outer_height, dropzone, form_card, path_field,
    progress_bar, section_title, stat_columns, stat_flow, summary_row, toggle_row,
};
use eframe::egui::{
    self, Align, Frame, Layout, Margin, Panel, Popup, RectAlign, Response, RichText, Vec2,
    vec2,
};
use fastframe_fonts::Weight;
use fastframe_scroll::Scrolling;
use fastframe_text::TextRendering;
use fastframe_theme::{
    Catalog, DesktopThemes, Status as ThemeStatus, Transition, Waker, display_name, omarchy,
};
use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub enum Incoming {
    Show,
    Open(PathBuf),
}

pub type Inbox = Arc<Mutex<Vec<Incoming>>>;

#[derive(Clone)]
struct Summary {
    dir: PathBuf,
    elapsed: Duration,
    files: usize,
    bytes: u64,
    skipped: usize,
}

#[derive(Clone)]
enum RunState {
    Idle,
    Running { started: Instant },
    Done(Summary),
    Failed(String),
}

pub struct App {
    dirs: AppDirs,
    settings: Settings,
    locale: Locale,
    colors: Colors,
    logo: egui::TextureHandle,
    rendering: TextRendering,
    initial_load: bool,
    catalog: Catalog<Colors>,
    waker: Waker,
    transition: Transition,
    scrolling: Scrolling,
    pkg: Option<PathBuf>,
    pkg_size: Option<u64>,
    output: Option<PathBuf>,
    overwrite: bool,
    state: RunState,
    shared: Arc<Shared>,
    result_rx: Option<Receiver<Result<PathBuf, String>>>,
    inbox: Inbox,
    centered: bool,
    dirty: bool,
    last_save: Instant,
    copied_at: Option<f64>,
}

// The shared palettes are compiled in, so the first frame already has its
// colors before the themes folder has been scanned.
fn initial_colors(name: &str) -> Colors {
    let find = |wanted: &str| {
        fastframe_theme::presets::themes::<Colors>().find(|theme| theme.filename == wanted)
    };
    find(name)
        .or_else(|| find(DEFAULT_THEME))
        .map_or(Colors::DARK, |theme| theme.palette)
}

fn format_duration(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s % 3600 / 60, s % 60)
    } else {
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn open_in_file_manager(dir: &Path) {
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let program = "xdg-open";
    // The console is gone when launched by double-click, so inherited standard
    // handles are invalid and the child cannot start without these.
    let spawned = Command::new(program)
        .arg(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match spawned {
        Ok(child) => log::info!("opened {} with {program} (pid {})", dir.display(), child.id()),
        Err(error) => log::warn!("could not open {} with {program}: {error}", dir.display()),
    }
}

fn focus_window(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        dirs: AppDirs,
        settings: Settings,
        initial_pkg: Option<PathBuf>,
        inbox: Inbox,
    ) -> Self {
        let ctx = &cc.egui_ctx;

        let mut fonts = crate::gui::fonts::definitions();
        let rendering: TextRendering = fastframe_text::detect();
        rendering.apply_to(&mut fonts);
        ctx.set_fonts(fonts);

        ctx.options_mut(|options| options.zoom_with_keyboard = false);
        egui_extras::install_image_loaders(ctx);
        fastframe_icons::install::<Icon>(ctx);
        let logo = ctx.load_texture(
            "pkgextract-logo",
            crate::gui::window_icon::logo(),
            egui::TextureOptions::LINEAR,
        );

        let colors = initial_colors(&settings.theme);
        palette::apply(ctx, &colors, &rendering);

        let waker = {
            let ctx = ctx.clone();
            Waker::new(move || ctx.request_repaint())
        };
        let mut catalog = Catalog::<Colors>::default();
        catalog.enable_desktop_themes(DesktopThemes {
            slug: "pkgextract",
            omarchy_template: omarchy::BASE_TEMPLATE,
            omarchy_previous_templates: &[],
            presets: true,
        });
        catalog.start(dirs.themes_dir(), Some(settings.theme.clone()), &waker);

        let locale = settings.language.unwrap_or_else(Locale::detect);
        let mut app = Self {
            dirs,
            settings,
            locale,
            colors,
            logo,
            rendering,
            initial_load: true,
            catalog,
            waker,
            transition: Transition::default(),
            scrolling: Scrolling::default(),
            pkg: None,
            pkg_size: None,
            output: None,
            overwrite: false,
            state: RunState::Idle,
            shared: Arc::new(Shared::default()),
            result_rx: None,
            inbox,
            centered: false,
            dirty: false,
            last_save: Instant::now(),
            copied_at: None,
        };
        if let Some(path) = initial_pkg {
            app.set_pkg(path);
        }
        app
    }

    fn t(&self, source: &'static str) -> Cow<'static, str> {
        self.locale.tr(source)
    }

    fn running(&self) -> bool {
        matches!(self.state, RunState::Running { .. })
    }

    fn set_pkg(&mut self, path: PathBuf) {
        self.pkg_size = fs::metadata(&path).ok().map(|m| m.len());
        self.pkg = Some(path);
        self.output = None;
        self.state = RunState::Idle;
    }

    fn effective_output(&self) -> Option<PathBuf> {
        self.output
            .clone()
            .or_else(|| self.pkg.as_deref().and_then(default_output_dir))
    }

    fn browse_pkg(&mut self) {
        if self.running() {
            return;
        }
        let mut dialog = rfd::FileDialog::new().add_filter("PS4 package", &["pkg"]);
        if let Some(parent) = self.pkg.as_deref().and_then(Path::parent) {
            dialog = dialog.set_directory(parent);
        }
        if let Some(path) = dialog.pick_file() {
            self.set_pkg(path);
        }
    }

    fn browse_output(&mut self) {
        if self.running() {
            return;
        }
        let mut dialog = rfd::FileDialog::new();
        if let Some(parent) = self.effective_output().as_deref().and_then(Path::parent) {
            dialog = dialog.set_directory(parent);
        }
        if let Some(dir) = dialog.pick_folder() {
            self.output = Some(dir);
        }
    }

    fn start(&mut self, ctx: &egui::Context) {
        let (Some(pkg), Some(output)) = (self.pkg.clone(), self.effective_output()) else {
            return;
        };
        self.shared = Arc::new(Shared::default());
        let shared = Arc::clone(&self.shared);
        let overwrite = self.overwrite;
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let result =
                run_extraction(pkg, output, overwrite, shared).map_err(|e| format!("{e:#}"));
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.result_rx = Some(rx);
        self.state = RunState::Running {
            started: Instant::now(),
        };
    }

    fn poll_result(&mut self) {
        let Some(rx) = &self.result_rx else { return };
        let RunState::Running { started } = self.state else {
            return;
        };
        let outcome = match rx.try_recv() {
            Ok(outcome) => outcome,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("extraction thread stopped unexpectedly".to_owned())
            }
        };
        self.result_rx = None;
        self.state = match outcome {
            Ok(dir) => RunState::Done(Summary {
                dir,
                elapsed: started.elapsed(),
                files: match self.shared.phase() {
                    Phase::Files { total, .. } => total,
                    Phase::Reading => 0,
                },
                bytes: self.shared.bytes(),
                skipped: self.shared.skipped(),
            }),
            Err(message) => {
                log::warn!("extraction failed: {message}");
                RunState::Failed(message)
            }
        };
    }

    fn poll_catalog(&mut self, ctx: &egui::Context) {
        if self.catalog.needs_reload() {
            self.catalog.start(
                self.dirs.themes_dir(),
                Some(self.settings.theme.clone()),
                &self.waker,
            );
        }
        if self.catalog.poll() {
            ctx.request_repaint();
        }
    }

    fn drain_inbox(&mut self, ctx: &egui::Context) {
        let items: Vec<Incoming> = self
            .inbox
            .lock()
            .map(|mut queue| std::mem::take(&mut *queue))
            .unwrap_or_default();
        for item in items {
            if let Incoming::Open(path) = item
                && !self.running()
                && path.is_file()
            {
                self.set_pkg(path);
            }
            focus_window(ctx);
        }
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        if self.running() {
            return;
        }
        let dropped = ctx.input(|i| {
            i.raw
                .dropped_files
                .first()
                .map(|f| f.path().to_path_buf())
        });
        if let Some(path) = dropped
            && path.is_file()
        {
            self.set_pkg(path);
        }
    }

    fn save(&mut self, sync: bool) {
        self.dirty = false;
        self.last_save = Instant::now();
        let path = self.dirs.settings_file();
        let bytes = self.settings.to_bytes();
        let write = move || {
            if let Err(error) = fastframe_file::write(&path, bytes) {
                log::warn!("could not save settings: {error}");
            }
        };
        if sync {
            write();
        } else {
            thread::spawn(write);
        }
    }

    fn resolve_colors(&self) -> Colors {
        self.catalog
            .find(&self.settings.theme)
            .map_or(self.colors, |theme| theme.palette)
    }

    fn sync_theme(&mut self, ctx: &egui::Context) {
        let wanted = self.resolve_colors();
        let settled = !self.catalog.loading();
        if wanted != self.colors {
            let apply = if self.initial_load {
                true
            } else {
                self.transition.begin(ctx);
                !self.transition.holding(ctx)
            };
            if apply {
                palette::apply(ctx, &wanted, &self.rendering);
                self.colors = wanted;
            }
        }
        if self.initial_load && settled && wanted == self.colors {
            self.initial_load = false;
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.centered
            && let Some(command) = egui::ViewportCommand::center_on_screen(ctx)
        {
            ctx.send_viewport_cmd(command);
            self.centered = true;
        }

        self.poll_result();
        self.poll_catalog(ctx);
        self.drain_inbox(ctx);
        self.handle_drops(ctx);

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.browse_pkg();
        }

        let closing = ctx.input(|i| i.viewport().close_requested());
        if self.dirty && (closing || self.last_save.elapsed() > Duration::from_secs(1)) {
            self.save(closing);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.scrolling.apply(&ctx);
        self.sync_theme(&ctx);
        if self.running() {
            ctx.request_repaint_after(Duration::from_millis(80));
        }

        let c = self.colors;
        Panel::bottom("footer")
            .frame(
                Frame::new()
                    .fill(c.window)
                    .inner_margin(Margin {
                        left: 14,
                        right: 14,
                        top: 10,
                        bottom: 14,
                    }),
            )
            .show(ui, |ui| self.footer(ui, &ctx, &c));
        egui::CentralPanel::default_margins()
            .frame(Frame::new().fill(c.window).inner_margin(14))
            .show(ui, |ui| self.body(ui, &ctx, &c));

        self.transition.paint(&ctx);
    }
}

impl App {
    fn body(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, c: &Colors) {
        ui.spacing_mut().item_spacing.y = 8.0;
        self.header(ui, c);
        ui.add_space(4.0);
        match self.state.clone() {
            RunState::Idle => {
                self.source_card(ui, ctx, c);
                self.output_card(ui, c);
            }
            RunState::Running { started } => {
                self.summary_card(ui, c);
                self.running_card(ui, c, started);
            }
            RunState::Done(summary) => {
                self.summary_card(ui, c);
                self.done_card(ui, c, &summary);
            }
            RunState::Failed(message) => {
                self.summary_card(ui, c);
                self.failed_card(ui, ctx, c, &message);
            }
        }
    }

    fn header(&mut self, ui: &mut egui::Ui, c: &Colors) {
        ui.horizontal(|ui| {
            ui.add(egui::Image::new(&self.logo).fit_to_exact_size(Vec2::splat(36.0)));

            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(
                    RichText::new("PS4 PKG Extractor")
                        .font(Weight::SemiBold.font_id(14.0))
                        .color(c.text),
                );
                ui.label(
                    RichText::new(self.t("Extract PlayStation 4 .pkg files"))
                        .size(11.5)
                        .color(c.dim),
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let language = icon_button(ui, c, Icon::Globe, &self.t("Language"));
                self.language_menu(&language, c);
                let theme = icon_button(ui, c, Icon::Palette, &self.t("Theme"));
                self.theme_menu(&theme, c);
            });
        });
    }

    fn language_menu(&mut self, button: &Response, c: &Colors) {
        let mut chosen = None;
        Popup::menu(button)
            .align(RectAlign::BOTTOM_END)
            .width(170.0)
            .show(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for locale in Locale::ALL {
                    if menu_item(ui, c, locale.native_name(), locale == self.locale, None) {
                        chosen = Some(locale);
                    }
                }
            });
        if let Some(locale) = chosen {
            self.locale = locale;
            self.settings.language = Some(locale);
            self.dirty = true;
        }
    }

    fn theme_menu(&mut self, button: &Response, c: &Colors) {
        let mut chosen = None;
        Popup::menu(button)
            .align(RectAlign::BOTTOM_END)
            .width(200.0)
            .show(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let current = &self.settings.theme;
                for theme in self.catalog.picker_themes() {
                    if menu_item(
                        ui,
                        c,
                        display_name(&theme.filename),
                        theme.filename == *current,
                        Some(theme.palette.accent),
                    ) {
                        chosen = Some(theme.filename.clone());
                    }
                }

                let note = match self.catalog.status(Some(current.as_str())) {
                    Some(ThemeStatus::Loading) => Some(self.t("Loading themes...")),
                    Some(ThemeStatus::SelectedUnavailable) => {
                        Some(self.t("The selected theme is missing"))
                    }
                    Some(ThemeStatus::Problem(_)) => Some(self.t("Themes could not be listed")),
                    _ => None,
                };
                if let Some(note) = note {
                    ui.add_space(4.0);
                    ui.label(RichText::new(note).size(11.0).color(c.dim));
                }
            });
        if let Some(theme) = chosen {
            self.settings.theme = theme;
            self.dirty = true;
        }
    }

    fn source_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, c: &Colors) {
        let idle = !self.running();
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());

        let Some(path) = self.pkg.clone() else {
            let title = if hovering {
                self.t("Release to open")
            } else {
                self.t("Drop a .pkg file here")
            };
            let hint = self.t("or click to browse");
            let height = card_outer_height(BODY_HEIGHT);
            if dropzone(ui, c, hovering, height, &title, &hint).clicked() {
                self.browse_pkg();
            }
            return;
        };

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let dir = path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let size = self.pkg_size.map(format_bytes);
        let title = self.t("PKG file");
        let change_label = self.t("Change");
        let mut change = false;

        let stroke = if hovering { c.accent } else { c.outline };
        form_card(ui, c, stroke, |ui| {
            section_title(ui, c, Icon::FileArchive, &title, None);
            ui.horizontal(|ui| {
                let button_w = 84.0;
                let w = (ui.available_width() - button_w - ui.spacing().item_spacing.x).max(80.0);
                ui.vertical(|ui| {
                    ui.set_width(w);
                    ui.spacing_mut().item_spacing.y = 3.0;
                    ui.add(
                        egui::Label::new(
                            RichText::new(name)
                                .font(Weight::Medium.font_id(13.0))
                                .color(c.text),
                        )
                        .truncate(),
                    );
                    ui.add(
                        egui::Label::new(RichText::new(dir).size(11.0).color(c.dim)).truncate(),
                    );
                    if let Some(size) = size {
                        ui.label(RichText::new(size).size(11.0).color(c.secondary));
                    }
                });
                if button(
                    ui,
                    c,
                    None,
                    &change_label,
                    Kind::Secondary,
                    idle,
                    vec2(button_w, 28.0),
                )
                .clicked()
                {
                    change = true;
                }
            });
        });
        if change {
            self.browse_pkg();
        }
    }

    fn output_card(&mut self, ui: &mut egui::Ui, c: &Colors) {
        let idle = !self.running();
        let output = self.effective_output();
        let exists = self.pkg.is_some() && output.as_ref().is_some_and(|p| p.exists());

        let title = self.t("Output folder");
        let placeholder = self.t("Defaults to a folder next to the PKG");
        let browse_label = self.t("Browse");
        let replace_label = self.t("Replace existing output folder");
        let delete_note = self.t("The folder is deleted before extracting");
        let exists_note = self.t("This folder already exists");

        let mut overwrite = self.overwrite;
        let mut browse = false;

        let output_text = output.as_ref().map(|path| path.display().to_string());
        let note = if exists && overwrite {
            Some(Note {
                icon: Icon::Alert,
                color: c.danger,
                text: &delete_note,
            })
        } else if exists {
            Some(Note {
                icon: Icon::Alert,
                color: c.warning,
                text: &exists_note,
            })
        } else {
            None
        };

        form_card(ui, c, c.outline, |ui| {
            section_title(ui, c, Icon::FolderOpen, &title, note);
            ui.horizontal(|ui| {
                let button_w = 84.0;
                let w = (ui.available_width() - button_w - ui.spacing().item_spacing.x).max(80.0);
                path_field(ui, c, output_text.as_deref(), &placeholder, w);
                if button(
                    ui,
                    c,
                    None,
                    &browse_label,
                    Kind::Secondary,
                    idle,
                    vec2(button_w, 28.0),
                )
                .clicked()
                {
                    browse = true;
                }
            });

            toggle_row(ui, c, &mut overwrite, idle, &replace_label);
        });

        self.overwrite = overwrite;
        if browse {
            self.browse_output();
        }
    }

    fn summary_card(&self, ui: &mut egui::Ui, c: &Colors) {
        let name = self
            .pkg
            .as_deref()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let size = self.pkg_size.map(format_bytes);
        let output = self
            .effective_output()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        card(ui, c, c.outline, |ui| {
            summary_row(ui, c, Icon::FileArchive, &name, size.as_deref(), true);
            summary_row(ui, c, Icon::FolderOpen, &output, None, false);
        });
    }

    fn footer(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, c: &Colors) {
        let now = ctx.input(|i| i.time);
        let copied = self.copied_at.is_some_and(|at| now - at < 1.5);
        if copied {
            ctx.request_repaint_after(Duration::from_millis(200));
        }

        let state = self.state.clone();
        let has_pkg = self.pkg.is_some();
        let hint = match (&state, has_pkg) {
            (RunState::Idle, false) => Some(self.t("Choose a .pkg file to begin")),
            (RunState::Idle, true) => Some(self.t("Ready to extract")),
            _ => None,
        };
        let extract_label = self.t("Extract");
        let extracting_label = self.t("Extracting");
        let open_label = self.t("Open folder");
        let new_label = self.t("New extraction");
        let dismiss_label = self.t("Dismiss");
        let copy_label = if copied {
            self.t("Copied")
        } else {
            self.t("Copy path")
        };

        let (mut start, mut open, mut copy, mut reset) = (false, false, false, false);
        ui.spacing_mut().interact_size.y = 32.0;
        ui.horizontal(|ui| {
            if let Some(hint) = &hint {
                ui.label(RichText::new(hint.as_ref()).size(12.0).color(c.dim));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| match &state {
                RunState::Idle => {
                    let size = vec2(112.0, 32.0);
                    start = button(ui, c, Some(Icon::PackageOpen), &extract_label, Kind::Primary, has_pkg, size)
                        .clicked();
                }
                RunState::Running { .. } => {
                    let size = vec2(112.0, 32.0);
                    button(ui, c, Some(Icon::PackageOpen), &extracting_label, Kind::Busy, false, size);
                }
                RunState::Done(_) => {
                    let size = vec2(0.0, 32.0);
                    open = button(ui, c, Some(Icon::FolderOpen), &open_label, Kind::Primary, true, size)
                        .clicked();
                    reset = button(ui, c, None, &new_label, Kind::Secondary, true, size).clicked();
                    let icon = if copied { Icon::Check } else { Icon::Copy };
                    copy = button(ui, c, Some(icon), &copy_label, Kind::Ghost, true, size).clicked();
                }
                RunState::Failed(_) => {
                    let size = vec2(0.0, 32.0);
                    reset = button(ui, c, None, &dismiss_label, Kind::Primary, true, size).clicked();
                }
            });
        });

        if start {
            self.start(ctx);
        }
        if reset {
            self.state = RunState::Idle;
        }
        if let RunState::Done(summary) = &state {
            if open {
                open_in_file_manager(&summary.dir);
            }
            if copy {
                ctx.copy_text(summary.dir.display().to_string());
                self.copied_at = Some(now);
            }
        }
    }

    fn running_card(&self, ui: &mut egui::Ui, c: &Colors, started: Instant) {
        let fraction = self.shared.fraction();
        let elapsed = started.elapsed();
        let phase = self.shared.phase();

        let label = match phase {
            Phase::Reading => self.t("Reading package entries"),
            Phase::Files { .. } => self.t("Extracting files"),
        };
        let files = match phase {
            Phase::Files { done, total } => Some(
                self.t("{done} of {total} files")
                    .replace("{done}", &done.to_string())
                    .replace("{total}", &total.to_string()),
            ),
            Phase::Reading => None,
        };
        let written = self
            .t("{written} written")
            .replace("{written}", &format_bytes(self.shared.bytes()));
        let elapsed_text = self
            .t("Elapsed {time}")
            .replace("{time}", &format_duration(elapsed));
        let current = self.shared.current();
        let widest = [
            self.t("{done} of {total} files")
                .replace("{done}", "99999")
                .replace("{total}", "99999"),
            self.t("{written} written").replace("{written}", "999.9 GB"),
            self.t("Elapsed {time}").replace("{time}", "0:00:00"),
        ];

        card(ui, c, c.outline, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(13.0).color(c.accent));
                ui.label(
                    RichText::new(label.as_ref())
                        .font(Weight::Medium.font_id(12.5))
                        .color(c.text),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{:.0}%", fraction * 100.0))
                            .font(Weight::SemiBold.font_id(14.0))
                            .color(c.accent_hover),
                    );
                });
            });
            progress_bar(ui, c, fraction);
            ui.add(
                egui::Label::new(RichText::new(current).size(11.0).color(c.dim)).truncate(),
            );
            stat_columns(
                ui,
                c,
                [
                    files.map(|text| (Icon::FileArchive, text)),
                    Some((Icon::HardDrive, written)),
                    Some((Icon::Clock, elapsed_text)),
                ],
                [&widest[0], &widest[1], &widest[2]],
            );
        });
    }

    fn done_card(&self, ui: &mut egui::Ui, c: &Colors, s: &Summary) {
        let title = self.t("Extraction finished");
        let completed = self
            .t("Completed in {time}")
            .replace("{time}", &format_duration(s.elapsed));
        let files = self
            .t("{done} of {total} files")
            .replace("{done}", &s.files.to_string())
            .replace("{total}", &s.files.to_string());
        let written = self
            .t("{written} written")
            .replace("{written}", &format_bytes(s.bytes));
        let skipped = (s.skipped > 0).then(|| {
            self.t("Skipped, no decryption key: {count}")
                .replace("{count}", &s.skipped.to_string())
        });

        card(ui, c, c.success, |ui| {
            ui.horizontal(|ui| {
                ui.add(Icon::Success.image(c.success, 22.0));
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new(title.as_ref())
                            .font(Weight::SemiBold.font_id(14.0))
                            .color(c.text),
                    );
                    ui.label(RichText::new(completed).size(11.5).color(c.dim));
                });
            });
            let mut stats = Vec::with_capacity(3);
            if s.files > 0 {
                stats.push((Icon::FileArchive, files));
            }
            stats.push((Icon::HardDrive, written));
            if let Some(skipped) = skipped {
                stats.push((Icon::Alert, skipped));
            }
            stat_flow(ui, c, &stats);
        });
    }

    fn failed_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, c: &Colors, message: &str) {
        let now = ctx.input(|i| i.time);
        let copied = self.copied_at.is_some_and(|at| now - at < 1.5);
        if copied {
            ctx.request_repaint_after(Duration::from_millis(200));
        }

        let title = self.t("Extraction failed");
        let copy_label = if copied {
            self.t("Copied")
        } else {
            self.t("Copy error")
        };

        let mut copy = false;
        card(ui, c, c.danger, |ui| {
            ui.horizontal(|ui| {
                ui.add(Icon::Failure.image(c.danger, 22.0));
                ui.label(
                    RichText::new(title.as_ref())
                        .font(Weight::SemiBold.font_id(14.0))
                        .color(c.text),
                );
            });
            egui::ScrollArea::vertical()
                .id_salt("error-message")
                .max_height(72.0)
                .show(ui, |ui| {
                    ui.label(RichText::new(message).size(11.5).color(c.secondary));
                });
            let icon = if copied { Icon::Check } else { Icon::Copy };
            copy = button(
                ui,
                c,
                Some(icon),
                &copy_label,
                Kind::Secondary,
                true,
                vec2(0.0, 28.0),
            )
            .clicked();
        });

        if copy {
            ctx.copy_text(message.to_owned());
            self.copied_at = Some(now);
        }
    }
}
