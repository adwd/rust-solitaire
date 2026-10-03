mod app;
mod board;
mod input;
mod theme;

use eframe::egui;
use solitaire_core::{DrawMode, Rules};

fn main() -> eframe::Result {
    let mut seed = None;
    let mut draw = DrawMode::One;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                seed = Some(
                    args.next()
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or_else(|| invalid("--seed requires an unsigned 64-bit integer")),
                )
            }
            "--draw" => {
                draw = match args.next().as_deref() {
                    Some("1") => DrawMode::One,
                    Some("3") => DrawMode::Three,
                    _ => invalid("--draw must be 1 or 3"),
                }
            }
            "--help" | "-h" => {
                println!("rust-solitaire [--seed <u64>] [--draw 1|3]");
                return Ok(());
            }
            _ => invalid(&format!("unknown argument: {arg}")),
        }
    }
    let seed = seed.unwrap_or_else(rand::random);
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("Rust Solitaire")
            .with_app_id("adwd.rust-solitaire")
            .with_inner_size([1100.0, 800.0])
            .with_min_inner_size([760.0, 540.0]),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Rust Solitaire",
        options,
        Box::new(move |cc| Ok(Box::new(app::SolitaireApp::new(cc, seed, Rules { draw })))),
    )
}

fn invalid(message: &str) -> ! {
    eprintln!("{message}\nUsage: rust-solitaire [--seed <u64>] [--draw 1|3]");
    std::process::exit(2)
}
