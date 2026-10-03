//! Browser lifecycle only; all game rules, drawing, and input live in shared crates.
#![cfg(target_arch = "wasm32")]

use solitaire_core::Rules;
use solitaire_egui::{SolitaireApp, random_seed};
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
                    Ok(Box::new(SolitaireApp::new(
                        cc,
                        random_seed(),
                        Rules::default(),
                    )))
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
