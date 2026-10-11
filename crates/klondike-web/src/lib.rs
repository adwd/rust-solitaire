//! Browser lifecycle only; all game rules, drawing, and input live in shared crates.
#![cfg(target_arch = "wasm32")]

use klondike_core::Rules;
use klondike_egui::{KlondikeApp, random_seed};
use wasm_bindgen::prelude::*;

/// Kept alive by the page until it is closed or reloaded.
#[wasm_bindgen]
pub struct WebGame {
    runner: eframe::WebRunner,
}

#[wasm_bindgen]
impl WebGame {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            runner: eframe::WebRunner::new(),
        }
    }

    pub async fn start(&self, canvas: web_sys::HtmlCanvasElement) -> Result<(), JsValue> {
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| {
                    Ok(Box::new(WebApp(KlondikeApp::new(
                        cc,
                        random_seed(),
                        Rules::default(),
                    ))))
                }),
            )
            .await
    }

    pub fn destroy(&self) {
        self.runner.destroy();
    }

    pub fn panic_message(&self) -> Option<String> {
        self.runner.panic_summary().map(|summary| summary.message())
    }
}

impl Default for WebGame {
    fn default() -> Self {
        Self::new()
    }
}

struct WebApp(KlondikeApp);

impl eframe::App for WebApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        self.0.ui(ui, frame);
    }

    /// eframe 0.35 drops texture uploads from frames where the page is hidden.
    /// A page opened in a background tab then loses the initial font atlas and
    /// panics on its next partial update, so treat a hidden page as visible.
    fn raw_input_hook(
        &mut self,
        _ctx: &eframe::egui::Context,
        raw_input: &mut eframe::egui::RawInput,
    ) {
        if let Some(viewport) = raw_input.viewports.get_mut(&eframe::egui::ViewportId::ROOT) {
            viewport.occluded = Some(false);
        }
    }
}
