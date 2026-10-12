use eframe::egui::{self, Color32, FontId, Stroke, Vec2};

pub const BOARD: Color32 = Color32::from_rgb(16, 65, 55);
pub const CHROME: Color32 = Color32::from_rgb(17, 42, 37);
pub const CREAM: Color32 = Color32::from_rgb(255, 250, 239);
pub const GOLD: Color32 = Color32::from_rgb(225, 196, 126);
pub const MUTED: Color32 = Color32::from_rgb(151, 183, 167);

pub fn setup(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = CHROME;
    visuals.window_fill = CHROME;
    visuals.override_text_color = Some(CREAM);
    visuals.selection.bg_fill = Color32::from_rgb(54, 97, 76);
    visuals.selection.stroke = Stroke::new(1.0, GOLD);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(35, 68, 56);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(51, 91, 72);
    visuals.widgets.active.bg_fill = Color32::from_rgb(73, 108, 77);
    ctx.set_visuals(visuals);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(14.0, 9.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
    });
}

pub fn parse_seed(value: &str) -> Result<Option<u64>, &'static str> {
    let value = value.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        value
            .parse()
            .map(Some)
            .map_err(|_| "Enter an integer between 0 and 18446744073709551615.")
    }
}
