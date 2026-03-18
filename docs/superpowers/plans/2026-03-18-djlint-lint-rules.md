# djLint Lint Rules Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement djLint-equivalent lint rules in djangofmt's `djangofmt_lint` crate, covering accessibility, correctness, and style checks.

**Architecture:** Each rule follows the existing pattern: a violation struct implementing the `Violation` trait, registered via `define_rules!` macro, wired into `checker.rs` visitor methods. Rules are grouped by category (correctness, suspicious, style). Tests use insta snapshot testing with `.html` fixtures.

**Tech Stack:** Rust, markup_fmt AST, miette diagnostics, insta snapshots

---

## How to Add a New Lint Rule (Reference)

Every rule follows these exact steps:

1. **Create violation file** at `crates/djangofmt_lint/src/rules/<category>/<rule_name>.rs`
2. **Export it** from `crates/djangofmt_lint/src/rules/<category>/mod.rs`
3. **Register in** `crates/djangofmt_lint/src/registry.rs` inside `define_rules!`
4. **Wire check in** `crates/djangofmt_lint/src/checker.rs` in the appropriate `visit_*` method
5. **Add test fixture** at `crates/djangofmt_lint/tests/check/<rule_name>/<rule_name>.html`
6. **Run tests** with `cargo test -p djangofmt_lint` to generate snapshot, then `cargo insta review`
7. **Commit**

The existing `invalid-attr-value` rule is the canonical example. See:

- Rule: `crates/djangofmt_lint/src/rules/correctness/invalid_attr_value.rs`
- Test: `crates/djangofmt_lint/tests/check/form_method/form_method.html`
- Snapshot: `crates/djangofmt_lint/tests/check/form_method/form_method.snap`

---

## Task 0: Add Source Text to Checker for Offset Computation

Element-level rules (H013, H037, H036, H020, etc.) need to point diagnostics at the element's tag name. The current AST's `NativeAttribute.value` includes a byte offset, but `Element.tag_name` is just a `&str` with no offset. We can compute offsets via pointer arithmetic since `tag_name` is borrowed from the same source string.

**Files:**

- Modify: `crates/djangofmt_lint/src/checker.rs`
- Modify: `crates/djangofmt_lint/src/lib.rs`

- [ ] **Step 1: Add `source` field to `Checker`**

In `checker.rs`, update the struct and constructor:

```rust
pub struct Checker<'a> {
    settings: &'a Settings,
    source: &'a str,
    diagnostics: Vec<LintDiagnostic>,
}

impl<'a> Checker<'a> {
    #[must_use]
    pub const fn new(settings: &'a Settings, source: &'a str) -> Self {
        Self {
            settings,
            source,
            diagnostics: Vec::new(),
        }
    }

    /// Compute the byte offset of a borrowed string slice within the source.
    /// Both `slice` and `self.source` must point into the same allocation.
    #[must_use]
    pub fn offset_of(&self, slice: &str) -> usize {
        let source_start = self.source.as_ptr() as usize;
        let slice_start = slice.as_ptr() as usize;
        debug_assert!(
            slice_start >= source_start
                && slice_start <= source_start + self.source.len(),
            "slice is not within source"
        );
        slice_start - source_start
    }
}
```

- [ ] **Step 2: Update `check_ast` to pass source**

In `lib.rs`:

```rust
pub fn check_ast(ast: &Root<'_>, settings: &Settings, source: &str) -> Vec<LintDiagnostic> {
    let mut checker = Checker::new(settings, source);
    checker.visit_root(ast);
    checker.into_diagnostics()
}
```

- [ ] **Step 3: Update all callers of `check_ast`**

Update `crates/djangofmt/src/commands/check.rs` and `crates/djangofmt_lint/tests/check/main.rs` to pass the source string.

- [ ] **Step 4: Verify it compiles and existing tests pass**

Run: `cargo test -p djangofmt_lint`
Expected: All existing tests pass (no behavior change)

- [ ] **Step 5: Commit**

```bash
git commit -m "refactor(lint): add source text tracking to Checker for offset computation"
```

---

## Task 1: Add Style and Suspicious Category Modules

Currently only the `correctness` category module exists. We need `style` and `suspicious`.

**Files:**

- Create: `crates/djangofmt_lint/src/rules/style/mod.rs`
- Create: `crates/djangofmt_lint/src/rules/suspicious/mod.rs`
- Modify: `crates/djangofmt_lint/src/rules/mod.rs`

- [ ] **Step 1: Create category module files**

`crates/djangofmt_lint/src/rules/style/mod.rs`:

```rust
```

`crates/djangofmt_lint/src/rules/suspicious/mod.rs`:

```rust
```

- [ ] **Step 2: Export new modules**

`crates/djangofmt_lint/src/rules/mod.rs`:

