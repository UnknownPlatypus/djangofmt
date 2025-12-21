use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;

const VALID_METHODS: &[&str] = &["get", "post", "dialog"];

pub fn check(element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("form") {
        return;
    }

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value, .. }) = attr {
            if name.eq_ignore_ascii_case("method") {
                if let Some((value_str, offset)) = value {
                    let normalized = value_str.to_ascii_lowercase();
                    if !VALID_METHODS.contains(&normalized.as_str()) {
                        checker.report(
                            "E001",
                            format!(
                                "Invalid form method '{value_str}'. Expected one of: get, post, dialog"
                            ),
                            (*offset, value_str.len()).into(),
                            "invalid method".into(),
                            Some("Use 'get', 'post', or 'dialog'".into()),
                        );
                    }
                }
            }
        }
    }
}
