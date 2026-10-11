//! Browser lifecycle only: start, stop, and mirror the current game in the URL.

use wasm_bindgen::prelude::*;

use crate::{GameKind, GamesApp};

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
                    let initial = GameKind::from_slug(&cc.integration_info.web_info.location.hash)
                        .unwrap_or(GameKind::Klondike);
                    Ok(Box::new(GamesApp::new(cc, initial, Box::new(show_in_url))))
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

/// `replaceState` keeps reloads and shared links on the same game without
/// adding history entries or firing `hashchange`.
fn show_in_url(kind: GameKind) {
    if let Some(history) = web_sys::window().and_then(|window| window.history().ok()) {
        let _ =
            history.replace_state_with_url(&JsValue::NULL, "", Some(&format!("#{}", kind.slug())));
    }
}