```rust
pub mod correctness;
pub mod style;
pub mod suspicious;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p djangofmt_lint`
Expected: success

- [ ] **Step 4: Commit**

```bash
git add crates/djangofmt_lint/src/rules/style/mod.rs crates/djangofmt_lint/src/rules/suspicious/mod.rs crates/djangofmt_lint/src/rules/mod.rs
git commit -m "feat(lint): add style and suspicious category modules"
```

---

## Task 2: Missing Attribute on Element — H013 (img alt)

Pattern: Check that a specific element has a required attribute. This is the template for H005 and H006.

**Files:**

- Create: `crates/djangofmt_lint/src/rules/style/missing_attr.rs`
- Modify: `crates/djangofmt_lint/src/rules/style/mod.rs`
- Modify: `crates/djangofmt_lint/src/registry.rs`
- Modify: `crates/djangofmt_lint/src/checker.rs`
- Create: `crates/djangofmt_lint/tests/check/missing_img_alt/missing_img_alt.html`

- [ ] **Step 1: Write the test fixture**

`crates/djangofmt_lint/tests/check/missing_img_alt/missing_img_alt.html`:

```html
<!-- Valid - has alt -->
<img src="photo.jpg" alt="A photo">
<img src="photo.jpg" alt="">

<!-- Invalid - missing alt -->
<img src="photo.jpg">
<img src="photo.jpg" width="100" height="100">

<!-- Dynamic attributes - skip -->
<img src="photo.jpg" {% if show_alt %}alt="Photo"{% endif %}>

<!-- Nested in Jinja block -->
{% if show_image %}
    <img src="photo.jpg">
{% endif %}
```

- [ ] **Step 2: Create the violation**

`crates/djangofmt_lint/src/rules/style/missing_attr.rs`:

```rust
//! missing-img-alt / missing-html-lang / missing-img-dimensions
//!
//! Checks that specific elements have required attributes for
//! accessibility and best practices.

use markup_fmt::ast::{Attribute, Element};

use crate::Checker;
use crate::registry::Rule;
use crate::violation::Violation;

/// ## What it does
/// Checks that `<img>` tags have an `alt` attribute.
///
/// ## Why is this bad?
/// Screen readers need alt text to describe images to visually impaired users.
/// An empty `alt=""` is acceptable for decorative images.
#[derive(Debug, PartialEq, Eq)]
pub struct MissingImgAlt;

impl Violation for MissingImgAlt {
    const RULE: Rule = Rule::MissingImgAlt;

    fn message(&self) -> String {
        "img tag should have an alt attribute.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Add alt=\"description\" or alt=\"\" for decorative images.".to_string())
    }
}

/// Check that `<img>` elements have an `alt` attribute.
pub fn check_img_alt(element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("img") {
        return;
    }

    let has_alt = element.attrs.iter().any(|attr| matches!(
        attr,
        Attribute::Native(native) if native.name.eq_ignore_ascii_case("alt")
    ));

    if !has_alt {
        let offset = checker.offset_of(element.tag_name);
        checker.report(&MissingImgAlt, (offset, element.tag_name.len()).into());
    }
}
```

> **Note:** This rule requires `Checker.offset_of()` from Task 0, since `Element.tag_name` has no explicit offset. Place in Wave 2 (after Task 0 is complete).

- [ ] **Step 3: Register and wire up**

In `registry.rs`, add to `define_rules!`:

```rust
/// Checks that <img> tags have an alt attribute.
(MissingImgAlt, Style, rules::style::missing_attr::MissingImgAlt),
```

In `checker.rs` `visit_element()`:

```rust
if self.is_rule_enabled(Rule::MissingImgAlt) {
    rules::style::missing_attr::check_img_alt(element, self);
}
```

In `crates/djangofmt_lint/src/rules/style/mod.rs`:

```rust
pub mod missing_attr;
```

- [ ] **Step 4: Run tests and review snapshot**

Run: `cargo test -p djangofmt_lint`
Then: `cargo insta review`
Expected: Snapshot shows diagnostics for the 3 `<img>` tags without `alt`

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(lint): add missing-img-alt rule (H013)"
```

### Rules following this same pattern:

**H005 — `<html>` must have `lang`:**

- Violation: `MissingHtmlLang` in same file
- Check: `element.tag_name == "html"` → look for `lang` attribute
- Message: `"html tag should have a lang attribute."`
- Test fixture: `tests/check/missing_html_lang/missing_html_lang.html`

**H006 — `<img>` should have `height` and `width`:**

- Violation: `MissingImgDimensions` in same file
- Check: `element.tag_name == "img"` → look for both `height` AND `width`
- Message: `"img tag should have height and width attributes."`
- Test fixture: `tests/check/missing_img_dimensions/missing_img_dimensions.html`

---

## Task 3: Forbidden Attribute — H021 (inline styles)

Pattern: Flag elements that have a specific attribute they shouldn't.

**Files:**

- Create: `crates/djangofmt_lint/src/rules/style/forbidden_attr.rs`
- Modify: `crates/djangofmt_lint/src/rules/style/mod.rs`
- Modify: `crates/djangofmt_lint/src/registry.rs`
- Modify: `crates/djangofmt_lint/src/checker.rs`
- Create: `crates/djangofmt_lint/tests/check/inline_style/inline_style.html`

- [ ] **Step 1: Write the test fixture**

`crates/djangofmt_lint/tests/check/inline_style/inline_style.html`:

```html
<!-- Valid - no inline style -->
<div class="red"></div>

