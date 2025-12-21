use markup_fmt::ast::{Attribute, Element, JinjaBlock, JinjaTagOrChildren, Node, NodeKind, Root};
use miette::SourceSpan;

use crate::LintDiagnostic;
use crate::rules;

pub struct Checker<'a> {
    source: &'a str,
    diagnostics: Vec<LintDiagnostic>,
}

impl<'a> Checker<'a> {
    #[must_use]
    pub const fn new(source: &'a str) -> Self {
        Self {
            source,
            diagnostics: Vec::new(),
        }
    }

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

    #[must_use]
    pub fn into_diagnostics(self) -> Vec<LintDiagnostic> {
        self.diagnostics
    }

    pub fn visit_root(&mut self, root: &Root<'_>) {
        for node in &root.children {
            self.visit_node(node);
        }
    }

    pub fn visit_node(&mut self, node: &Node<'_>) {
        match &node.kind {
            NodeKind::Element(element) => self.visit_element(element),
            NodeKind::JinjaBlock(block) => self.visit_jinja_block(block),
            _ => {}
        }
    }

    pub fn visit_element(&mut self, element: &Element<'_>) {
        rules::form_method::check(element, self);

        for attr in &element.attrs {
            self.visit_attribute(attr);
        }

        for child in &element.children {
            self.visit_node(child);
        }
    }

    pub fn visit_attribute(&mut self, attr: &Attribute<'_>) {
        if let Attribute::JinjaBlock(block) = attr {
            self.visit_jinja_attr_block(block);
        }
    }

    pub fn visit_jinja_block(&mut self, block: &JinjaBlock<'_, Node<'_>>) {
        for item in &block.body {
            if let JinjaTagOrChildren::Children(children) = item {
                for child in children {
                    self.visit_node(child);
                }
            }
        }
    }

    pub fn visit_jinja_attr_block(&mut self, block: &JinjaBlock<'_, Attribute<'_>>) {
        for item in &block.body {
            if let JinjaTagOrChildren::Children(children) = item {
                for child in children {
                    self.visit_attribute(child);
                }
            }
        }
    }
}
