use std::path::Path;

use markup_fmt::Language;
use markup_fmt::ast::{
    Attribute, Element, JinjaBlock, JinjaInterpolation, JinjaTag, JinjaTagOrChildren,
    NativeAttribute, Node, NodeKind, Root,
};
use markup_fmt::parser::parse_jinja_tag_name;
use miette::SourceSpan;
use smallvec::SmallVec;

use crate::LintDiagnostic;
use crate::Settings;
use crate::django_version::DjangoVersion;
use crate::lint_context::{DiagnosticGuard, LintContext};
use crate::registry::Rule;
use crate::rules;
use crate::suppression::IgnoreComment;
use crate::violation::Violation;
use djangofmt_syntax::{CommentDelimiters, HTML_COMMENT, TEMPLATE_COMMENT};

/// The rules that must not read what a Jinja `{% raw %}` body contains.
const RAW_SENSITIVE_RULES: &[Rule] = &[
    Rule::DuplicateBlockName,
    Rule::SameFilePartialInclude,
    Rule::UntrimmedBlocktranslate,
];

/// Every rule `visit_jinja_tag` runs: text is only scanned for tags when one is enabled.
const TAG_RULES: &[Rule] = &[
    Rule::DuplicateBlockName,
    Rule::SameFilePartialInclude,
    Rule::DeprecatedStaticLibrary,
    Rule::LegacyTranslationTag,
];

/// Where a tag reaching [`Checker::visit_jinja_tag`] comes from.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TagOrigin {
    Ast,
    /// Text the parser keeps verbatim but Django still compiles: an HTML comment, a raw-text body
    /// or an attribute value.
    Text,
}

/// AST visitor that collects lint diagnostics.
pub struct Checker<'a> {
    context: LintContext<'a>,
    /// Block names collected during the traversal.
    /// Inline-backed: templates rarely exceed a handful of blocks, so the common case never allocates.
    block_names: SmallVec<[&'a str; 8]>,
    /// Inside a Jinja `{% raw %}` body, where template tags are literal text.
    in_raw: bool,
    /// Whether a rule reads the tags in text, which is only scanned for them then.
    reads_text_tags: bool,
    /// Where the last search for a `{%` started, and the `{%` it found (`usize::MAX` for none).
    tag_search_from: usize,
    next_tag_open: usize,
}

impl<'a> Checker<'a> {
    #[must_use]
    pub const fn new(
        source: &'a str,
        settings: &'a Settings,
        language: Language,
        path: Option<&'a Path>,
        quarantined: bool,
    ) -> Self {
        Self {
            context: LintContext::new(source, settings, language, path, quarantined),
            block_names: SmallVec::new_const(),
            in_raw: false,
            reads_text_tags: settings.rules.contains_any(TAG_RULES),
            tag_search_from: usize::MAX,
            next_tag_open: usize::MAX,
        }
    }

