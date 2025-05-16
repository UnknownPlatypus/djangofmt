"""
Default projects for ecosystem checks
"""

from __future__ import annotations

from ecosystem_check.projects import (
    FormatOptions,
    Profile,
    Project,
    Repository,
)

DEFAULT_TARGETS = [
    # Jinja templates
    Project(
        repo=Repository(owner="zulip", name="zulip", ref="main"),
        format_options=FormatOptions(
            profile=Profile.JINJA,
        ),
    ),
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
            profile=Profile.JINJA,
        ),
    ),
    # Django templates
    Project(
        repo=Repository(owner="django", name="django", ref="main"),
        format_options=FormatOptions(
            exclude=(
                # Conditional open/close tags -> https://github.com/g-plane/markup_fmt/issues/97
                "django/contrib/admin/templates/admin/edit_inline/stacked.html",
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
            ),
            djade_stability_exclude=(
                "tests/i18n/commands/templates/test.html",  # Contains invalid blocktranslate syntax
                "django/contrib/admindocs/templates/admin_doc/missing_docutils.html",  # `val as key` syntax swapped to `key=val`, changing line width
            ),
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
    Project(
        repo=Repository(
            owner="django-commons", name="django-debug-toolbar", ref="main"
        ),
        format_options=FormatOptions(
            exclude=(
                # Conditional open/close tags -> https://github.com/g-plane/markup_fmt/issues/97
                "debug_toolbar/templates/debug_toolbar/includes/panel_button.html",
                "debug_toolbar/templates/debug_toolbar/panels/sql_explain.html",
            )
        ),
    ),
    Project(
        repo=Repository(owner="django-oscar", name="django-oscar", ref="master"),
        format_options=FormatOptions(
            exclude=(
                "tests/_site/templates/oscar/layout.html",  # Actual invalid html
                "src/oscar/templates/oscar/dashboard/partners/partner_manage.html",  # Missing closing div
                "src/oscar/templates/oscar/dashboard/shipping/messages/band_deleted.html",  # Missing closing p
                "src/oscar/templates/oscar/dashboard/users/detail.html",  # Last endblock should be in div
                # Conditional open/close tags -> https://github.com/g-plane/markup_fmt/issues/97
                "src/oscar/templates/oscar/catalogue/browse.html",
                "src/oscar/templates/oscar/catalogue/reviews/partials/review_stars.html",
                "src/oscar/templates/oscar/checkout/shipping_address.html",
                "src/oscar/templates/oscar/dashboard/reviews/review_list.html",
            ),
        ),
    ),
    Project(
        repo=Repository(owner="django-cms", name="django-cms", ref="main"),
        format_options=FormatOptions(
            exclude=(
                "cms/templates/admin/cms/page/tree/actions_dropdown.html",  # Invalid <span>{% trans "Copy" %}<span>
                "cms/templates/admin/cms/page/tree/base.html",  # Weird </form> tag placement
                "cms/templates/cms/headless/placeholder.html",  # Unconventional use of {% spaceless %}
                "cms/templates/cms/noapphook.html",  # Missing html closing tag
                # Weird comment that look like a tag error <--noplaceholder-->
                "cms/test_utils/project/sampleapp/templates/sampleapp/home.html",
            ),
        ),
    ),
    Project(
        repo=Repository(owner="wagtail", name="wagtail", ref="main"),
        format_options=FormatOptions(
            exclude=(
                # Conditional open/close tags -> https://github.com/g-plane/markup_fmt/issues/97
                "wagtail/admin/templates/wagtailadmin/shared/icon.html",
                "wagtail/admin/templates/wagtailadmin/tables/references_cell.html",
            ),
        ),
    ),
    Project(
        repo=Repository(owner="pennersr", name="django-allauth", ref="main"),
        format_options=FormatOptions(
            custom_blocks="slot,element",
            exclude=(
                "examples/regular-django/example/templates/allauth/elements/form.html",
                # Conditional open/close tags -> https://github.com/g-plane/markup_fmt/issues/97
                "allauth/templates/allauth/elements/button.html",
                "examples/regular-django/example/templates/allauth/elements/button.html",
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
    Project(
        repo=Repository(owner="unfoldadmin", name="django-unfold", ref="main"),
        format_options=FormatOptions(
            exclude=(
                "src/unfold/contrib/simple_history/templates/simple_history/object_history_list.html",  # Broken close tag
                "src/unfold/templates/admin/auth/user/add_form.html",  # Broken close tag
                "src/unfold/templates/unfold/helpers/display_header.html",  # Broken close tag
                # Conditional open/close tags -> https://github.com/g-plane/markup_fmt/issues/97
                "src/unfold/templates/admin/actions.html",
                "src/unfold/templates/admin/date_hierarchy.html",
                "src/unfold/templates/admin/edit_inline/stacked.html",
                "src/unfold/templates/admin/edit_inline/tabular.html",
                "src/unfold/templates/unfold/widgets/radio.html",
                "src/unfold/templates/unfold/widgets/radio_option.html",
                "src/unfold/templates/unfold_crispy/layout/table_inline_formset.html",
                "src/unfold/templates/unfold_crispy/whole_uni_form.html",
                # conditional tag name with differing end tag like </{% if cl.model_admin.list_filter_submit %}form{% else %}div{% endif %}>
                "src/unfold/templates/unfold/components/button.html",
                "src/unfold/templates/unfold/helpers/change_list_filter.html",
                "src/unfold/templates/unfold/helpers/display_dropdown.html",
                "src/unfold/templates/unfold/helpers/site_icon.html",
                "src/unfold/templates/unfold_crispy/field.html",
            )
        ),
    ),
    Project(
        repo=Repository(
            owner="DmytroLitvinov",
            name="django-admin-inline-paginator-plus",
            ref="master",
        ),
    ),
    Project(
        repo=Repository(owner="getsentry", name="sentry", ref="master"),
        format_options=FormatOptions(
            exclude=(
                "src/sentry/templates/sentry/debug/error-page-embed.html",  # Broken close tag
                "src/sentry/templates/sentry/emails/sentry-app-publish-confirmation.html",  # Broken close tag
                "src/sentry/templates/sentry/integrations/notify-disable.html",  # Dangling </a>
                "src/sentry/templates/sentry/integrations/sentry-app-notify-disable.html",  # Dangling </a>
                "src/sentry/templates/sentry/toolbar/iframe.html",  # Intentionally unclosed body tag
                # Conditional open/close tags -> https://github.com/g-plane/markup_fmt/issues/97
                "src/sentry/templates/sentry/emails/reports/body.html",
                "src/sentry/templates/sentry/partial/system-status.html",
            ),
            djade_stability_exclude=(
                # Djade changes `with plugin.auth_provider as auth_provider` to `with auth_provider=plugin.auth_provider`
                # This causes the line to be shorter, and djangofmt format it again
                "src/sentry/templates/sentry/plugins/bases/issue/not_configured.html",
            ),
        ),
    ),
    Project(
        repo=Repository(owner="makeplane", name="plane", ref="preview"),
        format_options=FormatOptions(
            exclude=(
                "apiserver/templates/emails/test_email.html",  # Invalid </br> tag
            )
        ),
    ),
]