<!-- Invalid - inline style -->
<div style="color: red;"></div>
<p style="margin: 0;"></p>

<!-- Dynamic style - still flag it -->
<div style="{{ my_style }}"></div>
```

- [ ] **Step 2: Create the violation**

`crates/djangofmt_lint/src/rules/style/forbidden_attr.rs`:

```rust
//! Rules that flag attributes that should not be used.

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::Rule;
use crate::violation::Violation;

/// ## What it does
/// Checks that elements do not use inline `style` attributes.
///
/// ## Why is this bad?
/// Inline styles are hard to maintain and override. Use CSS classes instead.
#[derive(Debug, PartialEq, Eq)]
pub struct InlineStyle;

impl Violation for InlineStyle {
    const RULE: Rule = Rule::InlineStyle;

    fn message(&self) -> String {
        "Inline styles should be avoided.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use a CSS class instead.".to_string())
    }
}

pub fn check_inline_style(element: &Element<'_>, checker: &mut Checker<'_>) {
    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, .. }) = attr {
            if name.eq_ignore_ascii_case("style") {
                let offset = checker.offset_of(name);
                checker.report(&InlineStyle, (offset, name.len()).into());
            }
        }
    }
}
```

- [ ] **Step 3: Register, wire, export, test, commit** (same pattern as Task 2)

### Rules following this same pattern:

**H024 — Omit `type` on scripts and styles:**

- Violation: `RedundantTypeAttr`
- Check: `<script type="text/javascript">` or `<style type="text/css">` — only flag these specific redundant values
- Message: `"Redundant type attribute on <{tag}> tag."`
- Help: `"type=\"text/javascript\" and type=\"text/css\" are defaults and can be removed."`
- Test: `tests/check/redundant_type_attr/redundant_type_attr.html`

**H026 — Empty `id` and `class`:**

- Violation: `EmptyAttrValue`
- Check: Any element with `id=""` or `class=""`
- Message: `"Empty '{attr}' attribute can be removed."`
- Test: `tests/check/empty_attr_value/empty_attr_value.html`

---

## Task 4: Duplicate Attributes — H037

Unique pattern: check for repeated attribute names on the same element.

**Files:**

- Create: `crates/djangofmt_lint/src/rules/suspicious/duplicate_attr.rs`
- Modify: `crates/djangofmt_lint/src/rules/suspicious/mod.rs`
- Modify: `crates/djangofmt_lint/src/registry.rs`
- Modify: `crates/djangofmt_lint/src/checker.rs`
- Create: `crates/djangofmt_lint/tests/check/duplicate_attr/duplicate_attr.html`

- [ ] **Step 1: Write the test fixture**

`crates/djangofmt_lint/tests/check/duplicate_attr/duplicate_attr.html`:

```html
<!-- Valid - unique attributes -->
<div class="foo" id="bar"></div>

<!-- Invalid - duplicate attributes -->
<div class="foo" class="bar"></div>
<img src="a.jpg" alt="A" src="b.jpg">

<!-- Jinja conditional attributes are fine (parsed as Attribute::JinjaBlock, not NativeAttribute) -->
<div {% if x %}class="a"{% else %}class="b"{% endif %}></div>
```

- [ ] **Step 2: Create the violation**

`crates/djangofmt_lint/src/rules/suspicious/duplicate_attr.rs`:

```rust
//! duplicate-attr: Duplicate attribute found on element.

use rustc_hash::FxHashSet;
use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::Rule;
use crate::violation::Violation;

#[derive(Debug, PartialEq, Eq)]
pub struct DuplicateAttr {
    pub name: String,
}

impl Violation for DuplicateAttr {
    const RULE: Rule = Rule::DuplicateAttr;

    fn message(&self) -> String {
        format!("Duplicate attribute '{}'.", self.name)
    }

    fn help(&self) -> Option<String> {
        Some("Remove the duplicate attribute.".to_string())
    }
}

