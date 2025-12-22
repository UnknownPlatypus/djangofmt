use divan::Bencher;

fn main() {
    divan::main();
}

// ============================================================================
// Original implementation (from markup_fmt)
// ============================================================================

static NON_WS_SENSITIVE_TAGS: [&str; 76] = [
    "address",
    "blockquote",
    "button",
    "caption",
    "center",
    "colgroup",
    "dialog",
    "div",
    "figure",
    "figcaption",
    "footer",
    "form",
    "select",
    "option",
    "optgroup",
    "header",
    "hr",
    "legend",
    "listing",
    "main",
    "p",
    "plaintext",
    "pre",
    "progress",
    "search",
    "object",
    "details",
    "summary",
    "xmp",
    "area",
    "base",
    "basefont",
    "datalist",
    "head",
    "link",
    "meta",
    "meter",
    "noembed",
    "noframes",
    "param",
    "rp",
    "title",
    "html",
    "body",
    "article",
    "aside",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hgroup",
    "nav",
    "section",
    "table",
    "tr",
    "thead",
    "th",
    "tbody",
    "td",
    "tfoot",
    "dir",
    "dd",
    "dl",
    "dt",
    "menu",
    "ol",
    "ul",
    "li",
    "fieldset",
    "video",
    "audio",
    "picture",
    "source",
    "track",
];

fn is_whitespace_sensitive_tag_original(name: &str) -> bool {
    // Simplified version for HTML-like languages (case-insensitive)
    // There's also a tag called "a" in SVG, so we need to check it specially.
    name.eq_ignore_ascii_case("a")
        || !NON_WS_SENSITIVE_TAGS
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(name))
            && !css_dataset::tags::SVG_TAGS
                .iter()
                .any(|tag| tag.eq_ignore_ascii_case(name))
}

// ============================================================================
// Alternative 1: Using cssparser's match_ignore_ascii_case! macro
// ============================================================================

fn is_non_ws_sensitive_tag_match_macro(name: &str) -> bool {
    cssparser::match_ignore_ascii_case! { name,
        "address" | "blockquote" | "button" | "caption" | "center" | "colgroup" |
        "dialog" | "div" | "figure" | "figcaption" | "footer" | "form" | "select" |
        "option" | "optgroup" | "header" | "hr" | "legend" | "listing" | "main" |
        "p" | "plaintext" | "pre" | "progress" | "search" | "object" | "details" |
        "summary" | "xmp" | "area" | "base" | "basefont" | "datalist" | "head" |
        "link" | "meta" | "meter" | "noembed" | "noframes" | "param" | "rp" |
        "title" | "html" | "body" | "article" | "aside" | "h1" | "h2" | "h3" |
        "h4" | "h5" | "h6" | "hgroup" | "nav" | "section" | "table" | "tr" |
        "thead" | "th" | "tbody" | "td" | "tfoot" | "dir" | "dd" | "dl" | "dt" |
        "menu" | "ol" | "ul" | "li" | "fieldset" | "video" | "audio" | "picture" |
        "source" | "track" => true,
        _ => false,
    }
}

fn is_svg_tag_match_macro(name: &str) -> bool {
    cssparser::match_ignore_ascii_case! { name,
        "a" | "altglyph" | "altglyphdef" | "altglyphitem" | "animate" | "animatecolor" |
        "animatemotion" | "animatetransform" | "circle" | "clippath" | "cursor" | "defs" |
        "desc" | "ellipse" | "feblend" | "fecolormatrix" | "fecomponenttransfer" |
        "fecomposite" | "feconvolvematrix" | "fediffuselighting" | "fedisplacementmap" |
        "fedistantlight" | "fedropshadow" | "feflood" | "fefunca" | "fefuncb" | "fefuncg" |
        "fefuncr" | "fegaussianblur" | "feimage" | "femerge" | "femergenode" |
        "femorphology" | "feoffset" | "fepointlight" | "fespecularlighting" | "fespotlight" |
        "fetile" | "feturbulence" | "filter" | "font" | "font-face" | "font-face-format" |
        "font-face-name" | "font-face-src" | "font-face-uri" | "foreignobject" | "g" |
        "glyph" | "glyphref" | "hkern" | "image" | "line" | "lineargradient" | "marker" |
        "mask" | "metadata" | "missing-glyph" | "mpath" | "path" | "pattern" | "polygon" |
        "polyline" | "radialgradient" | "rect" | "set" | "stop" | "svg" | "switch" |
        "symbol" | "text" | "textpath" | "title" | "tref" | "tspan" | "use" | "view" |
        "vkern" => true,
        _ => false,
    }
}

fn is_whitespace_sensitive_tag_match_macro(name: &str) -> bool {
    name.eq_ignore_ascii_case("a")
        || !is_non_ws_sensitive_tag_match_macro(name) && !is_svg_tag_match_macro(name)
}

