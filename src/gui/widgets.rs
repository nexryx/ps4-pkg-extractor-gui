use crate::gui::icons::Icon;
use crate::gui::palette::Colors;
use eframe::egui::{
    self, Color32, CursorIcon, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2, WidgetInfo, WidgetType, pos2, text::LayoutJob, vec2,
};
use fastframe_fonts::Weight;
use std::f32::consts::{FRAC_PI_2, PI};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Busy,
    Secondary,
    Ghost,
}

const CARD_MARGIN: i8 = 12;

/// Height of the body of the folder and file cards, so the drop zone can match them.
pub const BODY_HEIGHT: f32 = 86.0;

pub fn card_outer_height(body: f32) -> f32 {
    body + f32::from(CARD_MARGIN) * 2.0 + 2.0
}

fn card_frame<R>(
    ui: &mut Ui,
    c: &Colors,
    stroke: Color32,
    min_body: f32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    egui::Frame::new()
        .fill(c.panel)
        .stroke(Stroke::new(1.0, stroke))
        .corner_radius(10)
        .inner_margin(CARD_MARGIN)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(min_body);
            add_contents(ui)
        })
        .inner
}

pub fn card<R>(
    ui: &mut Ui,
    c: &Colors,
    stroke: Color32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    card_frame(ui, c, stroke, 0.0, add_contents)
}

pub fn form_card<R>(
    ui: &mut Ui,
    c: &Colors,
    stroke: Color32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    card_frame(ui, c, stroke, BODY_HEIGHT, add_contents)
}

pub struct Note<'a> {
    pub icon: Icon,
    pub color: Color32,
    pub text: &'a str,
}

pub fn section_title(ui: &mut Ui, c: &Colors, icon: Icon, text: &str, note: Option<Note<'_>>) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
    let ctx = ui.ctx().clone();
    icon.image(c.secondary, 14.0).paint_at(
        ui,
        Rect::from_center_size(pos2(rect.left() + 7.0, rect.center().y), Vec2::splat(14.0)),
    );
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        Weight::Medium.font_id(12.0),
        Color32::PLACEHOLDER,
    );
    let pos = pos2(
        snap(rect.left() + 22.0, &ctx),
        snap(rect.center().y - galley.size().y / 2.0, &ctx),
    );
    ui.painter().galley(pos, galley, c.secondary);

    if let Some(note) = note {
        let galley = ui.painter().layout_no_wrap(
            note.text.to_owned(),
            Weight::Regular.font_id(11.5),
            Color32::PLACEHOLDER,
        );
        let x = rect.right() - galley.size().x;
        let pos = pos2(snap(x, &ctx), snap(rect.center().y - galley.size().y / 2.0, &ctx));
        ui.painter().galley(pos, galley, note.color);
        note.icon.image(note.color, 13.0).paint_at(
            ui,
            Rect::from_center_size(pos2(x - 12.0, rect.center().y), Vec2::splat(13.0)),
        );
    }
}

const STAT_ROW: f32 = 18.0;
const STAT_ICON: f32 = 12.0;
const STAT_ICON_GAP: f32 = 4.0;

fn stat_width(ui: &Ui, text: &str) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        Weight::Regular.font_id(11.5),
        Color32::PLACEHOLDER,
    );
    STAT_ICON + STAT_ICON_GAP + galley.size().x
}

fn paint_stat(ui: &mut Ui, c: &Colors, icon: Icon, text: &str, left: f32, center_y: f32) {
    let ctx = ui.ctx().clone();
    icon.image(c.dim, STAT_ICON).paint_at(
        ui,
        Rect::from_center_size(pos2(left + STAT_ICON / 2.0, center_y), Vec2::splat(STAT_ICON)),
    );
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        Weight::Regular.font_id(11.5),
        Color32::PLACEHOLDER,
    );
    let pos = pos2(
        snap(left + STAT_ICON + STAT_ICON_GAP, &ctx),
        snap(center_y - galley.size().y / 2.0, &ctx),
    );
    ui.painter().galley(pos, galley, c.secondary);
}

