//! The formatting engine: `markup_fmt` for the markup, with `malva` and `dprint-plugin-json`
//! for embedded CSS and JSON.

pub mod line_width;
pub mod panic;

use std::borrow::Cow;
use std::panic::UnwindSafe;
use std::path::{Path, PathBuf};

use djangofmt_syntax::{FORMAT_IGNORE_DIRECTIVES, FileIgnores, LEGACY_IGNORE_DIRECTIVE, Profile};
use tracing::{debug, warn};

use crate::line_width::{IndentWidth, LineLength, SelfClosing};
use crate::panic::catch_unwind;

/// Pre-built configuration for all formatters.
#[derive(Clone)]
pub struct FormatterConfig {
    /// Config for main HTML/Jinja formatter
    pub markup: markup_fmt::config::FormatOptions,
    /// Config for CSS/SCSS formatter
    pub malva: malva::config::FormatOptions,
    /// Config for JSON formatter
    pub json: dprint_plugin_json::configuration::Configuration,
}

impl FormatterConfig {
    #[must_use]
    pub fn new(
        print_width: LineLength,
        indent_width: IndentWidth,
        custom_blocks: Option<Vec<String>>,
        raw_elements: Vec<String>,
        html_void_self_closing: SelfClosing,
        preserve_unquoted_attrs: bool,
    ) -> Self {
        Self {
            markup: build_markup_options(
                print_width,
                indent_width,
                custom_blocks,
                raw_elements,
                html_void_self_closing,
                preserve_unquoted_attrs,
            ),
            malva: build_malva_config(print_width, indent_width),
            json: build_json_config(print_width, indent_width),
        }
    }
}

/// Build default `markup_fmt` options for HTML/Jinja formatting.
#[must_use]
pub fn build_markup_options(
    print_width: LineLength,
    indent_width: IndentWidth,
    custom_blocks: Option<Vec<String>>,
    raw_elements: Vec<String>,
    html_void_self_closing: SelfClosing,
    preserve_unquoted_attrs: bool,
) -> markup_fmt::config::FormatOptions {
    markup_fmt::config::FormatOptions {
        layout: markup_fmt::config::LayoutOptions {
            print_width: print_width.into(),
            indent_width: indent_width.into(),
            ..markup_fmt::config::LayoutOptions::default()
        },
        language: markup_fmt::config::LanguageOptions {
            format_comments: false,
            // HTML void elements should not be self-closing by default:
            // See https://developer.mozilla.org/en-US/docs/Glossary/Void_element#self-closing_tags
            // <br/> -> <br>
            html_void_self_closing: html_void_self_closing.into(),
            // SVG elements should be self-closing:
            // <circle cx="50" cy="50" r="50"></circle> -> <circle cx="50" cy="50" r="50" />
            svg_self_closing: Some(true),
            // MathML elements should be self-closing:
            // <mspace width="1em"></mspace> -> <mspace width="1em" />
            mathml_self_closing: Some(true),
            // HTML normal elements should not be self-closing:
            html_normal_self_closing: Some(false),
            // This is actually nice to keep this setting false, it makes it possible to control wrapping
            // of props semi manually by inserting or not a newline before the first prop.
            // See https://github.com/g-plane/markup_fmt/issues/10 that showcase this.
            // <div
            //     class="foo"
            //     id="bar">
            // </div>
            prefer_attrs_single_line: false,
            // Parse custom Django template blocks:
            // For ex "stage,cache,flatblock,section,csp_compress"
            // {% stage %}...{% endstage %}
            // {% cache %}...{% endcache %}
            custom_blocks,
            // Keep the content of these elements byte for byte, like `<pre>`:
            // <c-markdown>
            // - item   one
            // </c-markdown>
            raw_elements,
            // Preserve unquoted HTML attribute values:
            // <c-button editable=True /> -> stays as editable=True
            preserve_unquoted_attrs,
            // Ignore formatting node with comment directive:
            //  - {# djangofmt: ignore[format] #}
            //  - {# djangofmt: ignore #} (deprecated)
            ignore_comment_directive: FORMAT_IGNORE_DIRECTIVES
                .iter()
                .map(ToString::to_string)
                .collect(),
            // Whole-file opt-outs are honored by `FileIgnores::parse` before we get here.
            // Empty so markup_fmt's own default directive stays off.
            ignore_file_comment_directive: vec![],
            // Indent style tags content:
            // <style>
            //     body { color: red }
            // </style>
            style_indent: true,
            // Indent script tags content:
            // <script>
            //     console.log("hello");
            // </script>
            script_indent: true,
            ..markup_fmt::config::LanguageOptions::default()
        },
    }
}

