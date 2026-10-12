//! A retained runner must stop feeding input and updates to its hidden game.
use eframe::egui;

pub(crate) struct Screen {
    pub app: Box<dyn eframe::App>,
    pub active: bool,
    pub ctx: egui::Context,
}

impl Screen {
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        self.ctx.request_repaint();
    }
}

impl eframe::App for Screen {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if self.active {
            self.app.ui(ui, frame);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        // eframe 0.35 otherwise drops initial font uploads in background tabs.
        if let Some(viewport) = input.viewports.get_mut(&egui::ViewportId::ROOT) {
            viewport.occluded = Some(false);
        }
        if self.active {
            self.app.raw_input_hook(ctx, input);
        } else {
            input.events.clear();
            input.modifiers = egui::Modifiers::NONE;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::App;

    #[test]
    fn hidden_games_do_not_receive_keyboard_input_but_keep_font_uploads() {
        struct Blank;
        impl App for Blank {
            fn ui(&mut self, _: &mut egui::Ui, _: &mut eframe::Frame) {}
        }
        let ctx = egui::Context::default();
        let mut screen = Screen {
            app: Box::new(Blank),
            active: false,
            ctx: ctx.clone(),
        };
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .occluded = Some(true);
        input.events.push(egui::Event::Text("x".into()));
        screen.raw_input_hook(&ctx, &mut input);
        assert!(input.events.is_empty());
        assert_eq!(
            input.viewports[&egui::ViewportId::ROOT].occluded,
            Some(false)
        );
        screen.set_active(true);
        input.events.push(egui::Event::Text("x".into()));
        screen.raw_input_hook(&ctx, &mut input);
        assert_eq!(input.events.len(), 1);
    }
}
