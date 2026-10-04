use crate::config::{self, Config};
use crate::worker::{Job, UiEvent};
use eframe::egui::{self, Color32, RichText, ViewportBuilder, ViewportCommand, ViewportId};
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct OverlayLine {
    speaker: String,
    original: String,
    english: String,
}

pub struct TranslateApp {
    config: Arc<Mutex<Config>>,
    draft: Config,
    events: Receiver<UiEvent>,
    interactive: Sender<Job>,
    lines: VecDeque<OverlayLine>,
    status: String,
    notice: String,
    reply: String,
    locked: bool,
    overlay_start: egui::Pos2,
    overlay_rect: Option<egui::Rect>,
    log_last_grew: Option<Instant>,
    log_lines: usize,
    test_result: String,
}

impl TranslateApp {
    pub fn new(config: Arc<Mutex<Config>>, events: Receiver<UiEvent>, interactive: Sender<Job>) -> Self {
        let draft = config.lock().unwrap().clone();
        let overlay_start = egui::pos2(draft.overlay_x, draft.overlay_y);
        Self {
            config,
            draft,
            events,
            interactive,
            lines: VecDeque::new(),
            status: "Starting".into(),
            notice: String::new(),
            reply: String::new(),
            locked: true,
            overlay_start,
            overlay_rect: None,
            log_last_grew: None,
            log_lines: 0,
            test_result: String::new(),
        }
    }

