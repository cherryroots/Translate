#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod chatlog;
mod clipboard;
mod config;
mod text;
mod translate;
mod worker;

use eframe::egui;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// egui ships without Chinese glyphs; borrow a system font if one exists.
fn add_cjk_font(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyh.ttf",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simsun.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    ];
    let Some(bytes) = CANDIDATES.iter().find_map(|p| std::fs::read(p).ok()) else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("cjk".into(), Arc::new(egui::FontData::from_owned(bytes)));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push("cjk".into());
    }
    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result {
    let config = Arc::new(Mutex::new(config::load()));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Translate companion")
            .with_inner_size([560.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Translate companion",
        options,
        Box::new(move |cc| {
            add_cjk_font(&cc.egui_ctx);
            let (ui_tx, ui_rx) = mpsc::channel();
            let ui = worker::UiSender::new(ui_tx, cc.egui_ctx.clone());
            let (clip_tx, clip_rx) = mpsc::channel();
            let (log_tx, log_rx) = mpsc::channel();
            let (interactive_tx, interactive_rx) = mpsc::channel();

            worker::spawn(config.clone(), log_rx, ui.clone(), clip_tx.clone());
            worker::spawn(config.clone(), interactive_rx, ui.clone(), clip_tx);
            chatlog::spawn(config.clone(), log_tx, ui.clone());
            clipboard::spawn(clip_rx, interactive_tx.clone(), ui);

            Ok(Box::new(app::TranslateApp::new(config, ui_rx, interactive_tx)))
        }),
    )
}