pub fn check(element: &Element<'_>, checker: &mut Checker<'_>) {
    let mut seen = FxHashSet::default();

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, .. }) = attr {
            let lower = name.to_ascii_lowercase();
            if !seen.insert(lower) {
                let offset = checker.offset_of(name);
                checker.report(
                    &DuplicateAttr { name: (*name).to_string() },
                    (offset, name.len()).into(),
                );
            }
        }
    }
}
```

- [ ] **Step 3: Register, wire, export, test, commit**

---

## Task 5: Suspicious URL Patterns — H019, H022

Pattern: Check attribute values for suspicious URL patterns.

**Files:**

- Create: `crates/djangofmt_lint/src/rules/suspicious/suspicious_url.rs`
- Modify: `crates/djangofmt_lint/src/rules/suspicious/mod.rs`
- Modify: `crates/djangofmt_lint/src/registry.rs`
- Modify: `crates/djangofmt_lint/src/checker.rs`
- Create: `crates/djangofmt_lint/tests/check/javascript_url/javascript_url.html`
- Create: `crates/djangofmt_lint/tests/check/use_https/use_https.html`

- [ ] **Step 1: Write test fixtures**

`javascript_url.html`:

```html
<!-- Valid -->
<a href="/page/">Link</a>
<a href="{% url 'home' %}">Home</a>
<button onclick="doStuff()">Click</button>

<!-- Invalid -->
<a href="javascript:void(0)">Link</a>
<a href="javascript:alert('xss')">Link</a>
```

`use_https.html`:

```html
<!-- Valid -->
<a href="https://example.com">Link</a>
<a href="/relative">Link</a>
<a href="{{ url }}">Link</a>

<!-- Invalid -->
<a href="http://example.com">Link</a>
<link rel="stylesheet" href="http://cdn.example.com/style.css">
<script src="http://cdn.example.com/script.js"></script>
```

- [ ] **Step 2: Create violations**

`crates/djangofmt_lint/src/rules/suspicious/suspicious_url.rs`:

```rust
//! URL-related lint rules.

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::Rule;
use crate::violation::Violation;

// --- H019: javascript: URLs ---

#[derive(Debug, PartialEq, Eq)]
pub struct JavascriptUrl;

impl Violation for JavascriptUrl {
    const RULE: Rule = Rule::JavascriptUrl;

    fn message(&self) -> String {
        "Avoid 'javascript:' URLs.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use an event handler and a real URL instead.".to_string())
    }
}

pub fn check_javascript_url(element: &Element<'_>, checker: &mut Checker<'_>) {
    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value: Some((val, offset)), .. }) = attr {
            if name.eq_ignore_ascii_case("href")
                && val.trim().to_ascii_lowercase().starts_with("javascript:")
            {
                checker.report(&JavascriptUrl, (*offset, val.len()).into());
            }
        }
    }
}

// --- H022: Use HTTPS ---

#[derive(Debug, PartialEq, Eq)]
pub struct UseHttps;

impl Violation for UseHttps {
    const RULE: Rule = Rule::UseHttps;

    fn message(&self) -> String {
        "Use HTTPS for external links.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Replace 'http://' with 'https://'.".to_string())
    }
}

const URL_ATTRS: &[&str] = &["href", "src", "action"];

pub fn check_use_https(element: &Element<'_>, checker: &mut Checker<'_>) {
    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value: Some((val, offset)), .. }) = attr {
            if URL_ATTRS.iter().any(|a| name.eq_ignore_ascii_case(a))
                && val.starts_with("http://")
                && !val.contains("{{")
                && !val.contains("{%")
            {
                checker.report(
                    &UseHttps,
                    (*offset, val.len()).into(),
                );
            }
        }
    }
}
```

- [ ] **Step 3: Register both rules, wire in `visit_element`, test, commit**

---

## Task 6: Django-specific URL Rules — D004, D018

Pattern: Detect hardcoded URLs that should use Django template tags.

**Files:**

- Create: `crates/djangofmt_lint/src/rules/suspicious/django_url.rs`
- Create: `crates/djangofmt_lint/tests/check/django_static_url/django_static_url.html`
- Create: `crates/djangofmt_lint/tests/check/django_url_pattern/django_url_pattern.html`

- [ ] **Step 1: Write test fixtures**

`django_static_url.html`:

```html
<!-- Valid -->
<link href="{% static 'style.css' %}">
<img src="{% static 'img/logo.png' %}">
<img src="{{ STATIC_URL }}img/logo.png">

<!-- Invalid -->
<link href="/static/style.css">
<img src="/static/img/logo.png">
```

`django_url_pattern.html`:

```html
<!-- Valid -->
<a href="{% url 'home' %}">Home</a>
<a href="{{ url }}">Link</a>
<a href="https://external.com">External</a>
<a href="#anchor">Anchor</a>
<a href="/static/file.css">Static (handled by D004)</a>
<a href="/media/file.jpg">Media</a>
<a href="mailto:user@example.com">Email</a>
<a href="tel:+1234567890">Phone</a>
<a href="data:text/html,Hello">Data URI</a>

