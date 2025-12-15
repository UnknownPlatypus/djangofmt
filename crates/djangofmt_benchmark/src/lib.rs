use djangofmt::args::Profile;

pub static LARGE_JINJA_TEMPLATE: TestFile = TestFile {
    name: "zulip/comparison_table_integrated.html",
    code: include_str!("../resources/zulip/comparison_table_integrated.html"),
    profile: Profile::Jinja,
};
pub static LARGE_DJANGO_TEMPLATE: TestFile = TestFile {
    name: "django/strip_tags1.html",
    code: include_str!("../resources/django/strip_tags1.html"),
    profile: Profile::Django,
};
pub static NESTED_DJANGO_TEMPLATE: TestFile = TestFile {
    name: "makeplane/project_invitation.html",
    code: include_str!("../resources/makeplane/project_invitation.html"),
    profile: Profile::Django,
};
#[derive(Debug, Clone)]
pub struct TestFile {
    pub name: &'static str,
    pub code: &'static str,
    pub profile: Profile,
}
