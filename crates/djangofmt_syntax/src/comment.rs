/// A UTF-8 BOM is not Rust whitespace, so strip it explicitly.
#[must_use]
#[inline]
pub fn strip_bom(source: &str) -> &str {
    source.strip_prefix('\u{feff}').unwrap_or(source)
}

/// The delimiters of one comment style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommentDelimiters {
    pub open: &'static str,
    pub close: &'static str,
}

/// A `{# #}` template comment, the only kind that carries a lint suppression.
pub const TEMPLATE_COMMENT: CommentDelimiters = CommentDelimiters {
    open: "{#",
    close: "#}",
};

/// An `<!-- -->` HTML comment, which is rendered to the client.
pub const HTML_COMMENT: CommentDelimiters = CommentDelimiters {
    open: "<!--",
    close: "-->",
};

impl CommentDelimiters {
    /// The body of a comment and the text after it, only if `text` starts with one.
    #[inline]
    #[must_use]
    pub fn split(self, text: &str) -> Option<(&str, &str)> {
        text.strip_prefix(self.open)?.split_once(self.close)
    }
}
