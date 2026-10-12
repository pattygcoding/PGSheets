//! A small XML writer for the SpreadsheetML parts.
//!
//! The C# original used `XmlWriter`. This produces the same shape — a declaration, elements carrying
//! their namespace once, escaped text — in about a hundred lines, so the file-format crates keep
//! their dependency surface to the two crates that provide what `std` cannot.

use crate::ooxml::MAIN_NAMESPACE;

/// Writes one XML part.
///
/// Empty elements are written as a start/end pair rather than being self-closed. That is what this
/// writer does; both forms parse identically and every reader accepts both.
#[derive(Default)]
pub struct XmlWriter {
    out: String,

    /// The elements that are open, innermost last, with the default namespace each was written in.
    open: Vec<(String, Option<String>)>,

    /// True while an element's start tag is still open, so that an attribute written after the element
    /// name lands inside it and the tag is closed exactly once.
    unclosed: bool,
}

impl XmlWriter {
    /// Creates a writer with an empty part.
    pub fn new() -> Self {
        Self::default()
    }

    /// Writes the declaration, which every part starts with.
    pub fn declaration(&mut self) {
        self.out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>");
    }

    /// Opens an element in the main SpreadsheetML namespace.
    pub fn start(&mut self, name: &str) {
        self.start_in(name, MAIN_NAMESPACE);
    }

    /// Opens an element, declaring the namespace when it differs from the one in scope.
    pub fn start_in(&mut self, name: &str, namespace: &str) {
        self.close_start_tag();

        let current = self.open.last().and_then(|(_, namespace)| namespace.as_deref());

        self.out.push('<');
        self.out.push_str(name);

        if current != Some(namespace) {
            self.out.push_str(" xmlns=\"");
            escape_attribute_into(&mut self.out, namespace);
            self.out.push('"');
        }

        self.open.push((name.to_string(), Some(namespace.to_string())));
        self.unclosed = true;
    }

    /// Closes the element that is currently open.
    ///
    /// The namespace declaration that element made goes out of scope with it, which is why the stack
    /// holds the namespace as well as the name.
    pub fn end(&mut self) {
        self.close_start_tag();

        let Some((name, _)) = self.open.pop() else {
            return;
        };

        self.out.push_str("</");
        self.out.push_str(&name);
        self.out.push('>');
    }

    /// Writes the `>` that finishes a start tag, if it has not been written yet.
    fn close_start_tag(&mut self) {
        if self.unclosed {
            self.out.push('>');
            self.unclosed = false;
        }
    }

    /// Writes an attribute.
    pub fn attribute(&mut self, name: &str, value: &str) {
        self.out.push(' ');
        self.out.push_str(name);
        self.out.push_str("=\"");
        escape_attribute_into(&mut self.out, value);
        self.out.push('"');
    }

    /// Writes an already-qualified attribute such as `xml:space`, which needs no declaration.
    pub fn qualified_attribute(&mut self, qualified_name: &str, value: &str) {
        self.attribute(qualified_name, value);
    }

    /// Declares a namespace prefix on the element being written.
    pub fn namespace_attribute(&mut self, prefix: &str, namespace: &str) {
        self.out.push_str(" xmlns:");
        self.out.push_str(prefix);
        self.out.push_str("=\"");
        escape_attribute_into(&mut self.out, namespace);
        self.out.push('"');
    }

    /// Writes text content, escaped.
    pub fn text(&mut self, text: &str) {
        self.close_start_tag();
        escape_text_into(&mut self.out, text);
    }

    /// The part as a string.
    pub fn as_str(&self) -> &str {
        &self.out
    }

    /// Consumes the writer and returns the part.
    pub fn into_string(self) -> String {
        self.out
    }
}

/// Escapes text content: the characters that would otherwise be markup, and a carriage return, which
/// XML would otherwise normalise away.
fn escape_text_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            _ => out.push(ch),
        }
    }
}

/// Escapes an attribute value, which additionally cannot carry a literal quote, newline or tab.
fn escape_attribute_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            '\t' => out.push_str("&#x9;"),
            _ => out.push(ch),
        }
    }
}
