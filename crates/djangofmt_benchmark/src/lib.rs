use djangofmt::options::Profile;
use std::fmt;

pub static DJANGO_TEMPLATE_SMALL: TestFile = TestFile {
    name: "small/404.html",
    code: include_str!("../resources/django/404.html"),
    profile: Profile::Django,
};
pub static DJANGO_TEMPLATE_WITH_SCRIPT_AND_STYLE_TAGS: TestFile = TestFile {
    name: "external_format/technical_500.html",
    code: include_str!("../resources/django/technical_500.html"),
    profile: Profile::Django,
};

pub static DJANGO_TEMPLATE_LARGE: TestFile = TestFile {
    name: "large/strip_tags1.html",
    code: include_str!("../resources/django/strip_tags1.html"),
    profile: Profile::Django,
};
pub static DJANGO_TEMPLATE_DEEPLY_NESTED: TestFile = TestFile {
    name: "deeply_nested/project_invitation.html",
    code: include_str!("../resources/makeplane/project_invitation.html"),
    profile: Profile::Django,
};

pub static JINJA_TEMPLATE_LARGE: TestFile = TestFile {
    name: "jinja_large/comparison_table_integrated.html",
    code: include_str!("../resources/zulip/comparison_table_integrated.html"),
    profile: Profile::Jinja,
};
#[derive(Clone)]
pub struct TestFile {
    pub name: &'static str,
    pub code: &'static str,
    pub profile: Profile,
}
impl TestFile {
    #[must_use]
    pub fn loc(&self) -> usize {
        self.code.lines().count()
    }

    #[must_use]
    pub const fn total_len(&self) -> usize {
        self.code.len()
    }
}

impl fmt::Debug for TestFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} lines, {} bytes)",
            self.name,
            self.loc(),
            self.total_len()
        )
    }
}
