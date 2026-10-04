use crate::config::Config;
use serde_json::{json, Value};
use std::time::Duration;

const TO_ENGLISH: &str = "You translate World of Warcraft in-game chat from Chinese into natural, casual English, the way an English-speaking player would write it. \
Keep player names, numbers, item names in brackets, and English words as they are. \
Know the slang: 奶/奶妈/治疗 = healer, T/坦/坦克 = tank, 输出/DPS = dps, 来人/缺人 = LF more, 组/组队 = LFG, 车/车队/开车 = group run, \
金团 = GDKP (gold run), 老板 = buyer, 打工 = booster, 拍卖 = auction, 1 = ready/yes, 2 = no, 散人 = pug, 工会/公会 = guild, 副本 = dungeon/raid. \
Reply with the translation only: no quotes, notes or romanization.";

const TO_CHINESE: &str = "Translate the user's World of Warcraft chat message into casual Simplified Chinese, the way a Chinese player would type it in game chat. \
Keep player names, numbers and item names as they are. Reply with the translation only.";

#[derive(Clone, Copy)]
pub enum Direction {
    ToEnglish,
    ToChinese,
}

/// WoW slang the general-purpose prompt explains and HY-MT receives as
/// terminology hints, for the terms that appear in the line.
const SLANG: &[(&str, &str)] = &[
    ("奶妈", "healer"),
    ("缺奶", "need a healer"),
    ("治疗", "healer"),
    ("坦克", "tank"),
    ("缺T", "need a tank"),
    ("输出", "DPS"),
    ("来人", "LF more"),
    ("缺人", "LF more"),
    ("组队", "LFG"),
    ("车队", "group run"),
    ("开车", "starting the run"),
    ("金团", "GDKP run"),
    ("老板", "buyer"),
    ("打工", "booster"),
    ("散人", "pug"),
    ("公会", "guild"),
    ("工会", "guild"),
    ("副本", "dungeon"),
];

/// Tencent's HY-MT models want their own prompt and no system message.
fn is_hy_mt(model: &str) -> bool {
    let m = model.to_lowercase();
    m.contains("hy-mt") || m.contains("hunyuan-mt")
}

/// Builds the request body for either prompt style.
fn request_body(model: &str, text: &str, direction: Direction) -> Value {
    if !is_hy_mt(model) {
        let system = match direction {
            Direction::ToEnglish => TO_ENGLISH,
            Direction::ToChinese => TO_CHINESE,
        };
        return json!({
            "model": model,
            "temperature": 0.2,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": text },
            ],
        });
    }

    // Templates and sampling settings from the HY-MT1.5 model card.
    let target = match direction {
        Direction::ToEnglish => "英语",
        Direction::ToChinese => "中文",
    };
    let terms: Vec<String> = match direction {
        Direction::ToEnglish => SLANG
            .iter()
            .filter(|(zh, _)| text.contains(zh))
            .map(|(zh, en)| format!("{zh} 翻译成 {en}"))
            .collect(),
        Direction::ToChinese => Vec::new(),
    };
    let prompt = if terms.is_empty() {
        format!("将以下文本翻译为{target}，注意只需要输出翻译后的结果，不要额外解释：\n\n{text}")
    } else {
        format!(
            "参考下面的翻译：\n{}\n\n将以下文本翻译为{target}，注意只需要输出翻译后的结果，不要额外解释：\n{text}",
            terms.join("\n")
        )
    };
    json!({
        "model": model,
        "temperature": 0.7,
        "top_p": 0.6,
        // Read by llama.cpp and Ollama; other servers ignore them.
        "top_k": 20,
        "repeat_penalty": 1.05,
        "messages": [{ "role": "user", "content": prompt }],
    })
}

