use crate::gui::icons::Icon;
use crate::gui::palette::Colors;
use eframe::egui::{
    Color32, CursorIcon, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
    WidgetType, pos2, vec2,
};
use fastframe_fonts::Weight;

pub fn icon_button(ui: &mut Ui, c: &Colors, icon: Icon, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, tooltip));
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.12);
    let painter = ui.painter();
    painter.rect_filled(rect, 6, c.surface.gamma_multiply(t));
    painter.rect_stroke(
        rect,
        6,
        Stroke::new(1.0, c.outline.lerp_to_gamma(c.accent, t)),
        StrokeKind::Inside,
    );
    if response.has_focus() {
        painter.rect_stroke(rect, 6, Stroke::new(2.0, c.accent_hover), StrokeKind::Outside);
    }
    icon.image(c.secondary.lerp_to_gamma(c.text, t), 15.0)
        .paint_at(ui, Rect::from_center_size(rect.center(), Vec2::splat(15.0)));
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response.on_hover_text(tooltip)
}

pub fn menu_item(
    ui: &mut Ui,
    c: &Colors,
    label: &str,
    selected: bool,
    swatch: Option<Color32>,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width().max(190.0), 28.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, label));
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.1);
    let weight = if selected { Weight::Medium } else { Weight::Regular };
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        weight.font_id(12.5),
        Color32::PLACEHOLDER,
    );
    let ctx = ui.ctx().clone();
    let painter = ui.painter();
    painter.rect_filled(rect, 6, c.surface_hover.gamma_multiply(t));
    if response.has_focus() {
        painter.rect_stroke(rect, 6, Stroke::new(2.0, c.accent_hover), StrokeKind::Inside);
    }
    let mut x = rect.left() + 10.0;
    if let Some(swatch) = swatch {
        let center = pos2(x + 6.0, rect.center().y);
        painter.circle_filled(center, 6.0, swatch);
        painter.circle_stroke(center, 6.0, Stroke::new(1.0, c.outline));
        x += 20.0;
    }
    let pos = pos2(
        fastframe_text::snap_to_pixels(x, ctx.pixels_per_point()),
        fastframe_text::snap_to_pixels(
            rect.center().y - galley.size().y / 2.0,
            ctx.pixels_per_point(),
        ),
    );
    painter.galley(pos, galley, c.text);
    if selected {
        Icon::Check.image(c.accent, 14.0).paint_at(
            ui,
            Rect::from_center_size(pos2(rect.right() - 16.0, rect.center().y), Vec2::splat(14.0)),
        );
    }
    if response.hovered() {
        ctx.set_cursor_icon(CursorIcon::PointingHand);
    }
    response.clicked()
}
