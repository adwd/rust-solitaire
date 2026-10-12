//! Browser lifecycle for one lazily loaded game.

use wasm_bindgen::prelude::*;

use crate::{GameKind, screen::Screen};

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

    pub async fn start(
        &self,
        canvas: web_sys::HtmlCanvasElement,
        slug: &str,
    ) -> Result<(), JsValue> {
        let kind = GameKind::from_slug(slug).ok_or_else(|| JsValue::from_str("Unknown game"))?;
        self.runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    let app = kind.create(cc).ok_or("Game not included in this module")?;
                    Ok(Box::new(Screen {
                        app,
                        active: false,
                        ctx: cc.egui_ctx.clone(),
                    }))
                }),
            )
            .await
    }

    pub fn set_active(&self, active: bool) {
        if let Some(mut screen) = self.runner.app_mut::<Screen>() {
            screen.set_active(active);
        }
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
