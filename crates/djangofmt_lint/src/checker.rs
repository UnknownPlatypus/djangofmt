use markup_fmt::ast::{Attribute, Element, JinjaBlock, JinjaTagOrChildren, Node, NodeKind, Root};
use miette::SourceSpan;

use crate::LintDiagnostic;
use crate::Settings;
use crate::lint_context::{DiagnosticGuard, LintContext};
use crate::registry::Rule;
use crate::rules;
use crate::rules::helpers::declares_native_attr;
use crate::violation::Violation;

/// One-time classification of an element's tag, computed once per element and
/// reused by every cluster instead of each rule re-folding `tag_name` with its
/// own `eq_ignore_ascii_case` chain.
// Each field is an independent tag category, so a few bools is the natural shape.
#[allow(clippy::struct_excessive_bools)]
struct TagKind {
    is_form: bool,
    /// HTML5 default `type` value for `<script>`/`<style>`, else `None`.
    redundant_type_default: Option<&'static str>,
    is_asset: bool,
    /// `javascript:`-checked URL attribute names for this tag (possibly empty).
    js_url_attrs: &'static [&'static str],
    is_html: bool,
    is_img: bool,
}

impl TagKind {
    const fn classify(tag_name: &str) -> Self {
        Self {
            is_form: tag_name.eq_ignore_ascii_case("form"),
            redundant_type_default: rules::style::redundant_type_attr::default_type_for(tag_name),
            is_asset: rules::suspicious::django_static_url::is_asset_tag(tag_name),
            js_url_attrs: rules::suspicious::javascript_url::url_attributes_for(tag_name),
            is_html: tag_name.eq_ignore_ascii_case("html"),
            is_img: tag_name.eq_ignore_ascii_case("img"),
        }
    }
}

/// Presence of attributes the accessibility "missing attribute" rules care
/// about, accumulated during the single attribute pass (native or
/// Jinja-declared).
// One independent presence flag per tracked attribute.
#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
struct AttrPresence {
    lang: bool,
    alt: bool,
    height: bool,
    width: bool,
}

/// Case-insensitively resolve `name` to its canonical spelling within
/// `candidates`, returning the `'static` canonical name or `None`.
fn canonical_match(candidates: &'static [&'static str], name: &str) -> Option<&'static str> {
    candidates
        .iter()
        .copied()
        .find(|candidate| candidate.eq_ignore_ascii_case(name))
}

/// Rules that inspect a single native attribute's value. Gated as one cluster
/// before the single attribute pass drills into individual attributes.
const VALUE_ATTR_RULES: &[Rule] = &[
    Rule::InvalidAttrValue,
    Rule::EmptyAttrValue,
    Rule::RedundantTypeAttr,
    Rule::DjangoStaticUrl,
    Rule::JavascriptUrl,
    Rule::UseHttps,
    Rule::UppercaseFormMethod,
    Rule::FormActionWhitespace,
];

/// Accessibility rules that report on a *missing* attribute, keyed on tag
/// presence. Gated as one cluster.
const ATTR_PRESENCE_RULES: &[Rule] = &[
    Rule::MissingHtmlLang,
    Rule::MissingImgAlt,
    Rule::MissingImgDimensions,
];

/// AST visitor that collects lint diagnostics.
pub struct Checker<'a> {
    context: LintContext<'a>,
}

impl<'a> Checker<'a> {
    #[must_use]
    pub const fn new(source: &'a str, settings: &'a Settings) -> Self {
        Self {
            context: LintContext::new(source, settings),
        }
    }