    /// Borrow the underlying [`LintContext`].
    #[must_use]
    pub const fn context(&self) -> &LintContext<'a> {
        &self.context
    }

    /// Whether the source is parsed as a Django template rather than a Jinja one.
    #[must_use]
    pub const fn is_django(&self) -> bool {
        matches!(self.context.language(), Language::Django)
    }

    /// The Django version the templates target, [`None`] when it is neither set nor inferred.
    #[must_use]
    pub const fn target_version(&self) -> Option<DjangoVersion> {
        self.context.settings().target_version
    }

    /// Block names recorded during the traversal, borrowed from the source.
    #[must_use]
    pub fn block_names(&self) -> &[&'a str] {
        &self.block_names
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

    /// The byte offset just past `slice` within the source.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is not fully contained within the source.
    #[must_use]
    pub fn source_end(&self, slice: &str) -> usize {
        self.context.source_end(slice)
    }

    /// The span of `slice` within the source.
    ///
    /// # Panics
    ///
    /// Panics if `slice` is not fully contained within the source.
    #[must_use]
    pub fn source_span(&self, slice: &str) -> SourceSpan {
        self.context.source_span(slice)
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
            rules::style::missing_doctype::check(self, root);
        }

        for node in &root.children {
            self.visit_node(node);
        }

        // Cross-node finalize: now that every `{% block %}` has been recorded, flag duplicates.
        if self.is_rule_enabled(Rule::DuplicateBlockName) {
            rules::correctness::duplicate_block_name::check(self);
        }
    }

    /// Lint the ignore comments themselves.
    pub fn visit_ignore_comments(&self, comments: &[IgnoreComment<'_>]) {
        if self.is_rule_enabled(Rule::InvalidIgnoreComment) {
            rules::suspicious::invalid_ignore_comment::check(self, comments);
        }
        if self.is_rule_enabled(Rule::InvalidIgnoreCode) {
            rules::suspicious::invalid_ignore_code::check(self, comments);
        }
    }

    /// Report the codes that silence nothing, once each comment knows what it matched.
    pub fn visit_unused_ignore_codes(&self, comments: &[IgnoreComment<'_>]) {
        if self.is_rule_enabled(Rule::UnusedIgnoreCode) {
            rules::suspicious::unused_ignore_code::check(self, comments);
        }
    }

    fn visit_node(&mut self, node: &Node<'a>) {
        match &node.kind {
            NodeKind::Element(element) => self.visit_element(element),
            NodeKind::JinjaBlock(block) => self.visit_jinja_block(block),
            NodeKind::JinjaTag(tag) => self.visit_jinja_tag(tag, TagOrigin::Ast),
            NodeKind::Comment(comment) => {
                self.visit_comment(HTML_COMMENT, comment.raw);
                if self.reads_text_tags {
                    self.visit_text_tags(comment.raw);
                }
            }
            NodeKind::JinjaComment(comment) => self.visit_comment(TEMPLATE_COMMENT, comment.raw),
            NodeKind::JinjaInterpolation(interpolation) => {
                self.visit_jinja_interpolation(interpolation);
            }
            _ => {}
        }
    }

    /// Only node-position `{{ }}` reaches here: in an attribute value it stays part of the
    /// value string.
    fn visit_jinja_interpolation(&self, interpolation: &JinjaInterpolation<'_>) {
        // No version-gated rule can fire without a target version.
        if let Some(version) = self.target_version()
            && self.is_rule_enabled(Rule::RedundantJsonScriptId)
        {
            rules::upgrade::redundant_json_script_id::check(self, interpolation, version);
        }
    }

    fn visit_comment(&self, delimiters: CommentDelimiters, body: &str) {
        if self.is_rule_enabled(Rule::DeprecatedIgnore) {
            rules::suspicious::deprecated_ignore::check(self, delimiters, body);
        }
    }

    fn visit_jinja_tag(&mut self, tag: &JinjaTag<'a>, origin: TagOrigin) {
        if let Some(version) = self.target_version()
            && self.is_rule_enabled(Rule::LegacyTranslationTag)
        {
            rules::upgrade::legacy_translation_tag::check_tag(self, tag, origin, version);
        }
        // `visit_jinja_block` records the blocks of the AST; a tag in text has no block around it.
        if origin == TagOrigin::Text && self.is_rule_enabled(Rule::DuplicateBlockName) {
            self.record_text_block_name(tag.content);
        }
        // Same-file detection needs the linted file's path (absent in e.g. the WASM playground).
        if let Some(path) = self.context.path()
            && !self.in_raw
            && self.is_rule_enabled(Rule::SameFilePartialInclude)
        {
            rules::style::same_file_partial_include::check(self, tag, path);
        }
        // No version-gated rule can fire without a target version.
        if let Some(version) = self.target_version()
            && self.is_rule_enabled(Rule::DeprecatedStaticLibrary)
        {
            rules::upgrade::deprecated_static_library::check(self, tag, version);
        }
    }

    fn visit_element(&mut self, element: &Element<'a>) {
        if self.is_rule_enabled(Rule::DuplicateAttr) {
            rules::suspicious::duplicate_attr::check(self, element);
        }

        if self.is_rule_enabled(Rule::EmptyTagPair) {
            rules::pedantic::empty_tag_pair::check(self, element);
        }

        if element.tag_name.eq_ignore_ascii_case("img") {
            if self.is_rule_enabled(Rule::MissingImgAlt) {
                rules::accessibility::missing_img_alt::check(self, element);
            }
            if self.is_rule_enabled(Rule::MissingImgDimensions) {
                rules::pedantic::missing_img_dimensions::check(self, element);
            }
        } else if element.tag_name.eq_ignore_ascii_case("html") {
            if self.is_rule_enabled(Rule::MissingHtmlLang) {
                rules::accessibility::missing_html_lang::check(self, element);
            }
            if self.is_rule_enabled(Rule::MissingTitle) {
                rules::accessibility::missing_title::check_html(self, element);
            }
        } else if element.tag_name.eq_ignore_ascii_case("head")
            && self.is_rule_enabled(Rule::MissingTitle)
        {
            rules::accessibility::missing_title::check(self, element);
        } else if element.tag_name.eq_ignore_ascii_case("th")
            && self.is_rule_enabled(Rule::TableHeaderMissingScope)
        {
            rules::pedantic::table_header_missing_scope::check(self, element);
        }

        for attr in &element.attrs {
            self.visit_attribute(attr, element);
        }

        for child in &element.children {
            self.visit_node(child);
        }

        // The parser keeps these bodies as raw text, but Django still reads the tags in them.
        if self.reads_text_tags
            && ["script", "style", "pre", "textarea"]
                .iter()
                .any(|tag| element.tag_name.eq_ignore_ascii_case(tag))
        {
            for child in &element.children {
                if let NodeKind::Text(text) = &child.kind {
                    self.visit_text_tags(text.raw);
                }
            }
        }
    }

    fn visit_attribute(&mut self, attr: &Attribute<'a>, element: &Element<'a>) {
        match attr {
            Attribute::Native(native) => {
                self.visit_native_attribute(native, element);
                if self.reads_text_tags
                    && let Some((value, _)) = native.value
                {
                    self.visit_text_tags(value);
                }
            }
            Attribute::JinjaBlock(block) => self.visit_jinja_attr_block(block, element),
            Attribute::JinjaTag(tag) => self.visit_jinja_tag(tag, TagOrigin::Ast),
            _ => {}
        }
    }

    fn visit_native_attribute(&self, attr: &NativeAttribute<'a>, element: &Element<'a>) {
        if self.is_rule_enabled(Rule::InvalidAttrValue) {
            rules::correctness::invalid_attr_value::check(self, attr, element);
        }

        if self.is_rule_enabled(Rule::EmptyAttrValue) {
            rules::style::empty_attr_value::check(self, attr, element);
        }

        if self.is_rule_enabled(Rule::RedundantTypeAttr) {
            rules::style::redundant_type_attr::check(self, attr, element);
        }

        if self.is_rule_enabled(Rule::DjangoStaticUrl) {
            rules::suspicious::django_static_url::check(self, attr, element);
        }

        if self.is_rule_enabled(Rule::DjangoUrlPattern) {
            rules::suspicious::django_url_pattern::check(self, attr, element);
        }

        if self.is_rule_enabled(Rule::JavascriptUrl) {
            rules::suspicious::javascript_url::check(self, attr, element);
        }

        if self.is_rule_enabled(Rule::UseHttps) {
            rules::suspicious::use_https::check(self, attr);
        }

        if element.tag_name.eq_ignore_ascii_case("form") {
            if self.is_rule_enabled(Rule::UppercaseFormMethod) {
                rules::style::uppercase_form_method::check(self, attr);
            }
            if self.is_rule_enabled(Rule::FormActionWhitespace) {
                rules::style::form_action_whitespace::check(self, attr);
            }
        }

        if self.is_rule_enabled(Rule::UnsortedTailwindClasses)
            && attr.name.eq_ignore_ascii_case("class")
        {
            rules::pedantic::unsorted_tailwind_classes::check(self, attr);
        }
    }

    fn visit_jinja_block(&mut self, block: &JinjaBlock<'a, Node<'a>>) {
        if let Some(version) = self.target_version()
            && self.is_rule_enabled(Rule::LegacyTranslationTag)
        {
            rules::upgrade::legacy_translation_tag::check_block(self, block, version);
        }
        if !self.in_raw {
            if self.is_rule_enabled(Rule::UntrimmedBlocktranslate) {
                rules::correctness::untrimmed_blocktranslate::check(self, block);
            }
            if self.is_rule_enabled(Rule::DuplicateBlockName) {
                self.record_block_name(block);
            }
        }

        self.check_block_end(block);

        let outer_raw = self.in_raw;
        self.in_raw |= self.opens_raw(block);
        for item in &block.body {
            match item {
                JinjaTagOrChildren::Tag(tag) => self.visit_jinja_tag(tag, TagOrigin::Ast),
                JinjaTagOrChildren::Children(children) => {
                    for child in children {
                        self.visit_node(child);
                    }
                }
            }
        }
        self.in_raw = outer_raw;
    }

    fn check_block_end<T>(&self, block: &JinjaBlock<'a, T>) {
        if self.is_django()
            && self.any_rule_enabled(&[Rule::MissingEndblockLabel, Rule::RedundantEndblockLabel])
            && let Some(end) = rules::helpers::block_end(self, block)
        {
            if self.is_rule_enabled(Rule::MissingEndblockLabel) {
                rules::style::missing_endblock_label::check(self, &end);
            }
            if self.is_rule_enabled(Rule::RedundantEndblockLabel) {
                rules::style::redundant_endblock_label::check(self, &end);
            }
        }
    }

    /// Whether `block` opens a `{% raw %}` body, which emits its contents verbatim: the HTML
    /// inside is real, the template tags are not. Django has no `raw` tag — only Jinja parses one
    /// into a block — and reading the name costs a scan, so only look when a rule needs the answer.
    fn opens_raw<T>(&self, block: &JinjaBlock<'a, T>) -> bool {
        !self.is_django()
            && self.any_rule_enabled(RAW_SENSITIVE_RULES)
            && matches!(
                block.body.first(),
                Some(JinjaTagOrChildren::Tag(tag)) if parse_jinja_tag_name(tag, self.context.language()) == "raw"
            )
    }

    fn record_block_name<T>(&mut self, block: &JinjaBlock<'a, T>) {
        if let Some(name) = rules::correctness::duplicate_block_name::block_name(block) {
            self.block_names.push(name);
        }
    }

    #[cold]
    fn record_text_block_name(&mut self, content: &'a str) {
        if let Some(name) =
            rules::correctness::duplicate_block_name::block_name_from_content(content)
        {
            self.block_names.push(name);
        }
    }

    /// Run the tag rules on the `{% %}` in `text`. Callers check `reads_text_tags` first, so that
    /// a run without such rules walks the same code as before.
    // Out of line, with the common case making no call: inlined or not, a heavier body would tax
    // every attribute.
    #[inline(never)]
    fn visit_text_tags(&mut self, text: &'a str) {
        let start = self.source_offset(text);
        // The visitor walks the source in order, so one search answers for every text before the
        // `{%` it finds.
        if start < self.tag_search_from || self.next_tag_open < start {
            self.search_tag_open(text, start);
        } else if self.next_tag_open < start + text.len() {
            self.scan_text_tags(text, start);
        }
    }

    #[inline(never)]
    fn search_tag_open(&mut self, text: &'a str, start: usize) {
        self.tag_search_from = start;
        self.next_tag_open = rules::helpers::find_tag_open(&self.context.source()[start..])
            .map_or(usize::MAX, |found| start + found);
        if self.next_tag_open < start + text.len() {
            self.scan_text_tags(text, start);
        }
    }

    #[inline(never)]
    fn scan_text_tags(&mut self, text: &'a str, start: usize) {
        if self.in_raw {
            return;
        }
        for tag in rules::helpers::tags_in_text(text) {
            let tag = JinjaTag {
                start: start + tag.start,
                ..tag
            };
            self.visit_jinja_tag(&tag, TagOrigin::Text);
        }
    }

    fn visit_jinja_attr_block(
        &mut self,
        block: &JinjaBlock<'a, Attribute<'a>>,
        element: &Element<'a>,
    ) {
        if let Some(version) = self.target_version()
            && self.is_rule_enabled(Rule::LegacyTranslationTag)
        {
            rules::upgrade::legacy_translation_tag::check_block(self, block, version);
        }
        if !self.in_raw && self.is_rule_enabled(Rule::DuplicateBlockName) {
            self.record_block_name(block);
        }
        self.check_block_end(block);

        let outer_raw = self.in_raw;
        self.in_raw |= self.opens_raw(block);
        for item in &block.body {
            match item {
                JinjaTagOrChildren::Tag(tag) => self.visit_jinja_tag(tag, TagOrigin::Ast),
                JinjaTagOrChildren::Children(children) => {
                    for child in children {
                        self.visit_attribute(child, element);
                    }
                }
            }
        }
        self.in_raw = outer_raw;
    }
}
