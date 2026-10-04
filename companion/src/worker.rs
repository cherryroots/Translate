use crate::config::Config;
use crate::translate::{translate, Direction};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub enum Job {
    /// A Chinese line read from the chat log.
    Incoming { speaker: String, text: String },
    /// A line copied from the addon window as "TR#<id> <text>".
    Tagged { id: u64, text: String },
    /// English the player typed in the companion, to send as Chinese.
    Reply { text: String },
}

pub enum UiEvent {
    Line { speaker: String, original: String, english: String },
    PasteReady { id: u64, english: String },
    ReplyReady { english: String, chinese: String },
    Status(String),
}

/// Sends to the UI and wakes it so the event shows without mouse movement.
#[derive(Clone)]
pub struct UiSender {
    tx: Sender<UiEvent>,
    ctx: eframe::egui::Context,
}

impl UiSender {
    pub fn new(tx: Sender<UiEvent>, ctx: eframe::egui::Context) -> Self {
        Self { tx, ctx }
    }

    pub fn send(&self, event: UiEvent) {
        let _ = self.tx.send(event);
        self.ctx.request_repaint();
    }
}

/// One thread per queue keeps the chat log from delaying click-to-translate.
pub fn spawn(config: Arc<Mutex<Config>>, jobs: Receiver<Job>, ui: UiSender, clipboard: Sender<String>) {
    thread::spawn(move || {
        let mut cache: HashMap<String, String> = HashMap::new();
        for job in jobs {
            let config = config.lock().unwrap().clone();
            let (text, direction) = match &job {
                Job::Incoming { text, .. } | Job::Tagged { text, .. } => (text.clone(), Direction::ToEnglish),
                Job::Reply { text } => (text.clone(), Direction::ToChinese),
            };
            let key = format!("{}|{text}", matches!(direction, Direction::ToEnglish));
            let result = match cache.get(&key) {
                Some(hit) => Ok(hit.clone()),
                None => translate(&config, &text, direction),
            };
            let translated = match result {
                Ok(t) => {
                    if cache.len() > 2000 {
                        cache.clear();
                    }
                    cache.insert(key, t.clone());
                    t
                }
                Err(e) => {
                    ui.send(UiEvent::Status(format!("Translation failed: {e}")));
                    continue;
                }
            };
            match job {
                Job::Incoming { speaker, text } => {
                    ui.send(UiEvent::Line { speaker, original: text, english: translated })
                }
                Job::Tagged { id, .. } => {
                    let _ = clipboard.send(format!("TR#{id} {translated}"));
                    ui.send(UiEvent::PasteReady { id, english: translated });
                }
                Job::Reply { text } => {
                    let _ = clipboard.send(translated.clone());
                    ui.send(UiEvent::ReplyReady { english: text, chinese: translated });
                }
            }
        }
    });
}
