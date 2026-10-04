use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Full path to WoWChatLog.txt inside the client's Logs folder.
    pub chat_log_path: String,
    /// Any OpenAI-compatible base URL, ending before /chat/completions.
    pub api_base: String,
    /// Left empty for local servers such as Ollama.
    pub api_key: String,
    pub model: String,
    pub overlay_x: f32,
    pub overlay_y: f32,
    pub overlay_width: f32,
    pub overlay_height: f32,
    pub overlay_opacity: f32,
    pub font_size: f32,
    pub max_lines: usize,
    pub show_original: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            chat_log_path: r"C:\Program Files (x86)\World of Warcraft\_classic_beta_\Logs\WoWChatLog.txt"
                .into(),
            api_base: "http://localhost:11434/v1".into(),
            api_key: String::new(),
            model: "hf.co/tencent/HY-MT1.5-7B-GGUF:Q4_K_M".into(),
            overlay_x: 20.0,
            overlay_y: 420.0,
            overlay_width: 520.0,
            overlay_height: 260.0,
            overlay_opacity: 0.55,
            font_size: 15.0,
            max_lines: 12,
            show_original: false,
        }
    }
}

/// config.json lives next to the executable so the app stays portable.
pub fn path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("config.json")))
        .unwrap_or_else(|| PathBuf::from("config.json"))
}

pub fn load() -> Config {
    match std::fs::read_to_string(path()) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => {
            let config = Config::default();
            let _ = save(&config);
            config
        }
    }
}

pub fn save(config: &Config) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(config).expect("config serializes");
    std::fs::write(path(), text)
}
