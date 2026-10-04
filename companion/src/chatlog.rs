use crate::config::Config;
use crate::text::{has_chinese, parse_log_line};
use crate::worker::{Job, UiEvent, UiSender};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Follows WoWChatLog.txt like `tail -f` and queues every Chinese line.
pub fn spawn(config: Arc<Mutex<Config>>, jobs: Sender<Job>, ui: UiSender) {
    thread::spawn(move || {
        let mut open_path = String::new();
        let mut file: Option<File> = None;
        let mut pos = 0u64;
        let mut partial: Vec<u8> = Vec::new();
        let mut reported_missing = false;

        loop {
            let path = config.lock().unwrap().chat_log_path.clone();
            if path != open_path {
                open_path = path.clone();
                file = None;
                reported_missing = false;
            }

            if file.is_none() {
                match File::open(&path) {
                    Ok(mut f) => {
                        // Start at the end: only new chat gets translated.
                        pos = f.seek(SeekFrom::End(0)).unwrap_or(0);
                        partial.clear();
                        file = Some(f);
                        ui.send(UiEvent::Status(format!("Reading {path}")));
                    }
                    Err(_) => {
                        if !reported_missing {
                            ui.send(UiEvent::Status(format!(
                                "Waiting for the chat log at {path}. In game, type /tr log or /chatlog."
                            )));
                            reported_missing = true;
                        }
                    }
                }
            }

            if let Some(f) = file.as_mut() {
                let len = f.metadata().map(|m| m.len()).unwrap_or(pos);
                if len < pos {
                    // The client started a fresh log.
                    pos = 0;
                    partial.clear();
                }
                if len > pos && f.seek(SeekFrom::Start(pos)).is_ok() {
                    let mut chunk = Vec::new();
                    if let Ok(n) = f.take(len - pos).read_to_end(&mut chunk) {
                        pos += n as u64;
                        partial.extend_from_slice(&chunk);
                        let lines = partial.iter().filter(|&&b| b == b'\n').count();
                        ui.send(UiEvent::LogGrew { lines });
                        while let Some(nl) = partial.iter().position(|&b| b == b'\n') {
                            let raw: Vec<u8> = partial.drain(..=nl).collect();
                            let line = String::from_utf8_lossy(&raw);
                            if let Some(parsed) = parse_log_line(&line) {
                                if has_chinese(&parsed.message) {
                                    let _ = jobs.send(Job::Incoming {
                                        speaker: parsed.speaker,
                                        text: parsed.message,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(250));
        }
    });
}
