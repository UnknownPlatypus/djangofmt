use std::cell::OnceCell;
use std::path::Path;

use markup_fmt::Language;
use markup_fmt::ast::{
    Attribute, Element, JinjaBlock, JinjaTagOrChildren, NativeAttribute, Node, NodeKind, Root,
};
use miette::SourceSpan;

use crate::LintDiagnostic;
use crate::Settings;
use crate::django_version::DjangoVersion;
use crate::lint_context::{DiagnosticGuard, LintContext};
use crate::registry::Rule;
use crate::rules;
use crate::suppression::IgnoreComment;
use crate::violation::Violation;
use djangofmt_syntax::tokens::{Token, Tokens, tag_name};
use djangofmt_syntax::{CommentDelimiters, HTML_COMMENT, TEMPLATE_COMMENT};

/// Runs the lint rules: the AST rules from a visitor over the `markup_fmt` tree, the token rules
/// from Django's token stream, which also holds the tags the parser kept in text.
pub struct Checker<'a> {
    context: LintContext<'a>,
    /// Lexed on first use, so a run without a token rule never pays for it.
    tokens: OnceCell<Tokens<'a>>,
    /// Inside a Jinja `{% raw %}` body, where template tags are literal text.
    in_raw: bool,
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
            tokens: OnceCell::new(),
            in_raw: false,
        }
    }

    /// Django's token stream for the source: every `{% %}` and `{{ }}`, in text too.
    pub fn tokens(&self) -> &Tokens<'a> {
        self.tokens
            .get_or_init(|| Tokens::lex(self.context.source(), self.context.language()))
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

    /// Whether a Django template targets `version` or newer: the gate of a version-gated rule.
    fn targets_django(&self, version: DjangoVersion) -> bool {
        self.is_django()
            && self
                .target_version()
                .is_some_and(|target| target >= version)
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
    }

    /// Run the token rules over Django's token stream: the tags by name, the `{{ }}` and
    /// the rules that read the whole stream.
    pub fn check_tokens(&self) {
        let tokens = self.tokens();
        // Whether a `{% load … from … %}` took `trans` or `blocktrans` from a library other than
        // `i18n`, so the legacy tags after it may be that library's.
        let mut foreign_translation_tags = false;
        for (i, token) in tokens.iter().enumerate() {
            match token {
                Token::Tag { tag, name } => match *name {
                    "include" => {
                        // Same-file detection needs the linted file's path (absent in e.g. the WASM playground).
                        if let Some(path) = self.context.path()
                            && self.is_rule_enabled(Rule::SameFilePartialInclude)
                        {
                            rules::style::same_file_partial_include::check(self, tag, path);
                        }
                    }
                    "load" => {
                        if self.is_rule_enabled(Rule::DeprecatedStaticLibrary)
                            && self.targets_django(DjangoVersion::new(2, 1))
                        {
                            rules::suspicious::deprecated_static_library::check(self, tag);
                        }
                        if self.is_rule_enabled(Rule::LegacyTranslationTag)
                            && self.targets_django(DjangoVersion::new(3, 1))
                        {
                            foreign_translation_tags |=
                                rules::style::legacy_translation_tag::check_load(self, tag);
                        }
                    }
                    "trans"
                        if !foreign_translation_tags
                            && self.is_rule_enabled(Rule::LegacyTranslationTag)
                            && self.targets_django(DjangoVersion::new(3, 1)) =>
                    {
                        rules::style::legacy_translation_tag::check_trans(self, tag);
                    }
                    "ifequal" | "ifnotequal"
                        if self.is_rule_enabled(Rule::DeprecatedIfequalTag)
                            && self.targets_django(DjangoVersion::new(3, 1))
                            && let Some(closer) = tokens.closer(i) =>
                    {
                        rules::suspicious::deprecated_ifequal_tag::check(self, tag, closer);
                    }
                    "with" => {
                        if self.is_rule_enabled(Rule::LegacyAsAssignment) {
                            rules::style::legacy_as_assignment::check_with(self, tag);
                        }
                    }
                    "blocktranslate" | "blocktrans" => {
                        if self.is_rule_enabled(Rule::LegacyAsAssignment) {
                            rules::style::legacy_as_assignment::check_blocktranslate(self, tag);
                        }
                        if *name == "blocktrans"
                            && !foreign_translation_tags
                            && self.is_rule_enabled(Rule::LegacyTranslationTag)
                            && self.targets_django(DjangoVersion::new(3, 1))
                        {
                            rules::style::legacy_translation_tag::check_blocktrans(
                                self,
                                tag,
                                &tokens[i + 1..],
                            );
                        }
                    }
                    _ => {}
                },
                Token::Var(interpolation) => {
                    if self.is_rule_enabled(Rule::RedundantJsonScriptId)
                        && self.targets_django(DjangoVersion::new(4, 1))
                    {
                        rules::complexity::redundant_json_script_id::check(self, interpolation);
                    }
                }
            }
        }
        if self.is_rule_enabled(Rule::DuplicateBlockName) {
            rules::correctness::duplicate_block_name::check(self, tokens);
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
            NodeKind::Comment(comment) => self.visit_comment(HTML_COMMENT, comment.raw),
            NodeKind::JinjaComment(comment) => self.visit_comment(TEMPLATE_COMMENT, comment.raw),
            _ => {}
        }
    }

    fn visit_comment(&self, delimiters: CommentDelimiters, body: &str) {
        if self.is_rule_enabled(Rule::DeprecatedIgnore) {
            rules::suspicious::deprecated_ignore::check(self, delimiters, body);
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
            rules::correctness::invalid_attr_value::check(self, attr, element);
        }

        if self.is_rule_enabled(Rule::EmptyAttrValue) {
            rules::style::empty_attr_value::check(self, attr, element);
        }

        if self.is_rule_enabled(Rule::RedundantTypeAttr) {
            rules::complexity::redundant_type_attr::check(self, attr, element);
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
        self.visit_opener(block);
        for item in &block.body {
            if let JinjaTagOrChildren::Children(children) = item {
                for child in children {
                    self.visit_node(child);
                }
            }
        }
        self.in_raw = outer_raw;
    }

    /// Shared by both block visitors, which differ only in how they visit children.
    fn visit_opener<T>(&mut self, block: &JinjaBlock<'a, T>) {
        let Some(JinjaTagOrChildren::Tag(opener)) = block.body.first() else {
            return;
        };
        let name = tag_name(opener, self.context.language());
        if self.in_raw {
            return;
        }
        if matches!(name, "blocktranslate" | "blocktrans")
            && self.is_rule_enabled(Rule::UntrimmedBlocktranslate)
        {
            rules::correctness::untrimmed_blocktranslate::check(self, opener, name);
        }
        if self.is_rule_enabled(Rule::EndblockLabel) {
            rules::style::endblock_label::check(self, block, name);
        }
        // Django has no `raw` tag; only Jinja parses one into a block.
        self.in_raw = !self.is_django() && name == "raw";
    }

    fn visit_jinja_attr_block(
        &mut self,
        block: &JinjaBlock<'a, Attribute<'a>>,
        element: &Element<'a>,
    ) {
        let outer_raw = self.in_raw;
        self.visit_opener(block);
        for item in &block.body {
            if let JinjaTagOrChildren::Children(children) = item {
                for child in children {
                    self.visit_attribute(child, element);
                }
            }
        }
        self.in_raw = outer_raw;
    }
}