<!-- Invalid -->
<a href="/home/">Home</a>
<a href="/accounts/login/">Login</a>
```

- [ ] **Step 2: Create violations**

`crates/djangofmt_lint/src/rules/suspicious/django_url.rs`:

```rust
//! Django-specific URL pattern rules.

use markup_fmt::ast::{Attribute, Element, NativeAttribute};

use crate::Checker;
use crate::registry::Rule;
use crate::violation::Violation;

// --- D004: Static URLs should use {% static %} ---

#[derive(Debug, PartialEq, Eq)]
pub struct DjangoStaticUrl;

impl Violation for DjangoStaticUrl {
    const RULE: Rule = Rule::DjangoStaticUrl;

    fn message(&self) -> String {
        "Static URL should use {% static %} template tag.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use {% static 'path/to/file' %} instead of hardcoding '/static/...'.".to_string())
    }
}

/// Detects hardcoded `/static/` paths in URL attributes.
pub fn check_static_url(element: &Element<'_>, checker: &mut Checker<'_>) {
    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value: Some((val, offset)), .. }) = attr {
            if is_url_attr(name)
                && (val.starts_with("/static/") || val.starts_with("static/"))
                && !val.contains("{{")
                && !val.contains("{%")
            {
                checker.report(&DjangoStaticUrl, (*offset, val.len()).into());
            }
        }
    }
}

// --- D018: Internal links should use {% url %} ---

#[derive(Debug, PartialEq, Eq)]
pub struct DjangoUrlPattern;

impl Violation for DjangoUrlPattern {
    const RULE: Rule = Rule::DjangoUrlPattern;

    fn message(&self) -> String {
        "Internal link should use {% url %} template tag.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use {% url 'view_name' %} instead of hardcoding internal paths.".to_string())
    }
}

/// Detects hardcoded internal URLs in `href` attributes.
/// Heuristic: paths starting with `/` that are not static/media, anchors, or protocol-relative.
pub fn check_url_pattern(element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("a") {
        return;
    }

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value: Some((val, offset)), .. }) = attr {
            if !name.eq_ignore_ascii_case("href") {
                continue;
            }
            // Skip template expressions, anchors, protocols, static/media, data URIs
            if val.contains("{{") || val.contains("{%")
                || val.starts_with('#')
                || val.starts_with("//")
                || val.starts_with("http")
                || val.starts_with("mailto:")
                || val.starts_with("tel:")
                || val.starts_with("data:")
                || val.starts_with("/static/")
                || val.starts_with("/media/")
            {
                continue;
            }
            // Flag paths that look like internal URLs (start with /)
            if val.starts_with('/') {
                checker.report(&DjangoUrlPattern, (*offset, val.len()).into());
            }
        }
    }
}

fn is_url_attr(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "href" | "src" | "action" | "data"
    )
}
```

> **Note:** D004 and D018 should only be enabled in Django profile (not Jinja). To implement this:
>
> 1. Add a `profile: Option<Profile>` field to `Settings`
> 2. In `checker.rs`, gate the D-prefixed rule checks with `self.settings.profile == Some(Profile::Django)`
> 3. Pass the profile from `CheckCommand` args into `Settings`
>    This should be done as part of this task before wiring the rules.

- [ ] **Step 3: Register, wire, test, commit**

---

## Task 7: Attribute Value Style Rules — H029, H033, H026

Pattern: Check attribute values for style issues.

**Files:**

- Create: `crates/djangofmt_lint/src/rules/style/attr_value_style.rs`
- Create: `crates/djangofmt_lint/tests/check/lowercase_form_method/lowercase_form_method.html`
- Create: `crates/djangofmt_lint/tests/check/form_action_whitespace/form_action_whitespace.html`
- Create: `crates/djangofmt_lint/tests/check/empty_attr_value/empty_attr_value.html`

### H029 — Lowercase form method values

```rust
#[derive(Debug, PartialEq, Eq)]
pub struct UppercaseFormMethod {
    pub value: String,
}

impl Violation for UppercaseFormMethod {
    const RULE: Rule = Rule::UppercaseFormMethod;

    fn message(&self) -> String {
        format!("Form method '{}' should be lowercase.", self.value)
    }

    fn help(&self) -> Option<String> {
        Some(format!("Use '{}' instead.", self.value.to_ascii_lowercase()))
    }
}

