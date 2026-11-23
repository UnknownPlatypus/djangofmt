//! Benchmark templates, see `resources/README.md` for their sources and licenses.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Syntax {
    Django,
    Jinja,
}

pub struct Template {
    pub name: &'static str,
    pub code: &'static str,
    pub syntax: Syntax,
}

/// The benchmark argument label, which `CodSpeed` uses as the benchmark ID.
impl fmt::Debug for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} lines, {} bytes)",
            self.name,
            self.code.lines().count(),
            self.code.len()
        )
    }
}

pub static DJANGO_TEMPLATE_SMALL: Template = Template {
    name: "small/404.html",
    code: include_str!("../resources/django/404.html"),
    syntax: Syntax::Django,
};
pub static DJANGO_TEMPLATE_WITH_SCRIPT_AND_STYLE_TAGS: Template = Template {
    name: "external_format/technical_500.html",
    code: include_str!("../resources/django/technical_500.html"),
    syntax: Syntax::Django,
};

pub static DJANGO_TEMPLATE_LARGE: Template = Template {
    name: "large/strip_tags1.html",
    code: include_str!("../resources/django/strip_tags1.html"),
    syntax: Syntax::Django,
};
pub static DJANGO_TEMPLATE_DEEPLY_NESTED: Template = Template {
    name: "deeply_nested/deeply_nested.html",
    code: include_str!("../resources/synthetic/deeply_nested.html"),
    syntax: Syntax::Django,
};

pub static DJANGO_TEMPLATE_TAG_DENSE: Template = Template {
    name: "tag_dense/change_form.html",
    code: include_str!("../resources/django/change_form.html"),
    syntax: Syntax::Django,
};
pub static DJANGO_TEMPLATE_FORM_HEAVY: Template = Template {
    name: "form_heavy/login.html",
    code: include_str!("../resources/wagtail/login.html"),
    syntax: Syntax::Django,
};
pub static DJANGO_TEMPLATE_ATTR_DENSE: Template = Template {
    name: "attr_dense/slim_header.html",
    code: include_str!("../resources/wagtail/slim_header.html"),
    syntax: Syntax::Django,
};
pub static DJANGO_TEMPLATE_COTTON: Template = Template {
    name: "cotton/home.html",
    code: include_str!("../resources/django-cotton/home.html"),
    syntax: Syntax::Django,
};
pub static DJANGO_TEMPLATE_LARGE_TAG_DENSE: Template = Template {
    name: "large_tag_dense/order_detail.html",
    code: include_str!("../resources/django-oscar/order_detail.html"),
    syntax: Syntax::Django,
};
pub static DJANGO_TEMPLATE_RAW_BLOCKS: Template = Template {
    name: "raw_blocks/components.html",
    code: include_str!("../resources/django-cotton/components.html"),
    syntax: Syntax::Django,
};

pub static JINJA_TEMPLATE_LARGE: Template = Template {
    name: "jinja_large/comparison_table_integrated.html",
    code: include_str!("../resources/zulip/comparison_table_integrated.html"),
    syntax: Syntax::Jinja,
};
pub static JINJA_TEMPLATE_FORM_HEAVY: Template = Template {
    name: "jinja_form_heavy/upgrade.html",
    code: include_str!("../resources/zulip/upgrade.html"),
    syntax: Syntax::Jinja,
};

pub static ALL_TEMPLATES: [&Template; 12] = [
    &DJANGO_TEMPLATE_SMALL,
    &DJANGO_TEMPLATE_WITH_SCRIPT_AND_STYLE_TAGS,
    &DJANGO_TEMPLATE_LARGE,
    &DJANGO_TEMPLATE_DEEPLY_NESTED,
    &DJANGO_TEMPLATE_TAG_DENSE,
    &DJANGO_TEMPLATE_FORM_HEAVY,
    &DJANGO_TEMPLATE_ATTR_DENSE,
    &DJANGO_TEMPLATE_COTTON,
    &DJANGO_TEMPLATE_LARGE_TAG_DENSE,
    &DJANGO_TEMPLATE_RAW_BLOCKS,
    &JINJA_TEMPLATE_FORM_HEAVY,
    &JINJA_TEMPLATE_LARGE,
];
