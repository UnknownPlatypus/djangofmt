use markup_fmt::ast::Root;
use markup_fmt::parser::Parser;
use markup_fmt::{Language, SyntaxError};

/// A parsed template: the AST, the source it borrows, and the profile it was parsed with.
///
/// Only [`parse`] builds one, so a check can never run against a source or a profile
/// the AST was not built from.
#[derive(Debug)]
pub struct Parsed<'a> {
    source: &'a str,
    language: Language,
    ast: Root<'a>,
}

impl<'a> Parsed<'a> {
    /// The AST.
    #[must_use]
    pub const fn ast(&self) -> &Root<'a> {
        &self.ast
    }

    /// The source the AST borrows.
    #[must_use]
    pub const fn source(&self) -> &'a str {
        self.source
    }

    /// The profile the source was parsed with.
    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }
}

/// Parse `source`, treating each of `custom_blocks` as a `{% tag %}...{% endtag %}` block
/// and keeping the content of `raw_elements` as text.
///
/// The single door to the parser: every consumer (check, fix, playground,
/// benches, tests) must parse with the same configuration or lint on a
/// different AST than the one the formatter sees.
pub fn parse<'a>(
    source: &'a str,
    language: Language,
    custom_blocks: &[String],
    raw_elements: &[String],
) -> Result<Parsed<'a>, SyntaxError> {
    Ok(Parsed {
        source,
        language,
        ast: Parser::new(
            source,
            language,
            custom_blocks.to_vec(),
            raw_elements.to_vec(),
        )
        .parse_root()?,
    })
}
