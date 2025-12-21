use markup_fmt::ast::{Attribute, Element, JinjaBlock, JinjaTagOrChildren, Node, NodeKind, Root};
use miette::SourceSpan;

use crate::LintDiagnostic;
use crate::rules;

/// AST visitor that collects lint diagnostics.
///
/// The `Checker` traverses the `markup_fmt` AST and runs lint rules at each node.
/// Rules report diagnostics via [`Checker::report`].
pub struct Checker<'a> {
    source: &'a str,
    diagnostics: Vec<LintDiagnostic>,
}

impl<'a> Checker<'a> {
    /// Create a new checker for the given source code.
    #[must_use]
    pub const fn new(source: &'a str) -> Self {
        Self {
            source,
            diagnostics: Vec::new(),
        }
    }

    /// Report a lint diagnostic.
    ///
    /// Called by lint rules when they detect an issue.
    pub fn report(
        &mut self,
        code: &'static str,
        message: String,
        span: SourceSpan,
        label: String,
        help: Option<String>,
    ) {
        self.diagnostics.push(LintDiagnostic {
            source_code: self.source.to_string(),
            code,
            message,
            span,
            label,
            help,
        });
    }

    /// Consume the checker and return all collected diagnostics.
    #[must_use]
    pub fn into_diagnostics(self) -> Vec<LintDiagnostic> {
        self.diagnostics
    }

    /// Visit the root of the AST and run all lint rules.
    pub fn visit_root(&mut self, root: &Root<'_>) {
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
        rules::invalid_attr_value::check(element, self);

        for attr in &element.attrs {
            self.visit_attribute(attr);
        }

        for child in &element.children {
            self.visit_node(child);
        }
    }

    fn visit_attribute(&mut self, attr: &Attribute<'_>) {
        if let Attribute::JinjaBlock(block) = attr {
            self.visit_jinja_attr_block(block);
        }
    }

    fn visit_jinja_block(&mut self, block: &JinjaBlock<'_, Node<'_>>) {
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
