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

/// Sends one message to an OpenAI-compatible /chat/completions endpoint.
pub fn translate(config: &Config, text: &str, direction: Direction) -> Result<String, String> {
    let system = match direction {
        Direction::ToEnglish => TO_ENGLISH,
        Direction::ToChinese => TO_CHINESE,
    };
    let url = format!("{}/chat/completions", config.api_base.trim_end_matches('/'));
    let body = json!({
        "model": config.model,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": text },
        ],
    });

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
        .ok_or_else(|| "the API reply had no text".to_string())
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
}