pub fn check_form_method_case(element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("form") {
        return;
    }
    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value: Some((val, offset)), .. }) = attr {
            if name.eq_ignore_ascii_case("method")
                && !val.contains("{{")
                && !val.contains("{%")
                && *val != val.to_ascii_lowercase()
            {
                checker.report(
                    &UppercaseFormMethod { value: (*val).to_string() },
                    (*offset, val.len()).into(),
                );
            }
        }
    }
}
```

Test fixture:

```html
<!-- Valid -->
<form method="get"></form>
<form method="post"></form>

<!-- Invalid -->
<form method="GET"></form>
<form method="POST"></form>
<form method="Post"></form>
```

### H033 — Whitespace in form action

```rust
#[derive(Debug, PartialEq, Eq)]
pub struct FormActionWhitespace;

impl Violation for FormActionWhitespace {
    const RULE: Rule = Rule::FormActionWhitespace;

    fn message(&self) -> String {
        "Extra whitespace found in form action.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Remove leading and trailing whitespace from the action URL.".to_string())
    }
}

pub fn check_form_action_whitespace(element: &Element<'_>, checker: &mut Checker<'_>) {
    if !element.tag_name.eq_ignore_ascii_case("form") {
        return;
    }
    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value: Some((val, offset)), .. }) = attr {
            if name.eq_ignore_ascii_case("action")
                && (val.starts_with(' ') || val.ends_with(' '))
            {
                checker.report(&FormActionWhitespace, (*offset, val.len()).into());
            }
        }
    }
}
```

### H026 — Empty id/class

```rust
#[derive(Debug, PartialEq, Eq)]
pub struct EmptyAttrValue {
    pub attr: String,
}

impl Violation for EmptyAttrValue {
    const RULE: Rule = Rule::EmptyAttrValue;

    fn message(&self) -> String {
        format!("Empty '{}' attribute can be removed.", self.attr)
    }
}

pub fn check_empty_id_class(element: &Element<'_>, checker: &mut Checker<'_>) {
    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value: Some((val, offset)), .. }) = attr {
            if (name.eq_ignore_ascii_case("id") || name.eq_ignore_ascii_case("class"))
                && val.is_empty()
            {
                checker.report(
                    &EmptyAttrValue { attr: (*name).to_string() },
                    (*offset, 0).into(), // empty value
                );
            }
        }
    }
}
```

- [ ] **Steps: Create file, register all 3 rules, wire in `visit_element`, test fixtures, commit**

---

## Task 8: Redundant Type Attribute — H024

**Files:**

- Create: `crates/djangofmt_lint/src/rules/style/redundant_type_attr.rs`
- Create: `crates/djangofmt_lint/tests/check/redundant_type_attr/redundant_type_attr.html`

```html
<!-- Valid - no type or non-default type -->
<script src="app.js"></script>
<script type="module" src="app.js"></script>
<script type="application/ld+json">{"@context": "..."}</script>
<style>.foo { color: red; }</style>

<!-- Invalid - redundant default type -->
<script type="text/javascript" src="app.js"></script>
<style type="text/css">.foo { color: red; }</style>
```

```rust
#[derive(Debug, PartialEq, Eq)]
pub struct RedundantTypeAttr {
    pub tag: String,
    pub type_value: String,
}

impl Violation for RedundantTypeAttr {
    const RULE: Rule = Rule::RedundantTypeAttr;

    fn message(&self) -> String {
        format!(
            "Redundant type=\"{}\" on <{}> tag.",
            self.type_value, self.tag,
        )
    }

    fn help(&self) -> Option<String> {
        Some("The type attribute is unnecessary for the default script/style type.".to_string())
    }
}

pub fn check(element: &Element<'_>, checker: &mut Checker<'_>) {
    let tag = element.tag_name.to_ascii_lowercase();
    let default_type = match tag.as_str() {
        "script" => "text/javascript",
        "style" => "text/css",
        _ => return,
    };

    for attr in &element.attrs {
        if let Attribute::Native(NativeAttribute { name, value: Some((val, offset)), .. }) = attr {
            if name.eq_ignore_ascii_case("type") && val.eq_ignore_ascii_case(default_type) {
                checker.report(
                    &RedundantTypeAttr {
                        tag: tag.clone(),
                        type_value: (*val).to_string(),
                    },
                    (*offset, val.len()).into(),
                );
            }
        }
    }
}
```

- [ ] **Steps: Create file, register, wire, test, commit**

---

## Task 9: Element Name Checks — H036 (avoid br)

Pattern: Flag use of specific elements.

**Files:**

- Create: `crates/djangofmt_lint/src/rules/style/avoid_element.rs`
- Create: `crates/djangofmt_lint/tests/check/avoid_br/avoid_br.html`

```html
<!-- Valid -->
<p>Paragraph 1</p>
<p>Paragraph 2</p>