    /// Borrow the underlying [`LintContext`].
    #[must_use]
    pub const fn context(&self) -> &LintContext<'a> {
        &self.context
    }

    /// Compute the byte offset of a string slice within the source.
    ///
    /// This is used to convert AST `raw` slices into [`SourceSpan`] offsets.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is not fully contained within the source.
    #[must_use]
    pub fn source_offset(&self, slice: &str) -> usize {
        self.context.source_offset(slice)
    }

    /// Returns whether the given rule should be checked.
    #[must_use]
    #[inline]
    pub const fn is_rule_enabled(&self, rule: Rule) -> bool {
        self.context.is_rule_enabled(rule)
    }

    /// Returns whether any rule in `rules` is enabled. Cheap cluster pre-filter
    /// delegating to [`LintContext::any_rule_enabled`].
    #[must_use]
    #[inline]
    pub const fn any_rule_enabled(&self, rules: &[Rule]) -> bool {
        self.context.any_rule_enabled(rules)
    }

    /// Report a diagnostic for a rule the caller has already gated on
    /// [`Self::is_rule_enabled`]. Returns a guard whose Drop pushes the
    /// diagnostic into the underlying context.
    pub fn report_diagnostic<V: Violation>(
        &self,
        violation: &V,
        span: SourceSpan,
    ) -> DiagnosticGuard<'_, 'a> {
        self.context.report_diagnostic(violation, span)
    }

    /// Report a diagnostic only if the rule is enabled. Returns `None`
    /// otherwise.
    pub fn report_diagnostic_if_enabled<V: Violation>(
        &self,
        violation: &V,
        span: SourceSpan,
    ) -> Option<DiagnosticGuard<'_, 'a>> {
        self.context.report_diagnostic_if_enabled(violation, span)
    }

    /// Consume the checker and return all collected diagnostics.
    #[must_use]
    pub fn into_diagnostics(self) -> Vec<LintDiagnostic> {
        self.context.into_diagnostics()
    }

    /// Visit the root of the AST and run all lint rules.
    pub fn visit_root(&mut self, root: &Root<'_>) {
        if self.is_rule_enabled(Rule::MissingDoctype) {
            rules::style::missing_doctype::check(root, self);
        }

        for node in &root.children {
            self.visit_node(node);
        }
    }

    fn visit_node(&mut self, node: &Node<'_>) {
        match &node.kind {
            NodeKind::Element(element) => self.visit_element(element),
            NodeKind::JinjaBlock(block) => self.visit_jinja_block(block),
            _ => {}
        }
    }

    fn visit_element(&mut self, element: &Element<'_>) {
        // (a) Classify the tag once; every cluster reuses this instead of
        // re-folding `tag_name`.
        let tag = TagKind::classify(element.tag_name);

        // (c) Single attribute pass: emits the value-cluster + duplicate
        // diagnostics and returns attribute presence for the accessibility
        // rules, which report afterwards.
        let presence = self.analyze_element_attrs(element, &tag);

        // The remaining diagnostics keep the original emission order so the
        // recorded snapshots stay byte-identical:
        //   value/duplicate rules (above) → EmptyTagPair → presence rules →
        //   MissingTitle.
        if self.is_rule_enabled(Rule::EmptyTagPair) {
            rules::suspicious::empty_tag_pair::check(element, self);
        }
        if tag.is_html && self.is_rule_enabled(Rule::MissingHtmlLang) {
            rules::accessibility::missing_html_lang::report_if_missing(
                self,
                element,
                presence.lang,
            );
        }
        if self.is_rule_enabled(Rule::MissingTitle) {
            rules::accessibility::missing_title::check(element, self);
        }
        if tag.is_img {
            if self.is_rule_enabled(Rule::MissingImgAlt) {
                rules::accessibility::missing_img_alt::report_if_missing(
                    self,
                    element,
                    presence.alt,
                );
            }
            if self.is_rule_enabled(Rule::MissingImgDimensions) {
                rules::accessibility::missing_img_dimensions::report_if_missing(
                    self,
                    element,
                    presence.height,
                    presence.width,
                );
            }
        }

        for attr in &element.attrs {
            self.visit_attribute(attr);
        }

        for child in &element.children {
            self.visit_node(child);
        }
    }

    /// (b)+(c) Cluster-gate, then walk `element.attrs` a single time, dispatching
    /// each native attribute to the value rules interested in its name + tag, and
    /// tracking attribute presence for the accessibility "missing attribute"
    /// rules in the same pass. Returns the presence flags; the caller reports the
    /// presence rules afterwards to preserve diagnostic order.
    fn analyze_element_attrs(&self, element: &Element<'_>, tag: &TagKind) -> AttrPresence {
        let mut presence = AttrPresence::default();

        // (b) Cluster pre-filters: one bitset OR each. A disabled cluster skips
        // its whole branch.
        let value_cluster = self.any_rule_enabled(VALUE_ATTR_RULES);
        let duplicate = self.is_rule_enabled(Rule::DuplicateAttr);

        // The presence cluster only matters on the tags those rules target.
        let presence_relevant =
            (tag.is_html || tag.is_img) && self.any_rule_enabled(ATTR_PRESENCE_RULES);

        // Whether any work in the attribute pass is needed at all.
        if !value_cluster && !duplicate && !presence_relevant {
            return presence;
        }

        for (i, attr) in element.attrs.iter().enumerate() {
            // Presence tracking sees every attribute kind (native + Jinja
            // blocks), matching `declares_native_attr`'s recursion.
            if presence_relevant {
                if tag.is_html {
                    presence.lang = presence.lang || declares_native_attr(attr, "lang");
                } else if tag.is_img {
                    presence.alt = presence.alt || declares_native_attr(attr, "alt");
                    presence.height = presence.height || declares_native_attr(attr, "height");
                    presence.width = presence.width || declares_native_attr(attr, "width");
                }
            }

            // Value rules and duplicate detection only act on native attributes.
            let Attribute::Native(native) = attr else {
                continue;
            };
            let name = native.name;

            if duplicate {
                rules::suspicious::duplicate_attr::check_attr(self, &element.attrs[..i], name);
            }

            if value_cluster && let Some((value_str, offset)) = native.value {
                self.dispatch_value_rules(
                    element.tag_name,
                    tag,
                    name,
                    value_str,
                    offset,
                    native.quote,
                );
            }
        }

        presence
    }

    /// Dispatch one pre-destructured native attribute (`name`, `value_str`,
    /// `offset`, `quote`) to the value rules interested in it, using the
    /// already-computed [`TagKind`]. Each rule stays gated on its own `Rule` bit.
    fn dispatch_value_rules(
        &self,
        tag_name: &str,
        tag: &TagKind,
        name: &str,
        value_str: &str,
        offset: usize,
        quote: Option<char>,
    ) {
        // id/class empty values — any tag.
        if (name.eq_ignore_ascii_case("id") || name.eq_ignore_ascii_case("class"))
            && self.is_rule_enabled(Rule::EmptyAttrValue)
        {
            rules::style::empty_attr_value::check_attr(self, name, value_str, offset, quote);
        }

        // <form method> — invalid value + uppercase.
        if tag.is_form && name.eq_ignore_ascii_case("method") {
            if self.is_rule_enabled(Rule::InvalidAttrValue) {
                rules::correctness::invalid_attr_value::check_method_attr(self, value_str, offset);
            }
            if self.is_rule_enabled(Rule::UppercaseFormMethod) {
                rules::style::uppercase_form_method::check_attr(self, value_str, offset);
            }
        }

        // <form action> — whitespace.
        if tag.is_form
            && name.eq_ignore_ascii_case("action")
            && self.is_rule_enabled(Rule::FormActionWhitespace)
        {
            rules::style::form_action_whitespace::check_attr(self, value_str, offset);
        }

        // <script>/<style> type — redundant default.
        if let Some(default_type) = tag.redundant_type_default
            && name.eq_ignore_ascii_case("type")
            && self.is_rule_enabled(Rule::RedundantTypeAttr)
        {
            rules::style::redundant_type_attr::check_attr(
                self,
                tag_name,
                default_type,
                name,
                value_str,
                offset,
                quote,
            );
        }

        // Asset-tag static URLs — href/src/srcset.
        if tag.is_asset
            && self.is_rule_enabled(Rule::DjangoStaticUrl)
            && let Some(canonical) =
                canonical_match(rules::suspicious::django_static_url::URL_ATTRIBUTES, name)
        {
            rules::suspicious::django_static_url::check_attr(self, canonical, value_str, offset);
        }

        // `javascript:` URLs — tag-specific attribute set.
        if !tag.js_url_attrs.is_empty()
            && self.is_rule_enabled(Rule::JavascriptUrl)
            && let Some(canonical) = canonical_match(tag.js_url_attrs, name)
        {
            rules::suspicious::javascript_url::check_attr(self, canonical, value_str, offset);
        }

        // `http://` URLs — any URL-bearing attribute, any tag.
        if self.is_rule_enabled(Rule::UseHttps)
            && let Some(canonical) = rules::suspicious::use_https::canonical_url_attr(name)
        {
            rules::suspicious::use_https::check_attr(self, canonical, value_str, offset);
        }
    }

    fn visit_attribute(&mut self, attr: &Attribute<'_>) {
        if let Attribute::JinjaBlock(block) = attr {
            self.visit_jinja_attr_block(block);
        }
    }

    fn visit_jinja_block(&mut self, block: &JinjaBlock<'_, Node<'_>>) {
        if self.is_rule_enabled(Rule::UntrimmedBlocktranslate) {
            rules::correctness::untrimmed_blocktranslate::check(block, self);
        }

        for item in &block.body {
            if let JinjaTagOrChildren::Children(children) = item {
                for child in children {
                    self.visit_node(child);
                }
            }
        }
    }

    fn visit_jinja_attr_block(&mut self, block: &JinjaBlock<'_, Attribute<'_>>) {
        for item in &block.body {
            if let JinjaTagOrChildren::Children(children) = item {
                for child in children {
                    self.visit_attribute(child);
                }
            }
        }
    }
}
