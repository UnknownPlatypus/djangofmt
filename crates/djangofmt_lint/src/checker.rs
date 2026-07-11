use markup_fmt::ast::{
    Attribute, Element, JinjaBlock, JinjaTagOrChildren, NativeAttribute, Node, NodeKind, Root,
};
use markup_fmt::parser::parse_jinja_tag_name;
use miette::SourceSpan;
use smallvec::SmallVec;

use crate::LintDiagnostic;
use crate::Settings;
use crate::lint_context::{DiagnosticGuard, LintContext};
use crate::registry::Rule;
use crate::rules;
use crate::violation::Violation;

bitflags::bitflags! {
    /// Semantic context of the current traversal position, queryable by any
    /// rule through [`Checker::flags`]. Flags are set when entering a construct
    /// and restored from a snapshot when leaving, so nesting needs no counters.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ContextFlags: u8 {
        /// Inside a raw-text element (`<script>`, `<style>`, `<pre>`, `<textarea>`).
        const RAW_TEXT_ELEMENT = 1 << 0;
        /// Inside a `{% blocktranslate %}` / `{% blocktrans %}` body.
        const TRANSLATED_BLOCK = 1 << 1;
        /// Inside a `{% comment %}` body.
        const COMMENT_BLOCK = 1 << 2;
        /// Inside a `{% verbatim %}` body.
        const VERBATIM_BLOCK = 1 << 3;
    }
}

/// AST visitor that collects lint diagnostics.
pub struct Checker<'a> {
    context: LintContext<'a>,
    /// Block names collected during the traversal.
    /// Blocks in attribute position (`<div {% block x %}…>`) are not visited here, so are not recorded.
    /// Inline-backed: templates rarely exceed a handful of blocks, so the common case never allocates.
    block_names: SmallVec<[&'a str; 8]>,
    /// Semantic context of the current traversal position (see [`ContextFlags`]).
    flags: ContextFlags,
}

impl<'a> Checker<'a> {
    #[must_use]
    pub const fn new(source: &'a str, settings: &'a Settings) -> Self {
        Self {
            context: LintContext::new(source, settings),
            block_names: SmallVec::new_const(),
            flags: ContextFlags::empty(),
        }
    }

    /// Borrow the underlying [`LintContext`].
    #[must_use]
    pub const fn context(&self) -> &LintContext<'a> {
        &self.context
    }

    /// Block names recorded during the traversal, borrowed from the source.
    #[must_use]
    pub fn block_names(&self) -> &[&'a str] {
        &self.block_names
    }

    /// Semantic context flags at the current traversal position.
    #[must_use]
    pub const fn flags(&self) -> ContextFlags {
        self.flags
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
    pub fn visit_root(&mut self, root: &Root<'a>) {
        if self.is_rule_enabled(Rule::MissingDoctype) {
            rules::style::missing_doctype::check(root, self);
        }

        self.visit_children(&root.children);

        // Cross-node finalize: now that every `{% block %}` has been recorded, flag duplicates.
        if self.is_rule_enabled(Rule::DuplicateBlockName) {
            rules::correctness::duplicate_block_name::check(self);
        }
    }

    fn visit_node(&mut self, node: &Node<'a>) {
        match &node.kind {
            NodeKind::Element(element) => self.visit_element(element),
            NodeKind::JinjaBlock(block) => self.visit_jinja_block(block),
            _ => {}
        }
    }

    /// Visit a group of sibling nodes, running sibling-scoped rules before recursing.
    fn visit_children(&mut self, children: &[Node<'a>]) {
        if self.is_rule_enabled(Rule::UntranslatedText) {
            rules::i18n::untranslated_text::check_text(children, self);
        }

        for child in children {
            self.visit_node(child);
        }
    }

    fn visit_element(&mut self, element: &Element<'a>) {
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

        let snapshot = self.flags;
        if is_raw_text_element(element.tag_name) {
            self.flags |= ContextFlags::RAW_TEXT_ELEMENT;
        }
        self.visit_children(&element.children);
        self.flags = snapshot;
    }

    fn visit_attribute(&mut self, attr: &Attribute<'a>, element: &Element<'a>) {
        match attr {
            Attribute::Native(native) => self.visit_native_attribute(native, element),
            Attribute::JinjaBlock(block) => self.visit_jinja_attr_block(block, element),
            _ => {}
        }
    }

    fn visit_native_attribute(&self, attr: &NativeAttribute<'a>, element: &Element<'a>) {
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

        if self.is_rule_enabled(Rule::UntranslatedText) {
            rules::i18n::untranslated_text::check_attr(attr, element, self);
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

    fn visit_jinja_block(&mut self, block: &JinjaBlock<'a, Node<'a>>) {
        if self.is_rule_enabled(Rule::UntrimmedBlocktranslate) {
            rules::correctness::untrimmed_blocktranslate::check(block, self);
        }

        if self.is_rule_enabled(Rule::DuplicateBlockName) {
            self.record_block_name(block);
        }

        let snapshot = self.flags;
        self.flags |= block_context_flags(&block.body);
        for item in &block.body {
            if let JinjaTagOrChildren::Children(children) = item {
                self.visit_children(children);
            }
        }
        self.flags = snapshot;
    }

    fn record_block_name(&mut self, block: &JinjaBlock<'a, Node<'a>>) {
        if let Some(name) = rules::correctness::duplicate_block_name::block_name(block) {
            self.block_names.push(name);
        }
    }

    fn visit_jinja_attr_block(
        &mut self,
        block: &JinjaBlock<'a, Attribute<'a>>,
        element: &Element<'a>,
    ) {
        let snapshot = self.flags;
        self.flags |= block_context_flags(&block.body);
        for item in &block.body {
            if let JinjaTagOrChildren::Children(children) = item {
                for child in children {
                    self.visit_attribute(child, element);
                }
            }
        }
        self.flags = snapshot;
    }
}

/// Elements whose content the parser captures as a single raw text node.
fn is_raw_text_element(tag_name: &str) -> bool {
    ["script", "style", "pre", "textarea"]
        .iter()
        .any(|tag| tag_name.eq_ignore_ascii_case(tag))
}

/// Context flags contributed by a Jinja block, keyed on its opening tag name.
fn block_context_flags<T>(body: &[JinjaTagOrChildren<'_, T>]) -> ContextFlags {
    let Some(JinjaTagOrChildren::Tag(open_tag)) = body.first() else {
        return ContextFlags::empty();
    };
    match parse_jinja_tag_name(open_tag) {
        "blocktranslate" | "blocktrans" => ContextFlags::TRANSLATED_BLOCK,
        "comment" => ContextFlags::COMMENT_BLOCK,
        "verbatim" => ContextFlags::VERBATIM_BLOCK,
        _ => ContextFlags::empty(),
    }
}
