//! The file layer: reading and writing through real files, and the worker thread reading runs on.
//!
//! The reader and writer suites in `pgsheets-io` work in memory; this is where a `File` is actually
//! handed to them, and where the workbook the window builds is the thing that goes around the trip.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use pgsheets_app::files;
use pgsheets_core::{CellStyle, HorizontalAlign};
use pgsheets_sheet::Workbook;

/// A unique path in the temporary directory, inside a directory of its own.
///
/// Tests must not depend on the working directory, and they must not see each other's files either â€” one
/// of them counts the staging files in the directory it wrote to.
fn temporary(extension: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);

    let unique = NEXT.fetch_add(1, Ordering::Relaxed);

    let directory = std::env::temp_dir().join(format!("pgsheets-app-test-{}-{unique}", std::process::id()));

    std::fs::create_dir_all(&directory).expect("the temporary directory should be creatable");

    directory.join(format!("book.{extension}"))
}

/// Removes the directory a test wrote into, so a run leaves nothing behind.
fn cleanup(path: &Path) {
    if let Some(directory) = path.parent() {
        let _ = std::fs::remove_dir_all(directory);
    }
}

/// Writes a single-sheet workbook under the worksheet name a new document uses.
fn save(path: &Path, workbook: &Workbook) -> Result<pgsheets_io::ExportResult, String> {
    files::save(path, &[("Sheet1", workbook)], "Sheet1")
}

/// A workbook with a little of everything: text, a number, a formula and a formatted date.
fn sample() -> Workbook {
    let mut workbook = files::new_workbook();

    workbook.set_raw_input(0, 0, "hello");
    workbook.set_raw_input(1, 0, "2");
    workbook.set_raw_input(2, 0, "3");
    workbook.set_raw_input(3, 0, "=SUM(A2:A3)");
    workbook.set_raw_input(4, 0, "45356");
    workbook.set_number_format(4, 0, Some("yyyy-mm-dd"));

    workbook
}

#[test]
fn a_workbook_comes_back_from_a_file_unchanged() {
    let path = temporary("xlsx");

    save(&path, &sample()).expect("the file should be written");

    let imported = files::open(&path).expect("the file should be read");
    let mut restored = files::new_workbook();

    imported.sheets[0].write_into(&mut restored);

    assert_eq!(restored.raw_input(0, 0), "hello");
    assert_eq!(restored.display_value(3, 0), "5", "the formula was recalculated");
    assert_eq!(
        restored.number_format(4, 0),
        Some("yyyy-mm-dd"),
        "a format travels with its value"
    );
    assert_eq!(
        imported.result.worksheet_count, 1,
        "one worksheet, so saving in place is allowed"
    );

    cleanup(&path);
}

#[test]
fn the_reading_half_runs_on_a_worker_thread() {
    let caller = std::thread::current().id();

    let worker = files::Job::spawn(move || Ok(std::thread::current().id()));

    assert_ne!(
        worker.wait().expect("the worker should answer"),
        caller,
        "the whole point of a job is that it does not run on the window's thread"
    );
}

