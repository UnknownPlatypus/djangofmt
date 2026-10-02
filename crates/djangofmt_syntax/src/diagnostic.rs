use miette::{GraphicalReportHandler, GraphicalTheme, SourceSpan};

/// Narrow a `usize` byte offset or length to the `u32` `oxc-miette` stores.
///
/// Files larger than 4 GiB are not a supported input, so this saturates.
#[must_use]
pub fn clamp_offset(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Wrap help and note text without splitting URLs.
/// `textwrap` treats `/` and `-` as break opportunities, so a URL is not one word.
#[must_use]
pub fn graphical_handler(theme: GraphicalTheme) -> GraphicalReportHandler {
    GraphicalReportHandler::new_themed(theme)
        .with_word_separator(textwrap::WordSeparator::AsciiSpace)
        .with_word_splitter(textwrap::WordSplitter::NoHyphenation)
        .with_break_words(false)
}

/// Build a [`SourceSpan`] from `usize` byte offsets.
#[must_use]
pub fn span(start: usize, len: usize) -> SourceSpan {
    SourceSpan::new(clamp_offset(start).into(), clamp_offset(len))
}
