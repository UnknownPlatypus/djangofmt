use djangofmt_lint::check_ast;
use insta::assert_yaml_snapshot;
use markup_fmt::{Language, Parser};

fn check(source: &str) -> Vec<String> {
    let mut parser = Parser::new(source, Language::Jinja, vec![]);
    let ast = parser.parse_root().unwrap();
    check_ast(source, &ast)
        .into_iter()
        .map(|d| format!("{}: {}", d.code, d.message))
        .collect()
}

#[test]
fn test_form_method_valid_get() {
    assert_yaml_snapshot!(check(r#"<form method="get"></form>"#));
}

#[test]
fn test_form_method_valid_post() {
    assert_yaml_snapshot!(check(r#"<form method="post"></form>"#));
}

#[test]
fn test_form_method_valid_dialog() {
    assert_yaml_snapshot!(check(r#"<form method="dialog"></form>"#));
}

#[test]
fn test_form_method_valid_uppercase() {
    assert_yaml_snapshot!(check(r#"<form method="GET"></form>"#));
}

#[test]
fn test_form_method_invalid_put() {
    assert_yaml_snapshot!(check(r#"<form method="put"></form>"#));
}

#[test]
fn test_form_method_invalid_delete() {
    assert_yaml_snapshot!(check(r#"<form method="delete"></form>"#));
}

#[test]
fn test_form_method_invalid_patch() {
    assert_yaml_snapshot!(check(r#"<form method="patch"></form>"#));
}

#[test]
fn test_form_method_nested_in_jinja() {
    assert_yaml_snapshot!(check(
        r#"
{% if foo %}
    <form method="invalid"></form>
{% endif %}
"#
    ));
}