<!-- Invalid -->
<p>Line 1<br>Line 2</p>
<p>Line 1<br/>Line 2</p>
```

```rust
#[derive(Debug, PartialEq, Eq)]
pub struct AvoidBrTag;

impl Violation for AvoidBrTag {
    const RULE: Rule = Rule::AvoidBrTag;

    fn message(&self) -> String {
        "Avoid use of <br> tags.".to_string()
    }

    fn help(&self) -> Option<String> {
        Some("Use block-level elements or CSS margins instead.".to_string())
    }
}

pub fn check_br(element: &Element<'_>, checker: &mut Checker<'_>) {
    if element.tag_name.eq_ignore_ascii_case("br") {
        let offset = checker.offset_of(element.tag_name);
        checker.report(&AvoidBrTag, (offset, element.tag_name.len()).into());
    }
}
```

- [ ] **Steps: Create file, register, wire, test, commit**

---

## Task 10: Empty Tag Pair — H020

**Files:**

- Create: `crates/djangofmt_lint/src/rules/suspicious/empty_tag.rs`
- Create: `crates/djangofmt_lint/tests/check/empty_tag/empty_tag.html`

```html
<!-- Valid -->
<div>Content</div>
<div>{{ variable }}</div>
<div>{% if x %}...{% endif %}</div>
<br>
<hr>
<img src="x.jpg" alt="x">
<i class="fa fa-icon"></i>

<!-- Invalid -->
<div></div>
<span></span>
<p></p>

<!-- Edge case: whitespace-only children (check parser behavior) -->
<div>   </div>
```

```rust
#[derive(Debug, PartialEq, Eq)]
pub struct EmptyTagPair {
    pub tag: String,
}

impl Violation for EmptyTagPair {
    const RULE: Rule = Rule::EmptyTagPair;

    fn message(&self) -> String {
        format!("Empty <{}> tag pair found. Consider removing.", self.tag)
    }
}

pub fn check(element: &Element<'_>, checker: &mut Checker<'_>) {
    // Void elements are naturally empty
    if element.void_element || element.self_closing {
        return;
    }
    // Skip elements commonly used empty with a class (icons, styled slots, etc.)
    if (element.tag_name.eq_ignore_ascii_case("i")
        || element.tag_name.eq_ignore_ascii_case("span"))
        && has_class_attr(element)
    {
        return;
    }
    if element.children.is_empty() {
        let offset = checker.offset_of(element.tag_name);
        checker.report(
            &EmptyTagPair { tag: element.tag_name.to_string() },
            (offset, element.tag_name.len()).into(),
        );
    }
}

fn has_class_attr(element: &Element<'_>) -> bool {
    element.attrs.iter().any(|attr| matches!(
        attr,
        Attribute::Native(NativeAttribute { name, .. }) if name.eq_ignore_ascii_case("class")
    ))
}
```

> **Note:** H020 needs careful thought about which empty tags to flag. `<i class="fa fa-icon"></i>`, `<span id="target"></span>`, `<div id="app"></div>` are all valid patterns. Consider making this a Nursery rule initially.

- [ ] **Steps: Create file, register, wire, test, commit**

---

## Task 11: Document Structure Checks — H007 (DOCTYPE), H016 (title)

Pattern: Analyze root-level AST nodes. Requires a new `visit_root`-level check in the checker.

**Files:**

- Create: `crates/djangofmt_lint/src/rules/style/document_structure.rs`
- Modify: `crates/djangofmt_lint/src/checker.rs` (add root-level checks)
- Create: `crates/djangofmt_lint/tests/check/missing_doctype/missing_doctype.html`
- Create: `crates/djangofmt_lint/tests/check/missing_title/missing_title.html`

> **Important:** These rules only make sense for full HTML documents, not template partials. Most Django templates are partials (e.g., `{% extends "base.html" %}`). These rules should probably skip files that contain `{% extends %}` or `{% block %}` at the root level. This needs design consideration.

- [ ] **Step 1: Add root-level check hook in checker**

In `checker.rs`, modify `visit_root()`:

```rust
pub fn visit_root(&mut self, root: &Root<'_>) {
    // Root-level checks
    if self.is_rule_enabled(Rule::MissingDoctype) {
        rules::style::document_structure::check_doctype(root, self);
    }
    if self.is_rule_enabled(Rule::MissingTitle) {
        rules::style::document_structure::check_title(root, self);
    }

    for node in &root.children {
        self.visit_node(node);
    }
}
```

- [ ] **Step 2: Implement checks**

```rust
use markup_fmt::ast::{Node, NodeKind, Root};

