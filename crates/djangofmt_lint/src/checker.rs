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
use crate::rules::upgrade::deprecated_ifequal_tag::EqualityTag;
use crate::suppression::IgnoreComment;
use crate::violation::Violation;
use djangofmt_syntax::dtl;
use djangofmt_syntax::{CommentDelimiters, HTML_COMMENT, TEMPLATE_COMMENT};

/// Every rule an arm of `visit_tag_named` or `visit_block_opener` reaches: a tag's name is only
/// read, and text only scanned for tags, when one is enabled. A new arm adds its rules here.
const TAG_RULES: &[Rule] = &[
    Rule::DuplicateBlockName,
    Rule::SameFilePartialInclude,
    Rule::UntrimmedBlocktranslate,
    Rule::DeprecatedStaticLibrary,
    Rule::LegacyTranslationTag,
    Rule::LegacyAsAssignment,
    Rule::DeprecatedIfequalTag,
    Rule::DeprecatedLengthIsFilter,
];

/// AST visitor that collects lint diagnostics.
pub struct Checker<'a> {
    context: LintContext<'a>,
    /// Block names collected during the traversal.
    /// Inline-backed: templates rarely exceed a handful of blocks, so the common case never allocates.
    block_names: SmallVec<[&'a str; 8]>,
    /// Inside a Jinja `{% raw %}` body, where template tags are literal text.
    in_raw: bool,
    /// Whether a rule in `TAG_RULES` is enabled.
    reads_tags: bool,
    /// Whether a `{% load … from … %}` took `trans` or `blocktrans` from a library other than
    /// `i18n`, so the legacy tags after it may be that library's.
    foreign_translation_tags: bool,
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
            reads_tags: settings.rules.contains_any(TAG_RULES),
            foreign_translation_tags: false,
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

    /// The Django version the templates target, [`None`] when it is neither set nor inferred;
    /// no version-gated rule can fire without one.
    #[must_use]
    const fn target_version(&self) -> Option<DjangoVersion> {
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
            NodeKind::JinjaTag(tag) => self.visit_jinja_tag(tag),
            NodeKind::Comment(comment) => {
                self.visit_comment(HTML_COMMENT, comment.raw);
                self.visit_text(comment.raw);
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

    /// Django names a tag by its first bit; Jinja strips whitespace-control markers first.
    fn tag_name(&self, tag: &JinjaTag<'a>) -> &'a str {
        if self.is_django() {
            dtl::tag_name(tag.content)
        } else {
            jinja_tag_name(tag, self.context.language())
        }
    }

    /// A tag has one name, so it is read once here and each arm holds that name's rules.
    fn visit_jinja_tag(&mut self, tag: &JinjaTag<'a>) {
        if self.reads_tags && !self.in_raw {
            self.visit_tag_named(tag, self.tag_name(tag));
        }
    }

    fn visit_tag_named(&mut self, tag: &JinjaTag<'a>, name: &'a str) {
        match name {
            "block" => {
                if self.is_rule_enabled(Rule::DuplicateBlockName)
                    && let Some(name) =
                        rules::correctness::duplicate_block_name::block_name_from_content(
                            tag.content,
                        )
                {
                    self.block_names.push(name);
                }
            }
            "include" => {
                // Same-file detection needs the linted file's path (absent in e.g. the WASM playground).
                if let Some(path) = self.context.path()
                    && self.is_rule_enabled(Rule::SameFilePartialInclude)
                {
                    rules::style::same_file_partial_include::check(self, tag, path);
                }
            }
            "load" => {
                if let Some(version) = self.target_version() {
                    if self.is_rule_enabled(Rule::DeprecatedStaticLibrary) {
                        rules::upgrade::deprecated_static_library::check(self, tag, version);
                    }
                    if self.is_rule_enabled(Rule::LegacyTranslationTag) {
                        self.foreign_translation_tags |=
                            rules::upgrade::legacy_translation_tag::check_load(self, tag, version);
                    }
                }
            }
            "trans" => {
                if let Some(version) = self.target_version()
                    && !self.foreign_translation_tags
                    && self.is_rule_enabled(Rule::LegacyTranslationTag)
                {
                    rules::upgrade::legacy_translation_tag::check_trans(self, tag, version);
                }
            }
            "with" => {
                if self.is_rule_enabled(Rule::LegacyAsAssignment) {
                    rules::upgrade::legacy_as_assignment::check_with(self, tag);
                }
            }
            "blocktranslate" | "blocktrans" => {
                if self.is_rule_enabled(Rule::LegacyAsAssignment) {
                    rules::upgrade::legacy_as_assignment::check_blocktranslate(self, tag);
                }
                if name == "blocktrans"
                    && let Some(version) = self.target_version()
                    && !self.foreign_translation_tags
                    && self.is_rule_enabled(Rule::LegacyTranslationTag)
                {
                    rules::upgrade::legacy_translation_tag::check_blocktrans(self, tag, version);
                }
            }
            "if" | "elif" => {
                if let Some(version) = self.target_version()
                    && self.is_rule_enabled(Rule::DeprecatedLengthIsFilter)
                {
                    rules::upgrade::deprecated_length_is_filter::check(self, tag, version);
                }
            }
            _ => {}
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
        if self.reads_tags
            && ["script", "style", "pre", "textarea"]
                .iter()
                .any(|tag| element.tag_name.eq_ignore_ascii_case(tag))
        {
            for child in &element.children {
                if let NodeKind::Text(text) = &child.kind {
                    self.visit_text(text.raw);
                }
            }
        }
    }

    fn visit_attribute(&mut self, attr: &Attribute<'a>, element: &Element<'a>) {
        match attr {
            Attribute::Native(native) => {
                self.visit_native_attribute(native, element);
                if let Some((value, _)) = native.value {
                    self.visit_text(value);
                }
            }
            Attribute::JinjaBlock(block) => self.visit_jinja_attr_block(block, element),
            Attribute::JinjaTag(tag) => self.visit_jinja_tag(tag),
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
        let outer_raw = self.in_raw;
        self.in_raw |= self.visit_block_opener(block);
        for item in inner_items(&block.body) {
            match item {
                JinjaTagOrChildren::Tag(tag) => self.visit_jinja_tag(tag),
                JinjaTagOrChildren::Children(children) => {
                    for child in children {
                        self.visit_node(child);
                    }
                }
            }
        }
        self.in_raw = outer_raw;
    }

    /// A block's opener has one name, read once for the block rules, the tag rules and `{% raw %}`.
    /// Returns whether the block is a Jinja `{% raw %}`, whose body emits its tags verbatim.
    fn visit_block_opener<T>(&mut self, block: &JinjaBlock<'a, T>) -> bool {
        let Some(JinjaTagOrChildren::Tag(opener)) = block.body.first() else {
            return false;
        };
        if !self.reads_tags {
            return false;
        }
        let name = self.tag_name(opener);
        if !self.in_raw {
            match name {
                "blocktranslate" | "blocktrans" => {
                    if self.is_rule_enabled(Rule::UntrimmedBlocktranslate) {
                        rules::correctness::untrimmed_blocktranslate::check(self, opener, name);
                    }
                }
                "ifequal" | "ifnotequal" => {
                    if let Some(version) = self.target_version()
                        && self.is_rule_enabled(Rule::DeprecatedIfequalTag)
                    {
                        let equality = if name == "ifequal" {
                            EqualityTag::Ifequal
                        } else {
                            EqualityTag::Ifnotequal
                        };
                        rules::upgrade::deprecated_ifequal_tag::check(
                            self, block, equality, version,
                        );
                    }
                }
                _ => {}
            }
            self.visit_tag_named(opener, name);
        }
        // Django has no `raw` tag; only Jinja parses one into a block.
        !self.is_django() && name == "raw"
    }

    /// The `{% %}` in `text` reach the tag rules, when one is on.
    #[inline]
    fn visit_text(&mut self, text: &'a str) {
        if self.reads_tags && !self.in_raw {
            self.visit_text_tags(text);
        }
    }

    // Out of line: inlined or not, a heavier body would tax every attribute.
    #[inline(never)]
    fn visit_text_tags(&mut self, text: &'a str) {
        let mut start = None;
        for tag in rules::helpers::tags_in_text(text) {
            let start = *start.get_or_insert_with(|| self.source_offset(text));
            let tag = JinjaTag {
                start: start + tag.start,
                ..tag
            };
            self.visit_jinja_tag(&tag);
        }
    }

    fn visit_jinja_attr_block(
        &mut self,
        block: &JinjaBlock<'a, Attribute<'a>>,
        element: &Element<'a>,
    ) {
        let outer_raw = self.in_raw;
        self.in_raw |= self.visit_block_opener(block);
        for item in inner_items(&block.body) {
            match item {
                JinjaTagOrChildren::Tag(tag) => self.visit_jinja_tag(tag),
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

/// The body between a block's opener and its closer, the two tags the opener's visit reads.
const fn inner_items<'b, 's, T>(
    body: &'b [JinjaTagOrChildren<'s, T>],
) -> &'b [JinjaTagOrChildren<'s, T>] {
    let body = match body {
        [JinjaTagOrChildren::Tag(_), rest @ ..] => rest,
        _ => body,
    };
    match body {
        [rest @ .., JinjaTagOrChildren::Tag(_)] => rest,
        _ => body,
    }
}

/// `parse_jinja_tag_name` on bytes: the name is the run of `[A-Za-z0-9_]` past the
/// whitespace-control markers and the whitespace. The fork decodes non-ASCII whitespace.
fn jinja_tag_name<'s>(tag: &JinjaTag<'s>, language: Language) -> &'s str {
    let rest = tag
        .content
        .trim_start_matches(['+', '-'])
        .trim_ascii_start();
    match rest.bytes().next() {
        // `\x0B` and the non-ASCII code points are the whitespace ASCII trimming leaves behind.
        Some(b) if b == b'\x0B' || b >= 0x80 => parse_jinja_tag_name(tag, language),
        _ => {
            let end = rest
                .bytes()
                .position(|b| !(b.is_ascii_alphanumeric() || b == b'_'))
                .unwrap_or(rest.len());
            &rest[..end]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jinja_tag_name_matches_the_parser() {
        for content in [
            "- if x -",
            "+for",
            " raw ",
            "",
            "set-x",
            "\u{a0}set",
            "\x0Bset",
            "é",
        ] {
            let tag = JinjaTag { content, start: 0 };
            assert_eq!(
                jinja_tag_name(&tag, Language::Jinja),
                parse_jinja_tag_name(&tag, Language::Jinja),
                "{content:?}"
            );
        }
    }
}
