//! The colour palette: the basic swatches, the colours recently used, and the colours the user saved.
//!
//! A colour here is a plain `[u8; 3]` — the same shape a cell style carries — so nothing in this module
//! needs a UI type, and all of it is testable without a window. The saved and recent lists live in a
//! small text file beside the user's configuration, which is why they survive a restart without a
//! serialisation crate in the dependency tree.

use std::path::PathBuf;

/// The basic colours the menu offers, in the order a spreadsheet shows them: a ramp of neutrals, the
/// saturated row most workbooks reach for, and lighter tints for a background.
pub const STANDARD_COLORS: [[u8; 3]; 40] = [
    [0x00, 0x00, 0x00],
    [0x26, 0x26, 0x26],
    [0x40, 0x40, 0x40],
    [0x59, 0x59, 0x59],
    [0x80, 0x80, 0x80],
    [0xA6, 0xA6, 0xA6],
    [0xBF, 0xBF, 0xBF],
    [0xD9, 0xD9, 0xD9],
    [0xC0, 0x00, 0x00],
    [0xFF, 0x00, 0x00],
    [0xFF, 0xC0, 0x00],
    [0xFF, 0xFF, 0x00],
    [0x92, 0xD0, 0x50],
    [0x00, 0xB0, 0x50],
    [0x00, 0xB0, 0xF0],
    [0x00, 0x70, 0xC0],
    [0x00, 0x20, 0x60],
    [0x70, 0x30, 0xA0],
    [0xC0, 0x00, 0xFF],
    [0xFF, 0x00, 0xFF],
    [0x83, 0x3C, 0x00],
    [0xBF, 0x8F, 0x00],
    [0x37, 0x56, 0x23],
    [0x54, 0x82, 0x35],
    [0xED, 0x7D, 0x31],
    [0xF4, 0xB1, 0x83],
    [0xFF, 0xD9, 0x66],
    [0xA9, 0xD0, 0x8E],
    [0x70, 0xAD, 0x47],
    [0x9D, 0xC3, 0xE6],
    [0x2E, 0x75, 0xB6],
    [0x44, 0x72, 0xC4],
    [0xFC, 0xE4, 0xD6],
    [0xFF, 0xF2, 0xCC],
    [0xE2, 0xEF, 0xDA],
    [0xDD, 0xEB, 0xF7],
    [0xDA, 0xE3, 0xF3],
    [0xE4, 0xDF, 0xEC],
    [0xFB, 0xE5, 0xD6],
    [0xF2, 0xF2, 0xF2],
];

/// How many recently used colours are remembered.
const RECENT_LIMIT: usize = 12;

/// The palette a document is edited with: what was used lately, and what the user chose to keep.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Palette {
    recent: Vec<[u8; 3]>,
    saved: Vec<[u8; 3]>,

    /// Where the saved colours live, or `None` when they are not persisted — which is what a palette
    /// built by hand, and every test, wants: neither should touch the user's real configuration.
    path: Option<PathBuf>,
}

impl Palette {
    /// An empty palette.
    pub fn new() -> Self {
        Self::default()
    }

    /// The recently used colours, most recent first.
    pub fn recent(&self) -> &[[u8; 3]] {
        &self.recent
    }

    /// The colours the user saved.
    pub fn saved(&self) -> &[[u8; 3]] {
        &self.saved
    }

    /// Records a colour as used: most recent first, without duplicates, and bounded.
    pub fn note_used(&mut self, colour: [u8; 3]) {
        self.recent.retain(|existing| *existing != colour);
        self.recent.insert(0, colour);
        self.recent.truncate(RECENT_LIMIT);
    }

    /// Keeps a colour, unless it is already kept.
    pub fn save(&mut self, colour: [u8; 3]) {
        if !self.saved.contains(&colour) {
            self.saved.push(colour);
        }
    }

    /// Drops a saved colour.
    pub fn forget(&mut self, colour: [u8; 3]) {
        self.saved.retain(|existing| *existing != colour);
    }

    /// The palette as the text of its file: one `saved` line per colour.
    ///
    /// Only the saved colours are written. The recently used ones are a session's convenience, and
    /// writing a file every time a colour is applied would be a side effect nobody asked for.
    pub fn encode(&self) -> String {
        let mut text = String::new();

        for colour in &self.saved {
            text.push_str(&format!("saved {}\n", hex(*colour)));
        }

        text
    }

    /// Reads a palette from the text of its file, ignoring any line it cannot make sense of.
    pub fn decode(text: &str) -> Self {
        let mut palette = Self::default();

        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let (Some("saved"), Some(value)) = (parts.next(), parts.next()) else {
                continue;
            };

            if let Some(colour) = parse_hex(value) {
                palette.save(colour);
            }
        }

        palette
    }

    /// Reads the saved colours from the user's configuration, or an empty palette when there is none.
    pub fn load() -> Self {
        config_path()
            .map(|path| Self::load_from(&path))
            .unwrap_or_default()
    }

    /// Reads a palette from a particular file, and remembers it for [`Palette::store`].
    pub fn load_from(path: &std::path::Path) -> Self {
        let mut palette = std::fs::read_to_string(path)
            .map(|text| Self::decode(&text))
            .unwrap_or_default();

        palette.path = Some(path.to_path_buf());
        palette
    }

    /// Writes the saved colours back to the file they came from.
    ///
    /// Best-effort, and a no-op when the palette has no file: a palette that cannot be saved is not
    /// worth interrupting anyone over, so every failure here is deliberately ignored.
    pub fn store(&self) {
        let Some(path) = &self.path else {
            return;
        };

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let _ = std::fs::write(path, self.encode());
    }
}

/// The file the palette is kept in, or `None` when no configuration directory can be worked out.
pub fn config_path() -> Option<PathBuf> {
    config_path_from(|name| std::env::var_os(name))
}

/// The path, given a way to read the environment — which is what makes it testable without one.
pub fn config_path_from(env: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = PathBuf::from(env("APPDATA").or_else(|| env("USERPROFILE"))?).join("PGSheets");

    #[cfg(target_os = "macos")]
    let base = PathBuf::from(env("HOME")?)
        .join("Library")
        .join("Application Support")
        .join("PGSheets");

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = env("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".config")))?
        .join("pgsheets");

    Some(base.join("colors.txt"))
}

/// A colour as `#RRGGBB`.
fn hex(colour: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", colour[0], colour[1], colour[2])
}

/// Reads `#RRGGBB` or `RRGGBB`.
fn parse_hex(value: &str) -> Option<[u8; 3]> {
    let hex = value.strip_prefix('#').unwrap_or(value);

    if hex.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some([red, green, blue])
}