/// Build default `malva` options for CSS/SCSS/SASS/LESS formatting.
fn build_malva_config(
    print_width: LineLength,
    indent_width: IndentWidth,
) -> malva::config::FormatOptions {
    malva::config::FormatOptions {
        layout: malva::config::LayoutOptions {
            print_width: print_width.into(),
            indent_width: indent_width.into(),
            ..malva::config::LayoutOptions::default()
        },
        language: malva::config::LanguageOptions {
            // Because markup_fmt uses DoubleQuotes
            quotes: malva::config::Quotes::AlwaysSingle,
            operator_linebreak: malva::config::OperatorLineBreak::Before,
            linebreak_in_pseudo_parens: true,
            declaration_order: Some(malva::config::DeclarationOrder::Smacss),
            keyframe_selector_notation: Some(malva::config::KeyframeSelectorNotation::Percentage),
            single_line_top_level_declarations: true,
            selector_override_comment_directive: "djangofmt-selector-override".into(),
            ignore_comment_directive: LEGACY_IGNORE_DIRECTIVE.into(),
            ignore_file_comment_directive: LEGACY_IGNORE_DIRECTIVE.into(),
            ..malva::config::LanguageOptions::default()
        },
    }
}

fn build_json_config(
    print_width: LineLength,
    indent_width: IndentWidth,
) -> dprint_plugin_json::configuration::Configuration {
    dprint_plugin_json::configuration::ConfigurationBuilder::new()
        .line_width(print_width.value().into())
        .indent_width(indent_width.value())
        .build()
}

/// Format the given source code.
///
/// `path` only labels diagnostics, pass `None` when the source has no file behind it.
pub fn format_text(
    source: &str,
    config: &FormatterConfig,
    profile: Profile,
    path: Option<&Path>,
) -> std::result::Result<Option<String>, markup_fmt::FormatError> {
    let ignores = FileIgnores::parse(source);
    if ignores.format {
        return Ok(None);
    }
    let result = markup_fmt::format_text(
        source,
        markup_fmt::Language::from(profile),
        &config.markup,
        |code, hints| {
            match hints.ext {
                "json" | "jsonc" => {
                    // dprint mangles such a snippet instead of rejecting it, leaving a string that gains indentation on every pass -> https://github.com/dprint/dprint-plugin-json/issues/63
                    if json_has_raw_control_char(code) {
                        debug!(
                            "JSON string holds a raw control character, leaving it unformatted."
                        );
                        return Ok(code.into());
                    }
                    let fake_filename = PathBuf::from(format!("djangofmt_fmt_stdin.{}", hints.ext));
                    let mut json_config = config.json.clone();
                    json_config.line_width = u32::try_from(hints.print_width).unwrap_or(u32::MAX);
                    Ok(format_or_fallback(code, "JSON", path, || {
                        match dprint_plugin_json::format_text(&fake_filename, code, &json_config) {
                            Ok(Some(formatted)) => formatted.into(),
                            Ok(None) => code.into(),
                            Err(error) => {
                                debug!(
                                    "Failed to format JSON, falling back to original code. Error: {:?}",
                                    error
                                );
                                code.into()
                            }
                        }
                    }))
                }
                "css" | "scss" | "sass" | "less" => {
                    let mut malva_config = config.malva.clone();
                    malva_config.layout.print_width = hints.print_width;

                    let formatted_css = format_or_fallback(code, "CSS", path, || {
                        malva::format_text(code, malva::Syntax::Css, &malva_config).map_or_else(
                            |error| {
                                debug!(
                                    "Failed to format CSS, falling back to original code. Error: {:?}",
                                    error
                                );
                                code.into()
                            },
                            Cow::from,
                        )
                    });

                    // malva can return nothing at all: `single_line_top_level_declarations` drops
                    // comments, and an unterminated `/*` swallows the sheet. Keep the source.
                    if formatted_css.trim().is_empty() {
                        return Ok(code.into());
                    }

                    // Workaround a bug in malva -> https://github.com/g-plane/malva/issues/44
                    // Tries to keep on formatting style attr on a single line like expected with
                    // single_line_top_level_declarations = true
                    if code.contains('{') {
                        Ok(formatted_css)
                    } else {
                        Ok(formatted_css
                            .lines()
                            .map(str::trim)
                            .collect::<Vec<_>>()
                            .join(" ")
                            .into())
                    }
                }
                _ => Ok(code.into()),
            }
        },
    );
    match result {
        // `file-ignore[invalid-syntax]` quarantines the file instead of reporting.
        Err(markup_fmt::FormatError::Syntax(_)) if ignores.invalid_syntax => Ok(None),
        other => other.map(Some),
    }
}

