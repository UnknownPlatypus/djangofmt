use djangofmt_benchmark::{
    DJANGO_TEMPLATE_ATTR_DENSE, DJANGO_TEMPLATE_DEEPLY_NESTED, DJANGO_TEMPLATE_FORM_HEAVY,
    DJANGO_TEMPLATE_LARGE, DJANGO_TEMPLATE_SMALL, DJANGO_TEMPLATE_TAG_DENSE,
    DJANGO_TEMPLATE_WITH_SCRIPT_AND_STYLE_TAGS, JINJA_TEMPLATE_LARGE, TestFile,
};
use djangofmt_lint::{Settings, check_ast};
use markup_fmt::parser::Parser;

fn main() {
    divan::main();
}

#[divan::bench(args = [
  &DJANGO_TEMPLATE_SMALL,
  &DJANGO_TEMPLATE_WITH_SCRIPT_AND_STYLE_TAGS,
  &DJANGO_TEMPLATE_LARGE,
  &DJANGO_TEMPLATE_DEEPLY_NESTED,
  &DJANGO_TEMPLATE_TAG_DENSE,
  &DJANGO_TEMPLATE_FORM_HEAVY,
  &DJANGO_TEMPLATE_ATTR_DENSE,
  &JINJA_TEMPLATE_LARGE]
)]
fn check_templates(bencher: divan::Bencher, template: &'static TestFile) {
    let settings = Settings::default();

    bencher
        .counter(divan::counter::BytesCount::of_str(template.code))
        .bench(|| {
            let mut parser = Parser::new(
                divan::black_box(template.code),
                divan::black_box(template.profile.into()),
                vec![],
            );
            let ast = parser.parse_root().expect("Parsing to succeed");
            check_ast(
                divan::black_box(template.code),
                divan::black_box(&ast),
                divan::black_box(&settings),
            )
        });
}
