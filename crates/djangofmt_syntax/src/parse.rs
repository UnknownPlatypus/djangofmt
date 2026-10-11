use markup_fmt::ast::Root;
use markup_fmt::parser::Parser;
use markup_fmt::{Language, SyntaxError};

/// A parsed template, carrying what every consumer of the AST needs to work on it.
///
/// The AST comes with the source it borrows and the configuration it was parsed with, and only
/// [`parse`] builds one, so a check or a fix can never run against a source or a configuration
/// the AST was not built from.
#[derive(Debug)]
pub struct Parsed<'a> {
    source: &'a str,
    language: Language,
    custom_blocks: &'a [String],
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

    /// The `custom-blocks` the source was parsed with.
    #[must_use]
    pub const fn custom_blocks(&self) -> &'a [String] {
        self.custom_blocks
    }
}

/// Parse `source`, treating each of `custom_blocks` as a `{% tag %}...{% endtag %}` block.
///
/// The single door to the parser: every consumer (check, fix, playground,
/// benches, tests) must parse with the same configuration or lint on a
/// different AST than the one the formatter sees.
pub fn parse<'a>(
    source: &'a str,
    language: Language,
    custom_blocks: &'a [String],
) -> Result<Parsed<'a>, SyntaxError> {
    Ok(Parsed {
        source,
        language,
        custom_blocks,
        ast: Parser::new(source, language, custom_blocks.to_vec()).parse_root()?,
    })
}