#[test]
fn a_job_reports_nothing_until_the_work_is_done() {
    let job = files::Job::spawn(|| {
        std::thread::sleep(Duration::from_millis(30));

        Ok(7)
    });

    // Nothing yet: a frame must never block on a file.
    if let Some(result) = job.poll() {
        assert_eq!(result.ok(), Some(7), "the work finished instantly, which is fine");
    }

    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        if let Some(result) = job.poll() {
            assert_eq!(result.expect("the worker should succeed"), 7);
            return;
        }

        assert!(Instant::now() < deadline, "the worker never finished");

        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_file_that_is_not_there_is_an_error_rather_than_a_panic() {
    let path = temporary("xlsx");

    let message = files::open(&path).expect_err("a missing file cannot be read");

    assert!(message.contains("could not be reached"), "{message}");

    cleanup(&path);
}

#[test]
fn a_file_of_another_kind_is_refused() {
    let path = temporary("docx");

    std::fs::write(&path, b"not a workbook").expect("the file should be written");

    let message = files::open(&path).expect_err("no reader claims a .docx");

    assert!(message.contains("not a format"), "{message}");

    cleanup(&path);
}

#[test]
fn a_format_this_application_cannot_write_is_refused() {
    let path = temporary("json");

    let message = save(&path, &sample()).expect_err("no writer claims a .json");

    assert!(message.contains("no writer"), "{message}");
    assert!(!path.exists(), "nothing should have been created");

    cleanup(&path);
}

#[test]
fn a_failed_save_leaves_the_destination_untouched() {
    let path = temporary("json");

    std::fs::write(&path, b"mine").expect("the file should be written");

    save(&path, &sample()).expect_err("no writer claims a .json");

    assert_eq!(
        std::fs::read(&path).expect("the file should still be readable"),
        b"mine",
        "a save that failed must not have truncated what was there"
    );

    // Nor may it leave its half-written staging file behind.
    let directory = path.parent().expect("a temporary file has a directory");
    let left_overs = std::fs::read_dir(directory)
        .expect("the directory should be readable")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".saving"))
        .count();

    assert_eq!(left_overs, 0, "no staging file should survive a failed save");

    cleanup(&path);
}

#[test]
fn delimited_text_goes_around_the_trip_too() {
    let path = temporary("csv");

    save(&path, &sample()).expect("the file should be written");

    let imported = files::open(&path).expect("the file should be read");
    let mut restored = files::new_workbook();

    imported.sheets[0].write_into(&mut restored);

    assert_eq!(restored.raw_input(0, 0), "hello");
    assert_eq!(
        restored.value(3, 0).as_number(),
        Some(5.0),
        "the formula came back and evaluated"
    );

    cleanup(&path);
}

#[test]
fn an_empty_workbook_writes_an_empty_file() {
    let path = temporary("xlsx");

    let result = save(&path, &files::new_workbook()).expect("the file should be written");

    assert_eq!(result.cells_written, 0);

    let imported = files::open(&path).expect("the file should be read");

    assert!(imported.sheets[0].cells.is_empty());
    assert_eq!(imported.result.rows, 0);

    cleanup(&path);
}

#[test]
fn a_cell_style_comes_back_from_a_file() {
    let path = temporary("xlsx");

    let mut workbook = files::new_workbook();
    workbook.set_raw_input(0, 0, "bold");

    workbook.set_cell_style(
        0,
        0,
        CellStyle::DEFAULT
            .with_bold(true)
            .with_align(HorizontalAlign::Right),
    );

    save(&path, &workbook).expect("the file should be written");

    let imported = files::open(&path).expect("the file should be read");
    let mut restored = files::new_workbook();

    imported.sheets[0].write_into(&mut restored);

    let style = restored
        .cell_style(0, 0)
        .expect("the style should come back with its value");

    assert!(style.bold);
    assert_eq!(style.align, HorizontalAlign::Right);
    assert_eq!(restored.raw_input(0, 0), "bold");

    cleanup(&path);
}

#[test]
fn a_multi_sheet_workbook_goes_around_the_trip() {
    let path = temporary("xlsx");

    let mut first = files::new_workbook();
    first.set_raw_input(0, 0, "alpha");

    let mut second = files::new_workbook();
    second.set_raw_input(0, 0, "beta");

    let sheets: [(&str, &Workbook); 2] = [("Alpha", &first), ("Beta", &second)];

    files::save(&path, &sheets, "Beta").expect("the file should be written");

    let imported = files::open(&path).expect("the file should be read");

    // Both worksheets come back, in order, with their own cells.
    assert_eq!(imported.sheets.len(), 2);
    assert_eq!(imported.result.worksheet_count, 2);

    assert_eq!(imported.sheets[0].name, "Alpha");
    assert_eq!(imported.sheets[1].name, "Beta");

    let mut restored_first = files::new_workbook();
    imported.sheets[0].write_into(&mut restored_first);
    assert_eq!(restored_first.raw_input(0, 0), "alpha");

    let mut restored_second = files::new_workbook();
    imported.sheets[1].write_into(&mut restored_second);
    assert_eq!(restored_second.raw_input(0, 0), "beta");

    cleanup(&path);
}
