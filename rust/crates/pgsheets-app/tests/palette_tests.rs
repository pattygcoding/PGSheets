//! The colour palette: what it remembers, and the file the saved colours live in.
//!
//! The palette is model-only — no UI type appears in it — so all of this runs without a window, and the
//! file round trip is the only part that touches a disk.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use pgsheets_app::palette::{Palette, STANDARD_COLORS, config_path_from};

/// A unique path in the temporary directory, inside a directory of its own.
fn temporary() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);

    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let directory =
        std::env::temp_dir().join(format!("pgsheets-palette-test-{}-{unique}", std::process::id()));

    std::fs::create_dir_all(&directory).expect("the temporary directory should be creatable");

    directory.join("colors.txt")
}

/// Removes the directory a test wrote into.
fn cleanup(path: &Path) {
    if let Some(directory) = path.parent() {
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[test]
fn a_used_colour_goes_to_the_front_without_duplicating() {
    let mut palette = Palette::new();

    palette.note_used([1, 2, 3]);
    palette.note_used([4, 5, 6]);
    palette.note_used([1, 2, 3]);

    assert_eq!(palette.recent(), &[[1, 2, 3], [4, 5, 6]]);
}

#[test]
fn the_recent_list_is_bounded() {
    let mut palette = Palette::new();

    for value in 0..40u8 {
        palette.note_used([value, 0, 0]);
    }

    assert_eq!(palette.recent().len(), 12, "the oldest fall off the end");
    assert_eq!(palette.recent()[0], [39, 0, 0], "the newest is first");
}

#[test]
fn a_saved_colour_is_kept_once_and_can_be_dropped() {
    let mut palette = Palette::new();

    palette.save([1, 2, 3]);
    palette.save([1, 2, 3]);
    assert_eq!(
        palette.saved(),
        &[[1, 2, 3]],
        "a colour saved twice is saved once"
    );

    palette.forget([1, 2, 3]);
    assert!(palette.saved().is_empty());
}

#[test]
fn a_palette_round_trips_through_its_file_text() {
    let mut palette = Palette::new();
    palette.save([0x12, 0x34, 0x56]);
    palette.save([0xAB, 0xCD, 0xEF]);

    assert_eq!(palette.encode(), "saved #123456\nsaved #ABCDEF\n");

    // A line that cannot be read is skipped rather than taking the rest of the file down with it.
    let restored = Palette::decode("saved #123456\nnonsense\nsaved zzz\nsaved #ABCDEF\n");
    assert_eq!(restored.saved(), &[[0x12, 0x34, 0x56], [0xAB, 0xCD, 0xEF]]);
}

#[test]
fn saved_colours_survive_a_trip_through_a_file() {
    let path = temporary();

    let mut palette = Palette::load_from(&path);
    assert!(
        palette.saved().is_empty(),
        "a file that is not there is simply an empty palette"
    );

    palette.save([0x11, 0x22, 0x33]);
    palette.store();

    let restored = Palette::load_from(&path);
    assert_eq!(restored.saved(), &[[0x11, 0x22, 0x33]]);

    cleanup(&path);
}

#[test]
fn a_palette_with_no_file_saves_nothing() {
    // A palette built by hand has no file, which is what keeps a test — and the default window — from
    // ever touching the real configuration.
    let mut palette = Palette::new();
    palette.save([1, 2, 3]);
    palette.store();

    assert_eq!(palette.saved(), &[[1, 2, 3]], "it is still held in memory");
}

#[test]
fn the_configuration_path_is_a_colors_file_in_the_users_configuration() {
    #[cfg(target_os = "windows")]
    let key = "APPDATA";
    #[cfg(target_os = "macos")]
    let key = "HOME";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let key = "XDG_CONFIG_HOME";

    let path = config_path_from(|name| (name == key).then(|| "/tmp/config".into()));

    assert_eq!(
        path.and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned())),
        Some("colors.txt".to_string())
    );

    // With nothing in the environment there is nowhere to write, and that is not an error.
    assert!(config_path_from(|_| None).is_none());
}

#[test]
fn the_standard_palette_has_no_duplicates() {
    let mut seen: Vec<[u8; 3]> = Vec::new();

    for colour in STANDARD_COLORS {
        assert!(!seen.contains(&colour), "{colour:?} appears twice");
        seen.push(colour);
    }
}
