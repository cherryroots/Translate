use regex::Regex;
use std::sync::OnceLock;

/// True when the text holds at least one CJK ideograph.
pub fn has_chinese(text: &str) -> bool {
    text.chars().any(|c| matches!(c, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}'))
}

/// Removes WoW escape codes (links, colors, textures) so only readable text remains.
pub fn strip_codes(text: &str) -> String {
    static CODES: OnceLock<Regex> = OnceLock::new();
    let re = CODES.get_or_init(|| {
        Regex::new(r"\|H[^|]*\|h(?P<label>.*?)\|h|\|c[0-9a-fA-F]{8}|\|r|\|T[^|]*\|t|\|A[^|]*\|a").unwrap()
    });
    re.replace_all(text, "$label").into_owned()
}

/// One parsed chat log line: who or where it came from, and what was said.
#[derive(Debug, PartialEq)]
pub struct ChatLine {
    pub speaker: String,
    pub message: String,
}

/// Parses a WoWChatLog.txt line such as
/// `10/4/2026 15:07:05.123  [2. Trade] Name-Realm: text`.
/// The exact timestamp shape on Forever is unverified, so the match is loose.
pub fn parse_log_line(line: &str) -> Option<ChatLine> {
    static STAMP: OnceLock<Regex> = OnceLock::new();
    let re = STAMP.get_or_init(|| {
        Regex::new(r"^\d{1,2}/\d{1,2}(?:/\d{2,4})?\s+\d{1,2}:\d{2}:\d{2}(?:\.\d+)?(?:-\d+)?\s+").unwrap()
    });
    let body = re.replace(line.trim_end(), "");
    let body = strip_codes(&body);
    if body.is_empty() {
        return None;
    }
    match body.split_once(": ") {
        Some((speaker, message)) if speaker.len() <= 80 => Some(ChatLine {
            speaker: speaker.trim().to_string(),
            message: message.trim().to_string(),
        }),
        _ => Some(ChatLine { speaker: String::new(), message: body.trim().to_string() }),
    }
}

/// Splits "TR#12 text" into (12, "text").
pub fn parse_tagged(text: &str) -> Option<(u64, &str)> {
    let rest = text.trim().strip_prefix("TR#")?;
    let (id, body) = rest.split_once(char::is_whitespace)?;
    Some((id.parse().ok()?, body.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_chinese() {
        assert!(has_chinese("需要奶妈"));
        assert!(!has_chinese("lf tank"));
        assert!(!has_chinese("こんにちは"));
    }

    #[test]
    fn strips_links_and_colors() {
        let raw = "卖 |cff0070dd|Hitem:19019::::|h[Thunderfury]|h|r 便宜";
        assert_eq!(strip_codes(raw), "卖 [Thunderfury] 便宜");
    }

    #[test]
    fn parses_channel_line() {
        let line = "10/4/2026 15:07:05.123  [2. Trade] Xiaoming-Ashbringer: 金团来人 缺奶";
        let parsed = parse_log_line(line).unwrap();
        assert_eq!(parsed.speaker, "[2. Trade] Xiaoming-Ashbringer");
        assert_eq!(parsed.message, "金团来人 缺奶");
    }

    #[test]
    fn parses_line_without_year() {
        let parsed = parse_log_line("10/4 15:07:05.123  Xiaoming says: 你好").unwrap();
        assert_eq!(parsed.speaker, "Xiaoming says");
        assert_eq!(parsed.message, "你好");
    }

    #[test]
    fn parses_tag() {
        assert_eq!(parse_tagged("TR#12 需要坦克"), Some((12, "需要坦克")));
        assert_eq!(parse_tagged("需要坦克"), None);
    }
}