    fn drain_events(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                UiEvent::Line { speaker, original, english } => {
                    self.lines.push_back(OverlayLine { speaker, original, english });
                    while self.lines.len() > self.draft.max_lines.max(1) {
                        self.lines.pop_front();
                    }
                }
                UiEvent::PasteReady { id, english } => {
                    self.notice = format!("#{id} ready: press Ctrl+V in the addon window");
                    self.lines.push_back(OverlayLine {
                        speaker: format!("#{id}"),
                        original: String::new(),
                        english,
                    });
                }
                UiEvent::ReplyReady { english, chinese } => {
                    self.notice = format!("Copied \"{chinese}\" for \"{english}\". Paste it into WoW chat.");
                }
                UiEvent::TestResult { result, seconds } => {
                    self.test_result = match result {
                        Ok(english) if crate::text::has_chinese(&english) => {
                            format!("The model answered but did not translate ({seconds:.1} s): {english}")
                        }
                        Ok(english) => format!("Model works ({seconds:.1} s): {} -> {english}", crate::worker::TEST_LINE),
                        Err(e) => format!("Model test failed: {e}"),
                    };
                }
                UiEvent::LogGrew { lines } => {
                    self.log_last_grew = Some(Instant::now());
                    self.log_lines += lines;
                }
                UiEvent::Status(text) => self.status = text,
            }
        }
    }

    fn save(&mut self) {
        if let Some(rect) = self.overlay_rect {
            self.draft.overlay_x = rect.min.x;
            self.draft.overlay_y = rect.min.y;
        }
        *self.config.lock().unwrap() = self.draft.clone();
        self.notice = match config::save(&self.draft) {
            Ok(()) => format!("Saved {}", config::path().display()),
            Err(e) => format!("Could not save settings: {e}"),
        };
    }

    fn control_panel(&mut self, root: &mut egui::Ui) {
        egui::CentralPanel::default().show(root, |ui| {
            ui.heading("Translate companion");
            ui.label(&self.status);
            if !self.notice.is_empty() {
                ui.colored_label(Color32::from_rgb(120, 200, 255), &self.notice);
            }
            let activity = match self.log_last_grew {
                Some(t) => format!(
                    "Chat log last written {} s ago, {} lines read since start",
                    t.elapsed().as_secs(),
                    self.log_lines
                ),
                None => "Chat log has not been written since the companion started".into(),
            };
            ui.label(activity);
            ui.horizontal(|ui| {
                if ui.button("Test model").clicked() {
                    // Test what is typed in Settings, even before Save.
                    *self.config.lock().unwrap() = self.draft.clone();
                    let _ = self.interactive.send(Job::Test);
                    self.test_result = format!("Testing {} at {}", self.draft.model, self.draft.api_base);
                }
                if !self.test_result.is_empty() {
                    ui.label(&self.test_result);
                }
            });
            ui.separator();

            ui.label("Reply in Chinese (Enter copies the translation for pasting into WoW chat):");
            let edit = ui.add(egui::TextEdit::singleline(&mut self.reply).desired_width(f32::INFINITY));
            if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.reply.trim().is_empty() {
                let _ = self.interactive.send(Job::Reply { text: self.reply.trim().to_string() });
                self.notice = "Translating your reply".into();
                self.reply.clear();
                edit.request_focus();
            }
            ui.separator();

            ui.horizontal(|ui| {
                let label = if self.locked { "Unlock overlay to move it" } else { "Lock overlay" };
                if ui.button(label).clicked() {
                    self.locked = !self.locked;
                    if self.locked {
                        self.save();
                    }
                }
                if ui.button("Clear overlay").clicked() {
                    self.lines.clear();
                }
                ui.checkbox(&mut self.draft.show_original, "Show Chinese too");
            });

            egui::CollapsingHeader::new("Settings").default_open(false).show(ui, |ui| {
                egui::Grid::new("settings").num_columns(2).show(ui, |ui| {
                    ui.label("Chat log file");
                    ui.add(egui::TextEdit::singleline(&mut self.draft.chat_log_path).desired_width(360.0));
                    ui.end_row();
                    ui.label("API base URL");
                    ui.text_edit_singleline(&mut self.draft.api_base);
                    ui.end_row();
                    ui.label("API key");
                    ui.add(egui::TextEdit::singleline(&mut self.draft.api_key).password(true));
                    ui.end_row();
                    ui.label("Model");
                    ui.text_edit_singleline(&mut self.draft.model);
                    ui.end_row();
                    ui.label("Overlay width");
                    ui.add(egui::Slider::new(&mut self.draft.overlay_width, 200.0..=1600.0));
                    ui.end_row();
                    ui.label("Overlay height");
                    ui.add(egui::Slider::new(&mut self.draft.overlay_height, 80.0..=1000.0));
                    ui.end_row();
                    ui.label("Background opacity");
                    ui.add(egui::Slider::new(&mut self.draft.overlay_opacity, 0.0..=1.0));
                    ui.end_row();
                    ui.label("Text size");
                    ui.add(egui::Slider::new(&mut self.draft.font_size, 10.0..=32.0));
                    ui.end_row();
                    ui.label("Lines kept");
                    ui.add(egui::Slider::new(&mut self.draft.max_lines, 1..=50));
                    ui.end_row();
                });
                if ui.button("Save settings").clicked() {
                    self.save();
                }
            });
        });
    }

    fn overlay(&mut self, ctx: &egui::Context) {
        let builder = ViewportBuilder::default()
            .with_title("Translate overlay")
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_taskbar(false)
            .with_mouse_passthrough(self.locked)
            .with_position(self.overlay_start)
            .with_inner_size([self.draft.overlay_width, self.draft.overlay_height]);

        let locked = self.locked;
        let draft = self.draft.clone();
        let lines = &self.lines;
        let mut rect = self.overlay_rect;

        ctx.show_viewport_immediate(ViewportId::from_hash_of("overlay"), builder, |root, _class| {
            let ctx = root.ctx().clone();
            rect = ctx.input(|i| i.viewport().outer_rect).or(rect);
            let alpha = (draft.overlay_opacity * 255.0) as u8;
            let fill = if locked {
                Color32::from_black_alpha(alpha)
            } else {
                Color32::from_rgba_unmultiplied(20, 60, 110, alpha.max(140))
            };
            let frame = egui::Frame::NONE.fill(fill).inner_margin(8.0);
            egui::CentralPanel::default().frame(frame).show(root, |ui| {
                if !locked {
                    let drag = ui.interact(ui.max_rect(), ui.id().with("drag"), egui::Sense::drag());
                    if drag.drag_started() {
                        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                    }
                    ui.label(RichText::new("Drag to move. Lock it from the companion window.").color(Color32::WHITE));
                }
                egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                    for line in lines {
                        let mut text = RichText::new(&line.english).size(draft.font_size).color(Color32::WHITE);
                        if line.speaker.is_empty() {
                            text = text.italics();
                        }
                        ui.horizontal_wrapped(|ui| {
                            if !line.speaker.is_empty() {
                                ui.label(
                                    RichText::new(format!("{}:", line.speaker))
                                        .size(draft.font_size)
                                        .color(Color32::from_rgb(255, 210, 120)),
                                );
                            }
                            ui.label(text);
                        });
                        if draft.show_original && !line.original.is_empty() {
                            ui.label(
                                RichText::new(&line.original)
                                    .size(draft.font_size * 0.85)
                                    .color(Color32::from_gray(170)),
                            );
                        }
                    }
                });
            });
        });
        self.overlay_rect = rect;
    }
}

impl eframe::App for TranslateApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Keeps the "last written N s ago" counter ticking.
        ctx.request_repaint_after(Duration::from_secs(1));
        self.drain_events();
        self.control_panel(ui);
        self.overlay(&ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}
