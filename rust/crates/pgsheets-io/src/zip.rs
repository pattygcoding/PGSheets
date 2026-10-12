//! The zip container an xlsx file is.
//!
//! `std` has no archive format, so this is one of the two things this crate takes a dependency for.
//! The dependency provides compression and nothing else: what goes *into* the entries — every part,
//! content type, relationship and style table — is written here.

use std::io::{Read, Seek, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// What a reader needs to open an archive.
///
/// Rust cannot form a trait object from two non-auto traits, so the pair is named here.
pub trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

/// What a writer needs to build an archive.
pub trait WriteSeek: Write + Seek {}

impl<T: Write + Seek> WriteSeek for T {}

/// An open package: the parts of a document, by path.
pub struct Archive<R: ReadSeek> {
    inner: ZipArchive<R>,
}

impl<R: ReadSeek> Archive<R> {
    /// Opens a package.
    pub fn open(reader: R) -> Result<Self, zip::result::ZipError> {
        Ok(Self {
            inner: ZipArchive::new(reader)?,
        })
    }

    /// True when the package holds a part at that path.
    pub fn has(&mut self, path: &str) -> bool {
        self.inner.by_name(path).is_ok()
    }

    /// The bytes of a part, or `None` when it is not there.
    pub fn read(&mut self, path: &str) -> Option<Vec<u8>> {
        let mut entry = self.inner.by_name(path).ok()?;
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }

    /// A part as text, or `None` when it is missing or is not UTF-8.
    ///
    /// A byte-order mark is stripped: a writer is allowed to emit one, and it would otherwise become
    /// part of the first element's name.
    pub fn read_text(&mut self, path: &str) -> Option<String> {
        let bytes = self.read(path)?;
        let text = String::from_utf8(bytes).ok()?;
        Some(text.trim_start_matches('\u{feff}').to_string())
    }
}

/// A package being written.
pub struct PackageWriter<W: WriteSeek> {
    inner: ZipWriter<W>,
    options: SimpleFileOptions,
}

impl<W: WriteSeek> PackageWriter<W> {
    /// Creates a package that writes its parts into `writer`.
    pub fn create(writer: W) -> Self {
        Self {
            inner: ZipWriter::new(writer),
            // Deflated, which is what every spreadsheet application writes and every one reads.
            options: SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        }
    }

    /// Adds a part.
    pub fn write_part(&mut self, path: &str, content: &str) -> std::io::Result<()> {
        self.inner.start_file(path, self.options)?;
        self.inner.write_all(content.as_bytes())
    }

    /// Closes the package, writing the central directory.
    pub fn finish(self) -> std::io::Result<W> {
        self.inner
            .finish()
            .map_err(|error| std::io::Error::other(error.to_string()))
    }
}