/// Three readings on one row. Each column is as wide as the widest text it can
/// show (`widest`), so a number that grows never pushes its neighbours sideways
/// and the last column never runs past the edge. A missing reading leaves its
/// column empty.
pub fn stat_columns(
    ui: &mut Ui,
    c: &Colors,
    items: [Option<(Icon, String)>; 3],
    widest: [&str; 3],
) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), STAT_ROW), Sense::hover());
    let widths = widest.map(|text| stat_width(ui, text));
    let spare = rect.width() - widths.iter().sum::<f32>();
    let gap = (spare / 2.0).clamp(4.0, 40.0);
    let mut x = 0.0;
    for (item, width) in items.iter().zip(widths) {
        if let Some((icon, text)) = item {
            paint_stat(ui, c, *icon, text, rect.left() + x, rect.center().y);
        }
        x += width + gap;
    }
}

/// Readings that wrap onto further rows when they do not fit.
pub fn stat_flow(ui: &mut Ui, c: &Colors, items: &[(Icon, String)]) {
    const GAP: f32 = 16.0;

    if items.is_empty() {
        return;
    }
    let width = ui.available_width();
    let mut placed = Vec::with_capacity(items.len());
    let (mut x, mut row) = (0.0_f32, 0_usize);
    for (_, text) in items {
        let item_width = stat_width(ui, text);
        if x > 0.0 && x + item_width > width {
            row += 1;
            x = 0.0;
        }
        placed.push((row, x));
        x += item_width + GAP;
    }

    let (rect, _) =
        ui.allocate_exact_size(vec2(width, (row + 1) as f32 * STAT_ROW), Sense::hover());
    for ((icon, text), (row, x)) in items.iter().zip(placed) {
        let center_y = rect.top() + row as f32 * STAT_ROW + STAT_ROW / 2.0;
        paint_stat(ui, c, *icon, text, rect.left() + x, center_y);
    }
}

fn snap(value: f32, ctx: &egui::Context) -> f32 {
    fastframe_text::snap_to_pixels(value, ctx.pixels_per_point())
}

pub fn button(
    ui: &mut Ui,
    c: &Colors,
    icon: Option<Icon>,
    label: &str,
    kind: Kind,
    enabled: bool,
    min_size: Vec2,
) -> Response {
    let icon_size = 14.0;
    let gap = 6.0;
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        Weight::SemiBold.font_id(13.0),
        Color32::PLACEHOLDER,
    );
    let content_w = galley.size().x + icon.map_or(0.0, |_| icon_size + gap);
    let size = vec2((content_w + 24.0).max(min_size.x), min_size.y.max(28.0));
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));

    if ui.is_rect_visible(rect) {
        let t = ui
            .ctx()
            .animate_bool_with_time(response.id, response.hovered() && enabled, 0.12);
        let (fill, stroke, fg) = match (kind, enabled) {
            (Kind::Primary, true) => (
                c.accent.lerp_to_gamma(c.accent_hover, t),
                Stroke::NONE,
                c.on_accent,
            ),
            (Kind::Busy, _) => (
                c.accent.lerp_to_gamma(c.window, 0.45),
                Stroke::NONE,
                c.on_accent.gamma_multiply(0.8),
            ),
            (Kind::Primary, false) => (c.surface, Stroke::NONE, c.dim),
            (Kind::Secondary, true) => (
                c.surface.lerp_to_gamma(c.surface_hover, t),
                Stroke::new(1.0, c.outline.lerp_to_gamma(c.accent, t)),
                c.text,
            ),
            (Kind::Secondary, false) => (c.surface, Stroke::new(1.0, c.outline), c.dim),
            (Kind::Ghost, true) => (
                c.surface.gamma_multiply(t),
                Stroke::new(1.0, c.outline.lerp_to_gamma(c.accent, t)),
                c.secondary.lerp_to_gamma(c.text, t),
            ),
            (Kind::Ghost, false) => (Color32::TRANSPARENT, Stroke::new(1.0, c.outline), c.dim),
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 6, fill);
        if stroke != Stroke::NONE {
            painter.rect_stroke(rect, 6, stroke, StrokeKind::Inside);
        }
        if response.has_focus() {
            painter.rect_stroke(rect, 6, Stroke::new(2.0, c.accent_hover), StrokeKind::Outside);
        }
        let mut x = rect.center().x - content_w / 2.0;
        if let Some(icon) = icon {
            let icon_rect = Rect::from_center_size(
                pos2(x + icon_size / 2.0, rect.center().y),
                Vec2::splat(icon_size),
            );
            icon.image(fg, icon_size).paint_at(ui, icon_rect);
            x += icon_size + gap;
        }
        let pos = pos2(
            snap(x, ui.ctx()),
            snap(rect.center().y - galley.size().y / 2.0, ui.ctx()),
        );
        ui.painter().galley(pos, galley, fg);
    }
    if enabled {
        ui.ctx().set_cursor_icon(if response.hovered() {
            CursorIcon::PointingHand
        } else {
            CursorIcon::Default
        });
    }
    response
}

