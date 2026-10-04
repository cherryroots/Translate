use crate::worker::{UiEvent, UiSender};
use std::thread;
use std::time::Duration;

/// Matches the WoW client by window title or process: Wow.exe, WowClassic.exe,
/// WowClassicB.exe, or "World of Warcraft" on macOS and under Wine.
pub fn is_wow(title: &str, app: &str) -> bool {
    let app = app.to_ascii_lowercase();
    let stem = app.rsplit(['/', '\\']).next().unwrap_or(&app);
    title.trim().eq_ignore_ascii_case("World of Warcraft")
        || app.contains("world of warcraft")
        || stem.starts_with("wow")
}

/// Whether WoW is the focused window, or None when focus cannot be read
/// (the overlay then stays visible).
fn wow_focused() -> Option<bool> {
    let windows = xcap::Window::all().ok()?;
    let focused = windows.iter().find(|w| w.is_focused().unwrap_or(false))?;
    let title = focused.title().unwrap_or_default();
    let app = focused.app_name().unwrap_or_default();
    Some(is_wow(&title, &app))
}

/// Polls the focused window and reports changes, so the overlay can hide
/// while another program is in front.
pub fn spawn(ui: UiSender) {
    thread::spawn(move || {
        let mut last = None;
        loop {
            let now = wow_focused();
            if now != last {
                ui.send(UiEvent::WowFocused(now));
                last = now;
            }
            thread::sleep(Duration::from_millis(250));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::is_wow;

    #[test]
    fn recognizes_wow_clients() {
        assert!(is_wow("World of Warcraft", "Wow.exe"));
        assert!(is_wow("", "WowClassic.exe"));
        assert!(is_wow("", r"C:\Games\World of Warcraft\_classic_beta_\WowClassicB.exe"));
        assert!(is_wow("World of Warcraft", "wine64-preloader"));
        assert!(!is_wow("Translate", "translate-companion"));
        assert!(!is_wow("Discord", "Discord.exe"));
    }
}
