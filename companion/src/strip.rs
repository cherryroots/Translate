//! Reads the addon's pixel strip off the screen. The layout must match
//! addon/Translate/Strip.lua: 64 x 8 blocks, 3 sync blocks, then
//! [seq][len hi][len lo][text...][fletcher16 hi][fletcher16 lo] packed
//! 3 bytes into 4 six-bit blocks.

use crate::worker::{Job, UiEvent, UiSender};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};
use xcap::image::RgbaImage;
use xcap::Monitor;

pub const COLS: usize = 64;
pub const ROWS: usize = 8;
const SYNC: [u8; 3] = [51, 15, 60];

/// Where the strip sits in an image, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lock {
    pub x: u32,
    pub y: u32,
    pub block: u32,
}

#[derive(Debug, PartialEq)]
pub struct Frame {
    pub seq: u8,
    pub checksum: u16,
    pub text: String,
}

#[derive(Debug, PartialEq)]
pub enum Decoded {
    Frame(Frame),
    /// Sync blocks present but the data did not check out (mid-repaint).
    Corrupt,
    /// No strip at this spot.
    Missing,
}

/// Maps a channel value to one of the four levels the addon draws (0, 85, 170, 255).
fn level(v: u8) -> u8 {
    ((v as u16 + 42) / 85).min(3) as u8
}

fn symbol(img: &RgbaImage, x: u32, y: u32) -> Option<u8> {
    if x >= img.width() || y >= img.height() {
        return None;
    }
    let p = img.get_pixel(x, y).0;
    Some(level(p[0]) * 16 + level(p[1]) * 4 + level(p[2]))
}

fn fletcher16(bytes: &[u8]) -> u16 {
    let (mut a, mut b) = (0u16, 0u16);
    for &byte in bytes {
        a = (a + byte as u16) % 255;
        b = (b + a) % 255;
    }
    b * 256 + a
}

pub fn decode(img: &RgbaImage, lock: Lock) -> Decoded {
    let mut symbols = Vec::with_capacity(COLS * ROWS);
    for i in 0..COLS * ROWS {
        let cx = lock.x + (i % COLS) as u32 * lock.block + lock.block / 2;
        let cy = lock.y + (i / COLS) as u32 * lock.block + lock.block / 2;
        match symbol(img, cx, cy) {
            Some(s) => symbols.push(s),
            None => return Decoded::Missing,
        }
    }
    if symbols[..3] != SYNC {
        return Decoded::Missing;
    }
    let mut bytes = Vec::with_capacity((symbols.len() - 3) / 4 * 3);
    for chunk in symbols[3..].chunks_exact(4) {
        let v = (chunk[0] as u32) << 18 | (chunk[1] as u32) << 12 | (chunk[2] as u32) << 6 | chunk[3] as u32;
        bytes.extend_from_slice(&[(v >> 16) as u8, (v >> 8) as u8, v as u8]);
    }
    let len = (bytes[1] as usize) << 8 | bytes[2] as usize;
    if 3 + len + 2 > bytes.len() {
        return Decoded::Corrupt;
    }
    let checksum = (bytes[3 + len] as u16) << 8 | bytes[4 + len] as u16;
    if fletcher16(&bytes[..3 + len]) != checksum {
        return Decoded::Corrupt;
    }
    match String::from_utf8(bytes[3..3 + len].to_vec()) {
        Ok(text) => Decoded::Frame(Frame { seq: bytes[0], checksum, text }),
        Err(_) => Decoded::Corrupt,
    }
}

/// Scans a whole image for the sync blocks and returns where the strip is.
pub fn find(img: &RgbaImage) -> Option<Lock> {
    let is = |x: u32, y: u32, s: u8| symbol(img, x, y) == Some(s);
    for y in 0..img.height() {
        let mut x = 0;
        while x < img.width() {
            if !is(x, y, SYNC[0]) {
                x += 1;
                continue;
            }
            let start = x;
            while is(x, y, SYNC[0]) {
                x += 1;
            }
            let block = x - start;
            let cyan = (x..).take_while(|&cx| is(cx, y, SYNC[1])).count() as u32;
            let yellow = (x + cyan..).take_while(|&cx| is(cx, y, SYNC[2])).count() as u32;
            if cyan == 0 || yellow == 0 {
                continue;
            }
            // Averaging the three runs tolerates a blended edge pixel.
            let block = ((block + cyan + yellow) as f32 / 3.0).round() as u32;
            let mut top = y;
            while top > 0 && is(start, top - 1, SYNC[0]) {
                top -= 1;
            }
            let lock = Lock { x: start, y: top, block: block.max(1) };
            if decode(img, lock) != Decoded::Missing {
                return Some(lock);
            }
        }
    }
    None
}

/// The payload is "<id>\t<speaker>\t<text>".
pub fn parse_payload(text: &str) -> Option<(String, String)> {
    let mut parts = text.splitn(3, '\t');
    let _id = parts.next()?;
    let speaker = parts.next()?.to_string();
    let message = parts.next()?.to_string();
    Some((speaker, message))
}