/// Whether a string literal in `code` holds a raw control character (U+0000-U+001F).
///
/// JSON forbids those ([RFC 8259 §7]), so a hit means the snippet is invalid and dprint can't be
/// trusted with it.
///
/// [RFC 8259 §7]: https://www.rfc-editor.org/info/rfc8259/#section-7:~:text=All%20Unicode%20characters%20may%20be%20placed%20within%20the%20quotation%20marks%2C%20except%20for%20the%20characters%20that%20MUST%20be%20escaped%3A%20quotation%20mark%2C%20reverse%20solidus%2C%20and%20the%20control%20characters%20%28U%2B0000%20through%20U%2B001F%29%2E
fn json_has_raw_control_char(code: &str) -> bool {
    let mut escaped = false;
    let mut quote = None;
    for byte in code.bytes() {
        let Some(quote_byte) = quote else {
            // jsonc-parser also accepts single quotes, and dprint mangles those the same way.
            quote = matches!(byte, b'"' | b'\'').then_some(byte);
            continue;
        };
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == quote_byte {
            quote = None;
        } else if byte < b' ' {
            return true;
        }
    }
    false
}

/// Run an embedded formatter, falling back to the original `code` if it panics.
fn format_or_fallback<'a>(
    code: &'a str,
    language: &str,
    path: Option<&Path>,
    format: impl FnOnce() -> Cow<'a, str> + UnwindSafe,
) -> Cow<'a, str> {
    catch_unwind(format).unwrap_or_else(|err| {
        let location = err
            .location
            .map_or_else(String::new, |location| format!(" at {location}"));
        warn!(
            "{}: the embedded {language} formatter panicked{location} ({}), leaving that snippet unformatted. Please report it at {}/issues",
            path.unwrap_or_else(|| Path::new("<source>")).display(),
            err.payload,
            env!("CARGO_PKG_REPOSITORY"),
        );
        Cow::Borrowed(code)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn format_or_fallback_returns_original_code_on_panic() {
        let code = "color: red";
        assert_eq!(
            format_or_fallback(code, "CSS", None, || panic!("boom")),
            code
        );
    }

    #[rstest]
    #[case::raw_newline_in_string("{\"a\": \"b\nc\"}", true)]
    #[case::raw_newline_in_single_quoted_string("{'a': 'b\nc'}", true)]
    #[case::escaped_newline_in_string(r#"{"a": "b\nc"}"#, false)]
    #[case::newline_between_tokens("{\n\"a\": \"b\"\n}", false)]
    #[case::escaped_quote_in_string("{\"a\": \"b\\\"c\",\n\"d\": 1}", false)]
    fn json_has_raw_control_char_cases(#[case] code: &str, #[case] expected: bool) {
        assert_eq!(json_has_raw_control_char(code), expected);
    }
}
