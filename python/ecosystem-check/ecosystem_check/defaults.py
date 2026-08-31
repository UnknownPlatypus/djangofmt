"""
Default projects for ecosystem checks
"""

from __future__ import annotations

from ecosystem_check.projects import (
    CliOptions,
    ExcludeReason,
    GitDomain,
    Profile,
    Project,
    Repository,
)

DEFAULT_TARGETS = [
    # Jinja templates
    Project(
        repo=Repository(owner="zulip", name="zulip", ref="main"),
        cli_options=CliOptions(
            profile=Profile.JINJA,
        ),
    ),
    Project(
        repo=Repository(
            owner="cookiecutter",
            name="cookiecutter-django",
            ref="main",
        ),
        cli_options=CliOptions(
            profile=Profile.JINJA,
            exclude={
                ExcludeReason.DYNAMIC_TAG_NAME: (
                    "{{cookiecutter.project_slug}}/{{cookiecutter.project_slug}}/templates/allauth/elements/button.html",
                ),
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "{{cookiecutter.project_slug}}/{{cookiecutter.project_slug}}/templates/allauth/layouts/entrance.html",
                    "{{cookiecutter.project_slug}}/{{cookiecutter.project_slug}}/templates/base.html",
                    "{{cookiecutter.project_slug}}/{{cookiecutter.project_slug}}/templates/users/user_detail.html",
                    "{{cookiecutter.project_slug}}/{{cookiecutter.project_slug}}/templates/users/user_form.html",
                ),
            },
        ),
    ),
    # Django templates
    Project(
        repo=Repository(owner="django", name="django", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
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
                ),
                ExcludeReason.MISSING_END_TAG: (
                    # Trailing <li> where </li> was intended
                    "django/contrib/admindocs/templates/admin_doc/model_index.html",
                ),
                ExcludeReason.INTENTIONALLY_INVALID: (
                    "tests/template_backends/templates/template_backends/syntax_error.html",
                    "tests/test_client_regress/bad_templates/404.html",
                ),
                ExcludeReason.FOREIGN_TEMPLATE_ENGINE: (
                    "django/forms/jinja2/django/forms/widgets/multiwidget.html",
                    "docs/_theme/djangodocs/layout.html",
                    "docs/_theme/djangodocs-epub/epub-cover.html",
                ),
            },
        ),
    ),
    Project(repo=Repository(owner="sissbruecker", name="linkding", ref="master")),
    Project(repo=Repository(owner="saleor", name="saleor", ref="main")),
    Project(
        repo=Repository(
            owner="django-commons", name="django-debug-toolbar", ref="main"
        ),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    # <template> opened inside {% if use_shadow_dom %}
                    "debug_toolbar/templates/debug_toolbar/base.html",
                    "debug_toolbar/templates/debug_toolbar/includes/panel_button.html",
                    "debug_toolbar/templates/debug_toolbar/panels/sql_explain.html",
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="django-oscar", name="django-oscar", ref="master"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "src/oscar/templates/oscar/catalogue/browse.html",
                    "src/oscar/templates/oscar/catalogue/reviews/partials/review_stars.html",
                    "src/oscar/templates/oscar/checkout/shipping_address.html",
                    "src/oscar/templates/oscar/dashboard/reviews/review_list.html",
                    "src/oscar/templates/oscar/dashboard/users/detail.html",
                ),
                ExcludeReason.MISSING_END_TAG: (
                    "src/oscar/templates/oscar/dashboard/shipping/messages/band_deleted.html",  # </p>
                    "tests/_site/templates/oscar/layout.html",  # </li>
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "src/oscar/templates/oscar/dashboard/partners/partner_manage.html",  # Missing closing div
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="django-cms", name="django-cms", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    # <body> opened inside a {% spaceless %} and closed outside
                    "cms/templates/cms/headless/placeholder.html",
                    # <div> opened in {% if %}/{% else %} branches, closed once outside
                    "cms/templates/cms/toolbar/toolbar_with_structure.html",
                ),
                ExcludeReason.TEMPLATE_TAG_AS_ATTRIBUTE: (
                    # {{ attr }}="{{ val }}" dynamic attribute name
                    "cms/templates/cms/widgets/pagesmartlinkwidget.html",
                ),
                ExcludeReason.MISSING_END_TAG: (
                    "cms/templates/cms/noapphook.html",  # </html>
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "cms/templates/admin/cms/page/tree/actions_dropdown.html",  # <span>{% trans "Copy" %}<span>
                    # Comment that looks like a broken tag: <--noplaceholder-->
                    "cms/test_utils/project/sampleapp/templates/sampleapp/home.html",
                ),
                ExcludeReason.UNKNOWN: (
                    # Breaks somewhere inside the multiline data-json='{...}' attribute
                    "cms/templates/admin/cms/page/tree/base.html",
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="wagtail", name="wagtail", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "wagtail/admin/templates/wagtailadmin/shared/icon.html",
                    "wagtail/admin/templates/wagtailadmin/tables/references_cell.html",
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="pennersr", name="django-allauth", ref="main"),
        cli_options=CliOptions(
            custom_blocks="slot,element",
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "examples/regular-django/example/templates/allauth/elements/form.html",
                ),
                ExcludeReason.DYNAMIC_TAG_NAME: (
                    "allauth/templates/allauth/elements/button.html",
                    "examples/regular-django/example/templates/allauth/elements/button.html",
                ),
            },
        ),
    ),
    Project(
        repo=Repository(
            owner="silentsokolov", name="django-admin-rangefilter", ref="master"
        )
    ),
    Project(
        repo=Repository(
            owner="carltongibson", name="django-template-partials", ref="main"
        )
    ),
    Project(
        repo=Repository(
            owner="django-import-export", name="django-import-export", ref="main"
        )
    ),
    Project(
        repo=Repository(owner="unfoldadmin", name="django-unfold", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "src/unfold/templates/admin/actions.html",
                    "src/unfold/templates/admin/dataset_actions.html",
                    "src/unfold/templates/admin/date_hierarchy.html",
                    "src/unfold/templates/admin/edit_inline/stacked.html",
                    "src/unfold/templates/admin/edit_inline/tabular.html",
                    "src/unfold/templates/admin/includes/fieldset.html",
                    "src/unfold/templates/unfold/widgets/radio.html",
                    "src/unfold/templates/unfold/widgets/radio_option.html",
                    "src/unfold/templates/unfold/widgets/select.html",
                    "src/unfold/templates/unfold_crispy/layout/table_inline_formset.html",
                    "src/unfold/templates/unfold_crispy/whole_uni_form.html",
                    "src/unfold/templates/unfold_crispy/whole_uni_formset.html",
                ),
                ExcludeReason.DYNAMIC_TAG_NAME: (
                    "src/unfold/templates/unfold/components/button.html",
                    "src/unfold/templates/unfold/components/card.html",
                    "src/unfold/templates/unfold/helpers/change_list_filter_vertical.html",
                    "src/unfold/templates/unfold/helpers/header_title.html",
                    "src/unfold/templates/unfold/helpers/label.html",
                    "src/unfold/templates/unfold/helpers/site_icon.html",
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "src/unfold/templates/unfold/helpers/display_header.html",  # </div> closing a <span>
                ),
            },
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
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "src/sentry/templates/sentry/emails/reports/body.html",
                    "src/sentry/templates/sentry/partial/system-status.html",
                ),
                ExcludeReason.MISSING_END_TAG: (
                    "src/sentry/templates/sentry/toolbar/iframe.html",  # </body>, left out on purpose
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "src/sentry/templates/sentry/debug/error-page-embed.html",  # Broken close tag
                    "src/sentry/templates/sentry/emails/sentry-app-publish-confirmation.html",  # Broken close tag
                    "src/sentry/templates/sentry/integrations/notify-disable.html",  # Dangling </a>
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="makeplane", name="plane", ref="preview"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "apps/api/templates/emails/test_email.html",  # Stray </br>
                ),
            },
        ),
    ),
    Project(repo=Repository(owner="e-valuation", name="EvaP", ref="main")),
    Project(
        repo=Repository(owner="django", name="djangoproject.com", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    # <a> opened inside a {% blocktranslate %} and closed outside
                    "djangoproject/templates/aggregator/local-django-community.html",
                    "djangoproject/templates/base_weblog.html",
                    "djangoproject/templates/foundation/coreawardcohort_list.html",
                    "djangoproject/templates/fundraising/includes/_hero_with_logo.html",
                    "djangoproject/templates/fundraising/includes/display_django_heroes.html",
                    "djangoproject/templates/fundraising/manage-donations.html",
                    # <ul><li> opened inside a {% for %}
                    "docs/templates/docs/doc.html",
                ),
                ExcludeReason.MISSING_END_TAG: (
                    "djangoproject/templates/releases/download.html",  # Nested </p>
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "djangoproject/templates/start.html",  # </span> inside a {% translate %} string
                    "docs/templates/docs/genindex.html",  # Invalid <br/ > self close
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="healthchecks", name="healthchecks", ref="master"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    # {% endwith %} placed inside the <div> the {% with %} opened before
                    "templates/accounts/project.html",
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="babybuddy", name="babybuddy", ref="master"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.MISSING_END_TAG: (
                    "babybuddy/templates/babybuddy/paginator.html",  # </li>
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "babybuddy/templates/babybuddy/form_field.html",  # Stray quote in class attribute
                    "babybuddy/templates/registration/base.html",  # Dangling </a>
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="inventree", name="InvenTree", ref="master"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "src/backend/InvenTree/web/templates/web/index.html",  # Missing closing div
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="netbox-community", name="netbox", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "netbox/templates/django/forms/widgets/select.html",
                    "netbox/utilities/templates/builtins/badge.html",
                    "netbox/utilities/templates/builtins/tag.html",
                ),
                ExcludeReason.TEMPLATE_TAG_AS_ATTRIBUTE: (
                    "netbox/templates/core/buttons/bulk_sync.html",
                    "netbox/templates/dcim/buttons/bulk_add_components.html",
                    "netbox/templates/dcim/buttons/bulk_disconnect.html",
                    "netbox/templates/virtualization/buttons/bulk_add_components.html",
                    "netbox/utilities/templates/buttons/bulk_delete.html",
                    "netbox/utilities/templates/buttons/bulk_edit.html",
                    "netbox/utilities/templates/buttons/bulk_rename.html",
                ),
                ExcludeReason.MISSING_END_TAG: (
                    "netbox/templates/core/inc/config_data.html",  # </tr>
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "netbox/templates/extras/inc/configcontext_data.html",  # Unterminated id attribute
                ),
                ExcludeReason.UNSTABLE_FORMATTING: (
                    # Code after a multiline template literal is re-indented on every pass
                    "netbox/templates/graphql/graphiql.html",
                ),
            },
        ),
    ),
    Project(
        repo=Repository(
            owner="emmaDelescolle",
            name="django-admin-deux",
            ref="main",
            domain=GitDomain.CODEBERG,
        ),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    # <div>s opened in one {% if %} and closed in a later one
                    "djadmin-classy-doc/djadmin_classy_doc/templates/djadmin_classy_doc/view_detail.html",
                    # <li> opened inside {% if use_list_items %}
                    "djadmin/templates/djadmin/includes/_action_buttons.html",
                ),
            },
        ),
    ),
]