// ============================================================================
// Alternative 2: Using phf (perfect hash function) with case-insensitive lookup
// ============================================================================

fn is_non_ws_sensitive_tag_phf(name: &str) -> bool {
    cssparser::ascii_case_insensitive_phf_map! {
        non_ws_tags -> bool = {
            "address" => true,
            "blockquote" => true,
            "button" => true,
            "caption" => true,
            "center" => true,
            "colgroup" => true,
            "dialog" => true,
            "div" => true,
            "figure" => true,
            "figcaption" => true,
            "footer" => true,
            "form" => true,
            "select" => true,
            "option" => true,
            "optgroup" => true,
            "header" => true,
            "hr" => true,
            "legend" => true,
            "listing" => true,
            "main" => true,
            "p" => true,
            "plaintext" => true,
            "pre" => true,
            "progress" => true,
            "search" => true,
            "object" => true,
            "details" => true,
            "summary" => true,
            "xmp" => true,
            "area" => true,
            "base" => true,
            "basefont" => true,
            "datalist" => true,
            "head" => true,
            "link" => true,
            "meta" => true,
            "meter" => true,
            "noembed" => true,
            "noframes" => true,
            "param" => true,
            "rp" => true,
            "title" => true,
            "html" => true,
            "body" => true,
            "article" => true,
            "aside" => true,
            "h1" => true,
            "h2" => true,
            "h3" => true,
            "h4" => true,
            "h5" => true,
            "h6" => true,
            "hgroup" => true,
            "nav" => true,
            "section" => true,
            "table" => true,
            "tr" => true,
            "thead" => true,
            "th" => true,
            "tbody" => true,
            "td" => true,
            "tfoot" => true,
            "dir" => true,
            "dd" => true,
            "dl" => true,
            "dt" => true,
            "menu" => true,
            "ol" => true,
            "ul" => true,
            "li" => true,
            "fieldset" => true,
            "video" => true,
            "audio" => true,
            "picture" => true,
            "source" => true,
            "track" => true,
        }
    }
    non_ws_tags::get(name).is_some()
}

fn is_svg_tag_phf(name: &str) -> bool {
    cssparser::ascii_case_insensitive_phf_map! {
        svg_tags -> bool = {
            "a" => true,
            "altglyph" => true,
            "altglyphdef" => true,
            "altglyphitem" => true,
            "animate" => true,
            "animatecolor" => true,
            "animatemotion" => true,
            "animatetransform" => true,
            "circle" => true,
            "clippath" => true,
            "cursor" => true,
            "defs" => true,
            "desc" => true,
            "ellipse" => true,
            "feblend" => true,
            "fecolormatrix" => true,
            "fecomponenttransfer" => true,
            "fecomposite" => true,
            "feconvolvematrix" => true,
            "fediffuselighting" => true,
            "fedisplacementmap" => true,
            "fedistantlight" => true,
            "fedropshadow" => true,
            "feflood" => true,
            "fefunca" => true,
            "fefuncb" => true,
            "fefuncg" => true,
            "fefuncr" => true,
            "fegaussianblur" => true,
            "feimage" => true,
            "femerge" => true,
            "femergenode" => true,
            "femorphology" => true,
            "feoffset" => true,
            "fepointlight" => true,
            "fespecularlighting" => true,
            "fespotlight" => true,
            "fetile" => true,
            "feturbulence" => true,
            "filter" => true,
            "font" => true,
            "font-face" => true,
            "font-face-format" => true,
            "font-face-name" => true,
            "font-face-src" => true,
            "font-face-uri" => true,
            "foreignobject" => true,
            "g" => true,
            "glyph" => true,
            "glyphref" => true,
            "hkern" => true,
            "image" => true,
            "line" => true,
            "lineargradient" => true,
            "marker" => true,
            "mask" => true,
            "metadata" => true,
            "missing-glyph" => true,
            "mpath" => true,
            "path" => true,
            "pattern" => true,
            "polygon" => true,
            "polyline" => true,
            "radialgradient" => true,
            "rect" => true,
            "set" => true,
            "stop" => true,
            "svg" => true,
            "switch" => true,
            "symbol" => true,
            "text" => true,
            "textpath" => true,
            "title" => true,
            "tref" => true,
            "tspan" => true,
            "use" => true,
            "view" => true,
            "vkern" => true,
        }
    }
    svg_tags::get(name).is_some()
}

fn is_whitespace_sensitive_tag_phf(name: &str) -> bool {
    name.eq_ignore_ascii_case("a") || !is_non_ws_sensitive_tag_phf(name) && !is_svg_tag_phf(name)
}

// ============================================================================
// Test inputs
// ============================================================================

