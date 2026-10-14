use eframe::egui::{
    self, Color32, CornerRadius, Shadow, Stroke, Theme, ThemePreference, Visuals, vec2,
};
use fastframe_text::TextRendering;
use fastframe_theme::Base;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    pub dark: bool,
    pub window: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub surface_active: Color32,
    pub outline: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub dim: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub overlay: Color32,
    pub shadow: Color32,
    pub success: Color32,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

impl Colors {
    pub const DARK: Self = Self {
        dark: true,
        window: rgb(0x12141a),
        panel: rgb(0x1b1e26),
        surface: rgb(0x262a34),
        surface_hover: rgb(0x303542),
        surface_active: rgb(0x3a4050),
        outline: rgb(0x2e3340),
        text: rgb(0xe8eaf0),
        secondary: rgb(0xa1a8b8),
        dim: rgb(0x7a8194),
        accent: rgb(0x0070d1),
        accent_hover: rgb(0x2b8ae6),
        on_accent: rgb(0xffffff),
        danger: rgb(0xeb6464),
        warning: rgb(0xe6b450),
        overlay: Color32::from_black_alpha(170),
        shadow: Color32::from_black_alpha(110),
        success: rgb(0x4cc476),
    };

    pub const LIGHT: Self = Self {
        dark: false,
        window: rgb(0xf3f4f7),
        panel: rgb(0xffffff),
        surface: rgb(0xe9ebf0),
        surface_hover: rgb(0xdfe2e9),
        surface_active: rgb(0xd3d7e0),
        outline: rgb(0xd5d9e2),
        text: rgb(0x1b1f2a),
        secondary: rgb(0x50586a),
        dim: rgb(0x7b8296),
        accent: rgb(0x0070d1),
        accent_hover: rgb(0x005bb0),
        on_accent: rgb(0xffffff),
        danger: rgb(0xc93c3c),
        warning: rgb(0xb7791f),
        overlay: Color32::from_black_alpha(110),
        shadow: Color32::from_black_alpha(50),
        success: rgb(0x1f9d55),
    };
}

impl fastframe_theme::Palette for Colors {
    fn base(base: Base) -> Self {
        match base {
            Base::Dark => Self::DARK,
            Base::Light => Self::LIGHT,
        }
    }

    fn set(&mut self, name: &str, color: Color32) -> bool {
        match name {
            "window" => self.window = color,
            "panel" => self.panel = color,
            "surface" => self.surface = color,
            "surface_hover" => self.surface_hover = color,
            "surface_active" => self.surface_active = color,
            "outline" => self.outline = color,
            "text" => self.text = color,
            "secondary" => self.secondary = color,
            "dim" => self.dim = color,
            "accent" => self.accent = color,
            "accent_hover" => self.accent_hover = color,
            "on_accent" => self.on_accent = color,
            "danger" => self.danger = color,
            "warning" => self.warning = color,
            "overlay" => self.overlay = color,
            "shadow" => self.shadow = color,
            "success" => self.success = color,
            _ => return false,
        }
        true
    }
}

fn visuals(c: &Colors) -> Visuals {
    let mut v = if c.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.panel_fill = c.window;
    v.window_fill = c.panel;
    v.extreme_bg_color = c.window;
    v.faint_bg_color = c.surface;
    v.code_bg_color = c.surface;
    v.hyperlink_color = c.accent;
    v.warn_fg_color = c.warning;
    v.error_fg_color = c.danger;
    v.window_stroke = Stroke::new(1.0, c.outline);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    let shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: c.shadow,
    };
    v.window_shadow = shadow;
    v.popup_shadow = shadow;
    v.selection.bg_fill = c.accent;
    v.selection.stroke = Stroke::new(1.0, c.on_accent);

    let radius = CornerRadius::same(6);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = c.panel;
    w.noninteractive.weak_bg_fill = c.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, c.outline);
    w.noninteractive.fg_stroke = Stroke::new(1.0, c.secondary);

    w.inactive.bg_fill = c.surface;
    w.inactive.weak_bg_fill = c.surface;
    w.inactive.bg_stroke = Stroke::new(1.0, c.outline);
    w.inactive.fg_stroke = Stroke::new(1.0, c.text);

    w.hovered.bg_fill = c.surface_hover;
    w.hovered.weak_bg_fill = c.surface_hover;
    w.hovered.bg_stroke = Stroke::new(1.0, c.accent);
    w.hovered.fg_stroke = Stroke::new(1.0, c.text);
    w.hovered.expansion = 0.0;

    w.active.bg_fill = c.surface_active;
    w.active.weak_bg_fill = c.surface_active;
    w.active.bg_stroke = Stroke::new(1.0, c.accent_hover);
    w.active.fg_stroke = Stroke::new(1.0, c.text);
    w.active.expansion = 0.0;

    w.open.bg_fill = c.surface_hover;
    w.open.weak_bg_fill = c.surface_hover;
    w.open.fg_stroke = Stroke::new(1.0, c.text);

    for widget in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        widget.corner_radius = radius;
    }
    v
}

pub fn apply(ctx: &egui::Context, colors: &Colors, rendering: &TextRendering) {
    let v = visuals(colors);
    ctx.set_theme(if colors.dark {
        ThemePreference::Dark
    } else {
        ThemePreference::Light
    });
    ctx.set_visuals_of(Theme::Dark, v.clone());
    ctx.set_visuals_of(Theme::Light, v);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = vec2(6.0, 6.0);
        style.spacing.button_padding = vec2(10.0, 5.0);
        style.spacing.interact_size.y = 28.0;
        for (text_style, size) in [
            (egui::TextStyle::Body, 13.0),
            (egui::TextStyle::Button, 13.0),
            (egui::TextStyle::Small, 11.0),
            (egui::TextStyle::Heading, 17.0),
        ] {
            if let Some(font) = style.text_styles.get_mut(&text_style) {
                font.size = size;
            }
        }
        rendering.apply_to_visuals(&mut style.visuals);
    });
}