/// Local servers (LM Studio, Ollama) serve the API under /v1; a bare
/// `http://host:port` gets it added so a missing /v1 is not a silent failure.
fn completions_url(base: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    let after_scheme = base.split_once("://").map_or(base, |(_, rest)| rest);
    if after_scheme.contains('/') {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

/// Sends one message to an OpenAI-compatible /chat/completions endpoint.
pub fn translate(config: &Config, text: &str, direction: Direction) -> Result<String, String> {
    let url = completions_url(&config.api_base);
    let body = request_body(&config.model, text, direction);

    let mut request = ureq::post(&url).timeout(Duration::from_secs(60));
    if !config.api_key.is_empty() {
        request = request.set("Authorization", &format!("Bearer {}", config.api_key));
    }
    let response: Value = match request.send_json(body) {
        Ok(r) => r.into_json().map_err(|e| format!("bad JSON from {url}: {e}"))?,
        Err(ureq::Error::Status(code, r)) => {
            let detail = r.into_string().unwrap_or_default();
            return Err(format!("{url} returned {code}: {}", detail.chars().take(200).collect::<String>()));
        }
        Err(e) => return Err(format!("could not reach {url}: {e}")),
    };

    response["choices"][0]["message"]["content"]
        .as_str()
        .map(clean_reply)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            let detail = response.to_string();
            format!("{url} gave no translation: {}", detail.chars().take(200).collect::<String>())
        })
}

/// Models sometimes wrap the answer in quotes or add a thinking block; drop both.
fn clean_reply(raw: &str) -> String {
    let text = match raw.rfind("</think>") {
        Some(end) => &raw[end + "</think>".len()..],
        None => raw,
    };
    let text = text.trim();
    let text = text
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(text);
    // WoW chat is single-line.
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn cleans_reply() {
        assert_eq!(clean_reply("\"Need a healer\"\n"), "Need a healer");
        assert_eq!(clean_reply("<think>hmm</think>\nLF tank"), "LF tank");
    }

    #[test]
    fn calls_openai_compatible_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let body = r#"{"choices":[{"message":{"role":"assistant","content":"LF healer for the dungeon"}}]}"#;
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).unwrap();
            request
        });
        let config = Config {
            api_base: format!("http://127.0.0.1:{port}/v1/"),
            api_key: "secret".into(),
            ..Config::default()
        };
        let english = translate(&config, "副本缺奶", Direction::ToEnglish).unwrap();
        assert_eq!(english, "LF healer for the dungeon");
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /v1/chat/completions"));
        assert!(request.contains("Bearer secret"));
    }

    #[test]
    fn adds_v1_to_bare_host() {
        assert_eq!(completions_url("http://localhost:1234"), "http://localhost:1234/v1/chat/completions");
        assert_eq!(completions_url("http://localhost:1234/"), "http://localhost:1234/v1/chat/completions");
        assert_eq!(completions_url("http://localhost:1234/v1/"), "http://localhost:1234/v1/chat/completions");
        assert_eq!(completions_url("https://api.openai.com/v1"), "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn hy_mt_uses_model_card_prompt_and_glossary() {
        let body = request_body("hf.co/tencent/HY-MT1.5-7B-GGUF:Q4_K_M", "金团来人 缺奶", Direction::ToEnglish);
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1, "HY-MT takes no system prompt");
        let prompt = messages[0]["content"].as_str().unwrap();
        assert!(prompt.starts_with("参考下面的翻译：\n"));
        assert!(prompt.contains("金团 翻译成 GDKP run"));
        assert!(prompt.contains("缺奶 翻译成 need a healer"));
        assert!(!prompt.contains("坦克"));
        assert!(prompt.ends_with("不要额外解释：\n金团来人 缺奶"));
        assert_eq!(body["top_k"], 20);

        let plain = request_body("HY-MT1.5-7B", "你好", Direction::ToEnglish);
        assert_eq!(
            plain["messages"][0]["content"],
            "将以下文本翻译为英语，注意只需要输出翻译后的结果，不要额外解释：\n\n你好"
        );
        let general = request_body("qwen2.5:7b", "你好", Direction::ToEnglish);
        assert_eq!(general["messages"][0]["role"], "system");
    }
}