pub fn toggle_row(
    ui: &mut Ui,
    c: &Colors,
    on: &mut bool,
    enabled: bool,
    label: &str,
) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, mut response) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), sense);
    if enabled && response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, enabled, *on, label));
    let t = ui.ctx().animate_bool_with_time(response.id, *on, 0.15);
    let track = Rect::from_min_size(pos2(rect.left(), rect.center().y - 10.0), vec2(34.0, 20.0));
    let fill = c.surface_active.lerp_to_gamma(c.accent, t);
    let fill = if enabled { fill } else { fill.gamma_multiply(0.5) };
    let ctx = ui.ctx().clone();
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        Weight::Regular.font_id(12.5),
        Color32::PLACEHOLDER,
    );
    let text_pos = pos2(
        snap(track.right() + 10.0, &ctx),
        snap(rect.center().y - galley.size().y / 2.0, &ctx),
    );
    let painter = ui.painter();
    painter.rect_filled(track, 10, fill);
    if response.has_focus() {
        painter.rect_stroke(track, 10, Stroke::new(2.0, c.accent_hover), StrokeKind::Outside);
    }
    let knob = pos2(track.left() + 10.0 + (track.width() - 20.0) * t, track.center().y);
    painter.circle_filled(knob, 7.0, Color32::WHITE);
    painter.galley(text_pos, galley, if enabled { c.text } else { c.dim });
    if enabled && response.hovered() {
        ctx.set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

pub fn summary_row(
    ui: &mut Ui,
    c: &Colors,
    icon: Icon,
    text: &str,
    detail: Option<&str>,
    strong: bool,
) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
    let ctx = ui.ctx().clone();
    icon.image(c.secondary, 14.0).paint_at(
        ui,
        Rect::from_center_size(pos2(rect.left() + 7.0, rect.center().y), Vec2::splat(14.0)),
    );
    let mut right = rect.right();
    if let Some(detail) = detail {
        let galley = ui.painter().layout_no_wrap(
            detail.to_owned(),
            Weight::Regular.font_id(11.5),
            Color32::PLACEHOLDER,
        );
        right -= galley.size().x;
        let pos = pos2(
            snap(right, &ctx),
            snap(rect.center().y - galley.size().y / 2.0, &ctx),
        );
        ui.painter().galley(pos, galley, c.dim);
        right -= 10.0;
    }
    let (font, color) = if strong {
        (Weight::Medium.font_id(12.5), c.text)
    } else {
        (Weight::Regular.font_id(12.0), c.secondary)
    };
    let mut job = LayoutJob::simple(
        text.to_owned(),
        font,
        color,
        (right - rect.left() - 24.0).max(20.0),
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.painter().layout_job(job);
    let pos = pos2(
        snap(rect.left() + 24.0, &ctx),
        snap(rect.center().y - galley.size().y / 2.0, &ctx),
    );
    ui.painter().galley(pos, galley, color);
}

pub fn path_field(ui: &mut Ui, c: &Colors, text: Option<&str>, placeholder: &str, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 28.0), Sense::hover());
    let ctx = ui.ctx().clone();
    let (shown, color) = match text {
        Some(text) => (text, c.text),
        None => (placeholder, c.dim),
    };
    let mut job = LayoutJob::simple(
        shown.to_owned(),
        Weight::Regular.font_id(12.0),
        color,
        rect.width() - 20.0,
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let painter = ui.painter();
    painter.rect_filled(rect, 6, c.window);
    painter.rect_stroke(rect, 6, Stroke::new(1.0, c.outline), StrokeKind::Inside);
    let galley = painter.layout_job(job);
    let pos = pos2(
        snap(rect.left() + 10.0, &ctx),
        snap(rect.center().y - galley.size().y / 2.0, &ctx),
    );
    painter.galley(pos, galley, color);
}