pub fn check_doctype(root: &Root<'_>, checker: &mut Checker<'_>) {
    // Skip template partials (files with {% extends %} or {% block %})
    if is_template_partial(root) {
        return;
    }

    let has_html = root.children.iter().any(|n| matches!(
        &n.kind,
        NodeKind::Element(el) if el.tag_name.eq_ignore_ascii_case("html")
    ));
    let has_doctype = root.children.iter().any(|n| matches!(
        &n.kind,
        NodeKind::Doctype(_)
    ));

    if has_html && !has_doctype {
        checker.report(&MissingDoctype, (0, 0).into());
    }
}

fn is_template_partial(root: &Root<'_>) -> bool {
    // {% extends %} appears as a JinjaTag at root level.
    // {% block %} appears as a JinjaBlock (with body) at root level,
    // and its opening tag is the first item in the body.
    root.children.iter().any(|n| match &n.kind {
        NodeKind::JinjaTag(tag) => {
            tag.content.trim().starts_with("extends")
        }
        NodeKind::JinjaBlock(_) => {
            // A root-level JinjaBlock (e.g., {% block content %}...{% endblock %})
            // indicates this is a partial/child template
            true
        }
        _ => false,
    })
}
```

- [ ] **Steps: Create file, register, wire, test, commit**

---

## Task 12: Entity References — H023

**Files:**

- Create: `crates/djangofmt_lint/src/rules/style/entity_ref.rs`
- Create: `crates/djangofmt_lint/tests/check/entity_ref/entity_ref.html`

> **Note:** This rule requires scanning text nodes for HTML entities like `&nbsp;`, `&amp;`, etc. The AST provides `TextNode` with `raw` text. The rule should flag entity references and suggest using the literal Unicode character instead. Exception: `&amp;`, `&lt;`, `&gt;`, `&quot;` which are necessary in certain contexts.

> **Complexity:** Medium — requires iterating text nodes and regex/pattern matching for `&...;` patterns. May require adding `NodeKind::Text` handling to the checker visitor.

---

## Task 13: Complex/Deferred Rules

These rules require more analysis or upstream changes and should be tackled after the simpler rules are stable.

### T027 — Unclosed string in template syntax

- **Complexity:** High
- **Approach:** Scan `JinjaTag.content` for unbalanced quotes
- **Risk:** False positives on multi-line tags, complex expressions

### T028 — Spaceless tags in attributes

- **Complexity:** Medium
- **Approach:** Check `JinjaTag` nodes that appear inside attribute context (via `visit_jinja_attr_block`) for missing `{%-` / `-%}` trim markers
- **Note:** The checker already has `visit_jinja_attr_block` — this is the right hook

### T034 — `{% ... }%` typo detection

- **Complexity:** Medium
- **Approach:** Requires raw source scanning since `}%` would likely be a parser error or produce unexpected AST. Check if the parser already catches this.

### H025 — Orphan tags

- **Complexity:** High
- **Approach:** The parser already detects unclosed tags as syntax errors. Check if this is redundant with parser error reporting. If not, would need parent-child tag matching analysis.

### T002 — Double quotes in template tags

- **Not a lint rule** — needs upstream formatter (markup_fmt) support for Jinja tag quote normalization

### T003 — Endblock naming

- **Not a lint rule** — needs upstream formatter support for `{% endblock %}` → `{% endblock name %}`

### T032 — Extra whitespace in template tags

- **Not a lint rule** — needs upstream formatter support for `{%  if  %}` → `{% if %}`

### H014 — Extra blank lines

- **Not a lint rule** — formatter territory

### H015 — Line break after heading tags

- **Not a lint rule** — formatter territory

---

## Implementation Priority

**Wave 0 — Prerequisites:**

1. Task 0: Add source text to Checker (required for element-level offset computation)
2. Task 1: Category modules (style, suspicious)

**Wave 1 — Attribute-value rules (correct offsets via `NativeAttribute.value`):**
3. Task 5: H019 javascript URLs, H022 HTTPS (Suspicious, security)
4. Task 7: H029 form method case, H033 form action whitespace, H026 empty id/class (Style)
5. Task 8: H024 redundant type attr (Style)

**Wave 2 — Element-level rules (require `Checker.offset_of()` from Task 0):**
6. Task 2: H013 img alt, H005 html lang, H006 img dimensions
7. Task 3: H021 inline styles
8. Task 4: H037 duplicate attributes
9. Task 6: D004 static URLs, D018 url pattern

**Wave 3 — Element & document checks:**
10. Task 9: H036 avoid br
11. Task 10: H020 empty tag pair
12. Task 11: H007 doctype, H016 title

**Wave 4 — Complex/deferred:**
13. Task 12: H023 entity references
14. Task 13: T027, T028, T034, H025

> **Note on offset types:** Wave 1 rules point at attribute *values* which already carry offsets via `NativeAttribute.value: Option<(&str, usize)>`. Wave 2+ rules point at attribute *names* or element tag names, which require the `Checker.offset_of()` helper from Task 0.
