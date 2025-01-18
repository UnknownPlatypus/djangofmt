"""
Default projects for ecosystem checks
"""

from __future__ import annotations

from ecosystem_check.projects import (
    FormatOptions,
    Project,
    Repository,
)

DEFAULT_TARGETS = [
    # Jinja templates
    Project(repo=Repository(owner="zulip", name="zulip", ref="main"), format_options=FormatOptions(profile="jinja")),
    Project(
        repo=Repository(
            owner="cookiecutter",
            name="cookiecutter-django",
            ref="master",
        ),
        format_options=FormatOptions(
            exclude=(
                # Conditionals using raw tags, similar to https://github.com/g-plane/markup_fmt/issues/97
                "{{cookiecutter.project_slug}}/{{cookiecutter.project_slug}}/templates/allauth/elements/button.html",
                "{{cookiecutter.project_slug}}/{{cookiecutter.project_slug}}/templates/allauth/layouts/entrance.html",
                "{{cookiecutter.project_slug}}/{{cookiecutter.project_slug}}/templates/base.html",
                "{{cookiecutter.project_slug}}/{{cookiecutter.project_slug}}/templates/users/user_detail.html",
                "{{cookiecutter.project_slug}}/{{cookiecutter.project_slug}}/templates/users/user_form.html",
            ),
            profile="jinja"
        ),
    ),
    # Django templates
    Project(
        repo=Repository(owner="django", name="django", ref="main"),
        format_options=FormatOptions(
            exclude=(
                # Conditional open/close tags -> https://github.com/g-plane/markup_fmt/issues/97
                "django/contrib/admin/templates/admin/edit_inline/tabular.html",
                "django/contrib/admin/templates/admin/includes/fieldset.html",
                "django/contrib/admin/templates/admin/widgets/clearable_file_input.html",
                "django/contrib/admin/templates/admin/widgets/foreign_key_raw_id.html",
                "django/contrib/admin/templates/admin/widgets/url.html",
                "django/forms/templates/django/forms/field.html",
                "django/forms/templates/django/forms/widgets/input_option.html",
                "django/forms/templates/django/forms/widgets/multiple_input.html",
                "django/forms/templates/django/forms/widgets/select.html",
                "django/views/templates/technical_500.html",
                "tests/forms_tests/templates/forms_tests/use_fieldset.html",
                "tests/template_backends/templates/template_backends/syntax_error.html",
                "tests/test_client_regress/bad_templates/404.html",
            )
        ),
    ),
    Project(repo=Repository(owner="sissbruecker", name="linkding", ref="master")),
    Project(
        repo=Repository(owner="saleor", name="saleor", ref="main"),
        format_options=FormatOptions(
            exclude=(
                # TODO: Fails to parse <a href={% url "api" %}  target="_blank">
                "templates/home/index.html",
            )
        ),
    ),
    # TODO: All fail because of {% trans %} tag https://github.com/UnknownPlatypus/djangofmt/issues/7
    # Project(
    #   repo=Repository(owner="django-commons", name="django-debug-toolbar", ref="main")
    # ),
    # Project(
    #     repo=Repository(owner="django-oscar", name="django-oscar", ref="master"),
    #     format_options=FormatOptions(
    #         exclude=(
    #             "tests/_site/templates/oscar/layout.html",  # Actual invalid html
    #         )
    #     ),
    # ),
    # Project(repo=Repository(owner="django-cms", name="django-cms", ref="develop-4")),
    # Project(repo=Repository(owner="wagtail", name="wagtail", ref="main")),
    Project(
        repo=Repository(owner="pennersr", name="django-allauth", ref="main"),
        format_options=FormatOptions(
            custom_blocks="slot,element",
            exclude=(
                "examples/regular-django/example/templates/allauth/elements/form.html",
            ),
        ),
    ),
    Project(
        repo=Repository(
            owner="silentsokolov", name="django-admin-rangefilter", ref="master"
        ),
        format_options=FormatOptions(
            exclude=(
                # Django comments https://github.com/UnknownPlatypus/djangofmt/issues/8
                "rangefilter/templates/rangefilter/date_range_quick_select_list_filter.html",
            )
        ),
    ),
    Project(
        repo=Repository(
            owner="carltongibson", name="django-template-partials", ref="main"
        )
    ),
    Project(
        repo=Repository(
            owner="django-import-export", name="django-import-export", ref="main"
        ),
        format_options=FormatOptions(
            exclude=(
                # https://github.com/g-plane/markup_fmt/pull/98
                "import_export/templates/admin/import_export/export.html",
            )
        ),
    ),
]