/// Polls the screen: a full scan about once a second until the strip turns
/// up, then only the strip's own rectangle several times a second.
pub fn spawn(jobs: Sender<Job>, ui: UiSender) {
    thread::spawn(move || {
        let mut locked: Option<(usize, Lock)> = None;
        let mut last_seen: Option<(u8, u16)> = None;
        let mut last_good = Instant::now();
        let mut said_searching = false;

        loop {
            let monitors = match Monitor::all() {
                Ok(m) => m,
                Err(e) => {
                    ui.send(UiEvent::Status(format!("Screen capture unavailable: {e}")));
                    thread::sleep(Duration::from_secs(5));
                    continue;
                }
            };

            let Some((index, lock)) = locked else {
                if !said_searching {
                    ui.send(UiEvent::Strip("Looking for the pixel strip. In game, type /tr test.".into()));
                    said_searching = true;
                }
                for (index, monitor) in monitors.iter().enumerate() {
                    let Ok(img) = monitor.capture_image() else { continue };
                    if let Some(lock) = find(&img) {
                        ui.send(UiEvent::Strip(format!(
                            "Reading the pixel strip on screen {} at {},{}",
                            index + 1,
                            lock.x,
                            lock.y
                        )));
                        locked = Some((index, lock));
                        last_good = Instant::now();
                        said_searching = false;
                        break;
                    }
                }
                if locked.is_none() {
                    thread::sleep(Duration::from_millis(1000));
                }
                continue;
            };

            let Some(monitor) = monitors.get(index) else {
                locked = None;
                continue;
            };
            let (w, h) = (COLS as u32 * lock.block, ROWS as u32 * lock.block);
            let decoded = match monitor.capture_region(lock.x, lock.y, w, h) {
                Ok(img) => decode(&img, Lock { x: 0, y: 0, block: lock.block }),
                Err(_) => Decoded::Missing,
            };
            match decoded {
                Decoded::Frame(frame) => {
                    last_good = Instant::now();
                    if last_seen != Some((frame.seq, frame.checksum)) {
                        // The first frame after locking may be an old line; it is
                        // still new to the overlay, so it is shown too.
                        last_seen = Some((frame.seq, frame.checksum));
                        if let Some((speaker, text)) = parse_payload(&frame.text) {
                            let _ = jobs.send(Job::Incoming { speaker, text });
                        }
                    }
                }
                Decoded::Corrupt => {}
                Decoded::Missing => {
                    // Hidden UI, a loading screen, or the window moved.
                    if last_good.elapsed() > Duration::from_secs(3) {
                        locked = None;
                    }
                }
            }
            thread::sleep(Duration::from_millis(80));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use xcap::image::Rgba;

    /// Rust copy of Strip.lua's encoder, for tests only.
    fn encode(seq: u8, text: &str) -> Vec<u8> {
        let mut bytes = vec![seq, (text.len() >> 8) as u8, text.len() as u8];
        bytes.extend_from_slice(text.as_bytes());
        let sum = fletcher16(&bytes);
        bytes.extend_from_slice(&[(sum >> 8) as u8, sum as u8]);
        let mut symbols = SYNC.to_vec();
        for chunk in bytes.chunks(3) {
            let v = (chunk[0] as u32) << 16
                | (*chunk.get(1).unwrap_or(&0) as u32) << 8
                | *chunk.get(2).unwrap_or(&0) as u32;
            symbols.extend_from_slice(&[(v >> 18) as u8 & 63, (v >> 12) as u8 & 63, (v >> 6) as u8 & 63, v as u8 & 63]);
        }
        symbols.resize(COLS * ROWS, 0);
        symbols
    }

    fn paint(symbols: &[u8], at: (u32, u32), block: u32) -> RgbaImage {
        let mut img = RgbaImage::from_pixel(800, 200, Rgba([30, 30, 40, 255]));
        for (i, s) in symbols.iter().enumerate() {
            let (r, g, b) = ((s >> 4) & 3, (s >> 2) & 3, s & 3);
            for dy in 0..block {
                for dx in 0..block {
                    let x = at.0 + (i % COLS) as u32 * block + dx;
                    let y = at.1 + (i / COLS) as u32 * block + dy;
                    // A small color shift, as a display pipeline might add.
                    img.put_pixel(x, y, Rgba([(r * 85).saturating_add(6), (g * 85).saturating_sub(5), b * 85, 255]));
                }
            }
        }
        img
    }

    #[test]
    fn finds_and_decodes() {
        let text = "7\t[Party] Lili\t副本缺T 速来";
        let img = paint(&encode(42, text), (37, 11), 4);
        let lock = find(&img).expect("strip found");
        assert_eq!(lock, Lock { x: 37, y: 11, block: 4 });
        match decode(&img, lock) {
            Decoded::Frame(f) => {
                assert_eq!(f.seq, 42);
                assert_eq!(f.text, text);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(parse_payload(text), Some(("[Party] Lili".into(), "副本缺T 速来".into())));
    }

    #[test]
    fn rejects_damaged_frame() {
        let mut symbols = encode(1, "1\tA\t你好");
        symbols[10] ^= 1;
        let img = paint(&symbols, (0, 0), 3);
        assert_eq!(decode(&img, Lock { x: 0, y: 0, block: 3 }), Decoded::Corrupt);
    }
}
