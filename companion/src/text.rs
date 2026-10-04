/// True when the text holds at least one CJK ideograph.
pub fn has_chinese(text: &str) -> bool {
    text.chars().any(|c| matches!(c, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}'))
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
    fn parses_tag() {
        assert_eq!(parse_tagged("TR#12 需要坦克"), Some((12, "需要坦克")));
        assert_eq!(parse_tagged("需要坦克"), None);
    }
}