pub fn progress_bar(ui: &mut Ui, c: &Colors, fraction: f32) {
    let height = 6.0;
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let id = response.id;
    let shown = ui.ctx().animate_value_with_time(id, fraction.clamp(0.0, 1.0), 0.25);
    let painter = ui.painter();
    painter.rect_filled(rect, 3, c.surface_active);
    if shown > 0.0 {
        let width = (rect.width() * shown).max(height);
        let fill = Rect::from_min_size(rect.min, vec2(width, height));
        painter.rect_filled(fill, 3, c.accent);
    }
}

fn rounded_path(rect: Rect, r: f32) -> Vec<Pos2> {
    let corners = [
        (pos2(rect.min.x + r, rect.min.y + r), PI),
        (pos2(rect.max.x - r, rect.min.y + r), PI + FRAC_PI_2),
        (pos2(rect.max.x - r, rect.max.y - r), 0.0),
        (pos2(rect.min.x + r, rect.max.y - r), FRAC_PI_2),
    ];
    let steps = 6;
    let mut points = Vec::with_capacity(corners.len() * (steps + 1) + 1);
    for (center, start) in corners {
        for i in 0..=steps {
            let angle = start + FRAC_PI_2 * i as f32 / steps as f32;
            points.push(center + r * vec2(angle.cos(), angle.sin()));
        }
    }
    if let Some(first) = points.first().copied() {
        points.push(first);
    }
    points
}

pub fn dropzone(
    ui: &mut Ui,
    c: &Colors,
    active: bool,
    height: f32,
    title: &str,
    hint: &str,
) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, title));
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered() || active, 0.15);
    let fill = if active {
        c.panel.lerp_to_gamma(c.accent, 0.12)
    } else {
        c.panel.lerp_to_gamma(c.surface, t * 0.7)
    };
    let edge = c.outline.lerp_to_gamma(c.accent, if active { 1.0 } else { t });
    let ctx = ui.ctx().clone();

    let title_galley = ui.painter().layout_no_wrap(
        title.to_owned(),
        Weight::SemiBold.font_id(13.5),
        Color32::PLACEHOLDER,
    );
    let hint_galley = ui.painter().layout_no_wrap(
        hint.to_owned(),
        Weight::Regular.font_id(11.5),
        Color32::PLACEHOLDER,
    );
    let badge = 34.0;
    let block = badge + 8.0 + title_galley.size().y + 2.0 + hint_galley.size().y;
    let top = rect.center().y - block / 2.0;
    let badge_center = pos2(rect.center().x, top + badge / 2.0);
    let title_y = top + badge + 8.0;
    let hint_y = title_y + title_galley.size().y + 2.0;

    let painter = ui.painter();
    painter.rect_filled(rect, 10, fill);
    painter.extend(Shape::dashed_line(
        &rounded_path(rect.shrink(1.0), 10.0),
        Stroke::new(1.5, edge),
        5.0,
        4.0,
    ));
    if response.has_focus() {
        painter.rect_stroke(rect, 10, Stroke::new(2.0, c.accent_hover), StrokeKind::Outside);
    }
    painter.circle_filled(
        badge_center,
        badge / 2.0,
        c.surface.lerp_to_gamma(c.accent, if active { 0.9 } else { t * 0.25 }),
    );
    let icon_tint = if active { c.on_accent } else { c.accent.lerp_to_gamma(c.accent_hover, t) };
    Icon::FileArchive
        .image(icon_tint, 18.0)
        .paint_at(ui, Rect::from_center_size(badge_center, Vec2::splat(18.0)));

    let painter = ui.painter();
    let title_pos = pos2(
        snap(rect.center().x - title_galley.size().x / 2.0, &ctx),
        snap(title_y, &ctx),
    );
    painter.galley(title_pos, title_galley, c.text);
    let hint_pos = pos2(
        snap(rect.center().x - hint_galley.size().x / 2.0, &ctx),
        snap(hint_y, &ctx),
    );
    painter.galley(hint_pos, hint_galley, c.dim);

    ctx.set_cursor_icon(if response.hovered() {
        CursorIcon::PointingHand
    } else {
        CursorIcon::Default
    });
    response
}
