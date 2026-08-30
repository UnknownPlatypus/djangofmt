//! Formats the input twice under each profile and panics on any panic inside
//! `markup_fmt`, non-idempotent output, or output that no longer parses.

#![no_main]

use std::sync::LazyLock;

use djangofmt::args::Profile;
use djangofmt::commands::format::{FormatterConfig, format_text};
use djangofmt::error::ParseError;
use djangofmt::line_width::{IndentWidth, LineLength, SelfClosing};
use libfuzzer_sys::{Corpus, fuzz_target};

static CONFIG: LazyLock<FormatterConfig> = LazyLock::new(|| {
    FormatterConfig::new(
        LineLength::default(),
        IndentWidth::default(),
        None,
        SelfClosing::default(),
        false,
    )
});

fn do_fuzz_profile(code: &str, profile: Profile) -> Corpus {
    let formatted = match format_text(code, &CONFIG, profile, None) {
        Ok(Some(formatted)) => formatted,
        // Skipped via a djangofmt:ignore directive.
        Ok(None) => return Corpus::Reject,
        Err(err) => {
            // Exercise the diagnostic path, it does offset arithmetic on the error location.
            let _ = ParseError::new(None, code.to_string(), &err);
            return Corpus::Reject;
        }
    };

    match format_text(&formatted, &CONFIG, profile, None) {
        Ok(Some(reformatted)) => {
            similar_asserts::assert_eq!(
                formatted,
                reformatted,
                "profile: {profile:?}, input: {code:?}"
            );
        }
        // Formatting can hoist an ignore directive to the start, turning the second pass into a skip.
        Ok(None) => return Corpus::Reject,
        Err(err) => panic!(
            "formatted output no longer parses:\nprofile: {profile:?}\ninput: {code:?}\nformatted: {formatted:?}\nerror: {err:?}"
        ),
    }
    Corpus::Keep
}

fn do_fuzz(case: &[u8]) -> Corpus {
    let Ok(code) = std::str::from_utf8(case) else {
        return Corpus::Reject;
    };

    // Both profiles share the seed corpus: fixtures are plain templates, not profile-tagged.
    let django = do_fuzz_profile(code, Profile::Django);
    let jinja = do_fuzz_profile(code, Profile::Jinja);
    if matches!(django, Corpus::Keep) || matches!(jinja, Corpus::Keep) {
        Corpus::Keep
    } else {
        Corpus::Reject
    }
}

fuzz_target!(|case: &[u8]| -> Corpus { do_fuzz(case) });