// Common HTML tags (mix of ws-sensitive and non-ws-sensitive)
static TEST_TAGS: &[&str] = &[
    // Non-whitespace-sensitive tags (should return false)
    "div",
    "DIV",
    "Div",
    "span", // Not in list, but also not SVG - ws sensitive
    "p",
    "table",
    "header",
    "footer",
    "section",
    "article",
    "nav",
    "main",
    "aside",
    "form",
    "button",
    "select",
    "option",
    "ul",
    "ol",
    "li",
    // Whitespace-sensitive tags (should return true)
    "a",
    "A",
    "span",
    "SPAN",
    "strong",
    "em",
    "b",
    "i",
    "code",
    "label",
    "input",
    "img",
    "br",
    // SVG tags (should return false - not ws sensitive)
    "svg",
    "path",
    "circle",
    "rect",
    "g",
    // HTML tags
    "address",
    "blockquote",
    "button",
    "caption",
    "center",
    "colgroup",
    "dialog",
    "div",
    "figure",
    "figcaption",
    "footer",
    "form",
    "select",
    "option",
    "optgroup",
    "header",
    "hr",
    "legend",
    "listing",
    "main",
    "p",
    "plaintext",
    "pre",
    "progress",
    "search",
    "object",
    "details",
    "summary",
    "xmp",
    "area",
    "base",
    "basefont",
    "datalist",
    "head",
    "link",
    "meta",
    "meter",
    "noembed",
    "noframes",
    "param",
    "rp",
    "title",
    "html",
    "body",
    "article",
    "aside",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hgroup",
    "nav",
    "section",
    "table",
    "tr",
    "thead",
    "th",
    "tbody",
    "td",
    "tfoot",
    "dir",
    "dd",
    "dl",
    "dt",
    "menu",
    "ol",
    "ul",
    "li",
    "fieldset",
    "video",
    "audio",
    "picture",
    "source",
    "track",
];

// ============================================================================
// Benchmarks
// ============================================================================

#[divan::bench(name = "original (iter + eq_ignore_ascii_case)")]
fn bench_original(bencher: Bencher) {
    bencher.bench_local(|| {
        for tag in TEST_TAGS {
            divan::black_box(is_whitespace_sensitive_tag_original(divan::black_box(tag)));
        }
    });
}

#[divan::bench(name = "match_ignore_ascii_case! macro")]
fn bench_match_macro(bencher: Bencher) {
    bencher.bench_local(|| {
        for tag in TEST_TAGS {
            divan::black_box(is_whitespace_sensitive_tag_match_macro(divan::black_box(
                tag,
            )));
        }
    });
}

#[divan::bench(name = "phf (perfect hash function)")]
fn bench_phf(bencher: Bencher) {
    bencher.bench_local(|| {
        for tag in TEST_TAGS {
            divan::black_box(is_whitespace_sensitive_tag_phf(divan::black_box(tag)));
        }
    });
}

// ============================================================================
// Correctness tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implementations_match() {
        for tag in TEST_TAGS {
            let original = is_whitespace_sensitive_tag_original(tag);
            let match_macro = is_whitespace_sensitive_tag_match_macro(tag);
            let phf = is_whitespace_sensitive_tag_phf(tag);

            assert_eq!(
                original, match_macro,
                "Mismatch for tag '{}': original={}, match_macro={}",
                tag, original, match_macro
            );
            assert_eq!(
                original, phf,
                "Mismatch for tag '{}': original={}, phf={}",
                tag, original, phf
            );
        }
    }

    #[test]
    fn test_case_insensitivity() {
        // Test that all implementations handle case insensitivity correctly
        let test_cases = [
            ("div", "DIV", "Div"),
            ("a", "A", "a"),
            ("svg", "SVG", "Svg"),
        ];

        for (lower, upper, mixed) in test_cases {
            let orig_lower = is_whitespace_sensitive_tag_original(lower);
            let orig_upper = is_whitespace_sensitive_tag_original(upper);
            let orig_mixed = is_whitespace_sensitive_tag_original(mixed);

            assert_eq!(orig_lower, orig_upper, "Case mismatch for {}", lower);
            assert_eq!(orig_lower, orig_mixed, "Case mismatch for {}", lower);

            let macro_lower = is_whitespace_sensitive_tag_match_macro(lower);
            let macro_upper = is_whitespace_sensitive_tag_match_macro(upper);
            let macro_mixed = is_whitespace_sensitive_tag_match_macro(mixed);

            assert_eq!(macro_lower, macro_upper, "Case mismatch for {}", lower);
            assert_eq!(macro_lower, macro_mixed, "Case mismatch for {}", lower);

            let phf_lower = is_whitespace_sensitive_tag_phf(lower);
            let phf_upper = is_whitespace_sensitive_tag_phf(upper);
            let phf_mixed = is_whitespace_sensitive_tag_phf(mixed);

            assert_eq!(phf_lower, phf_upper, "Case mismatch for {}", lower);
            assert_eq!(phf_lower, phf_mixed, "Case mismatch for {}", lower);
        }
    }
}
