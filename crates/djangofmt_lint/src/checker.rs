use std::path::Path;

use markup_fmt::ast::{
    Attribute, Element, JinjaBlock, JinjaTag, JinjaTagOrChildren, NativeAttribute, Node, NodeKind,
    Root,
};
use miette::SourceSpan;

use crate::LintDiagnostic;
use crate::Settings;
use crate::lint_context::{DiagnosticGuard, LintContext};
use crate::registry::Rule;
use crate::rules;
use crate::violation::Violation;

/// AST visitor that collects lint diagnostics.
pub struct Checker<'a> {
    context: LintContext<'a>,
}

impl<'a> Checker<'a> {
    #[must_use]
    pub const fn new(source: &'a str, settings: &'a Settings, path: Option<&'a Path>) -> Self {
        Self {
            context: LintContext::new(source, settings, path),
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

    /// Returns whether any of the given rules should be checked.
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
            NodeKind::JinjaTag(tag) => self.visit_jinja_tag(tag),
            _ => {}
        }
    }

    fn visit_jinja_tag(&self, tag: &JinjaTag<'_>) {
        if self.is_rule_enabled(Rule::SameFilePartialInclude) {
            rules::style::same_file_partial_include::check(tag, self);
        }
    }

    fn visit_element(&mut self, element: &Element<'_>) {
        if self.is_rule_enabled(Rule::DuplicateAttr) {
            rules::suspicious::duplicate_attr::check(element, self);
        }

        if self.is_rule_enabled(Rule::EmptyTagPair) {
            rules::suspicious::empty_tag_pair::check(element, self);
        }

        if element.tag_name.eq_ignore_ascii_case("img") {
            if self.is_rule_enabled(Rule::MissingImgAlt) {
                rules::accessibility::missing_img_alt::check(element, self);
            }
            if self.is_rule_enabled(Rule::MissingImgDimensions) {
                rules::accessibility::missing_img_dimensions::check(element, self);
            }
        } else if element.tag_name.eq_ignore_ascii_case("html")
            && self.is_rule_enabled(Rule::MissingHtmlLang)
        {
            rules::accessibility::missing_html_lang::check(element, self);
        } else if element.tag_name.eq_ignore_ascii_case("head")
            && self.is_rule_enabled(Rule::MissingTitle)
        {
            rules::accessibility::missing_title::check(element, self);
        }

        for attr in &element.attrs {
            self.visit_attribute(attr, element);
        }

        for child in &element.children {
            self.visit_node(child);
        }
    }

    fn visit_attribute(&mut self, attr: &Attribute<'_>, element: &Element<'_>) {
        match attr {
            Attribute::Native(native) => self.visit_native_attribute(native, element),
            Attribute::JinjaBlock(block) => self.visit_jinja_attr_block(block, element),
            Attribute::JinjaTag(tag) => self.visit_jinja_tag(tag),
            _ => {}
        }
    }

    fn visit_native_attribute(&self, attr: &NativeAttribute<'_>, element: &Element<'_>) {
        if self.is_rule_enabled(Rule::InvalidAttrValue) {
            rules::correctness::invalid_attr_value::check(attr, element, self);
        }

        if self.is_rule_enabled(Rule::EmptyAttrValue) {
            rules::style::empty_attr_value::check(attr, self);
        }

        if self.is_rule_enabled(Rule::RedundantTypeAttr) {
            rules::style::redundant_type_attr::check(attr, element, self);
        }

        if self.is_rule_enabled(Rule::DjangoStaticUrl) {
            rules::suspicious::django_static_url::check(attr, element, self);
        }

        if self.is_rule_enabled(Rule::DjangoUrlPattern) {
            rules::suspicious::django_url_pattern::check(attr, element, self);
        }

        if self.is_rule_enabled(Rule::JavascriptUrl) {
            rules::suspicious::javascript_url::check(attr, element, self);
        }

        if self.is_rule_enabled(Rule::UseHttps) {
            rules::suspicious::use_https::check(attr, self);
        }

        if element.tag_name.eq_ignore_ascii_case("form") {
            if self.is_rule_enabled(Rule::UppercaseFormMethod) {
                rules::style::uppercase_form_method::check(attr, self);
            }
            if self.is_rule_enabled(Rule::FormActionWhitespace) {
                rules::style::form_action_whitespace::check(attr, self);
            }
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

    fn visit_jinja_attr_block(
        &mut self,
        block: &JinjaBlock<'_, Attribute<'_>>,
        element: &Element<'_>,
    ) {
        for item in &block.body {
            if let JinjaTagOrChildren::Children(children) = item {
                for child in children {
                    self.visit_attribute(child, element);
                }
            }
        }
    }
}
