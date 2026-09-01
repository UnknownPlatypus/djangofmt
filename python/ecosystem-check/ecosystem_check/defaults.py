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
    Project(
        repo=Repository(owner="mozilla", name="addons-server", ref="master"),
        cli_options=CliOptions(
            profile=Profile.JINJA,
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "src/olympia/devhub/templates/devhub/addons/listing/macros.html",
                    # <details> opened inside a {% if %}
                    "src/olympia/scanners/templates/admin/scanners/scannerresult/formatted_matched_rules_with_files.html",
                    "src/olympia/scanners/templates/admin/scanners/scannerresult/formatted_matching_files_and_data.html",
                ),
                ExcludeReason.TEMPLATE_TAG_AS_ATTRIBUTE: (
                    # <p{{ ...|locale_html }}>, an interpolation standing for attributes
                    "src/olympia/reviewers/templates/reviewers/addon_details_box.html",
                    "src/olympia/versions/templates/versions/update_info.html",
                ),
                ExcludeReason.DYNAMIC_TAG_NAME: (
                    "src/olympia/templates/includes/forms.html",
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
    # `pretalx` formats its templates with djangofmt, so any diff here is a regression.
    Project(
        repo=Repository(owner="pretalx", name="pretalx", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.FOREIGN_TEMPLATE_ENGINE: (
                    "doc/_templates/index.html",  # Sphinx template using {% set %}
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="openstack", name="horizon", ref="master"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    # <div> opened in the {% if logout_status %} branches
                    "horizon/templates/auth/_login_form.html",
                    "horizon/templates/auth/_password_form.html",
                    "horizon/templates/auth/_totp_form.html",
                    # <div> opened in a {% block %}, closed in the parent template
                    "horizon/templates/auth/_login_modal.html",
                    "horizon/templates/auth/_login_page.html",
                    "horizon/templates/auth/_password_page.html",
                    "horizon/templates/auth/_totp_page.html",
                    "horizon/templates/bootstrap/progress_bar.html",
                    "horizon/templates/horizon/_messages.html",
                    "horizon/templates/horizon/_sidebar.html",
                    "horizon/templates/horizon/common/_breadcrumb.html",
                    "horizon/templates/horizon/common/_data_table.html",
                    # The `>` ending the <button> start tag sits in both {% if %} branches
                    "horizon/templates/horizon/common/_data_table_action.html",
                    "horizon/templates/horizon/common/_data_table_row_actions_dropdown.html",
                    "horizon/templates/horizon/common/_formset_table_row.html",
                    "horizon/templates/horizon/common/_limit_summary.html",
                    "openstack_dashboard/dashboards/admin/backups/templates/backups/_detail_overview.html",
                    "openstack_dashboard/dashboards/project/backups/templates/backups/_detail_overview.html",
                    "openstack_dashboard/dashboards/project/instances/templates/instances/_instance_ips.html",
                    "openstack_dashboard/dashboards/project/network_topology/templates/network_topology/_actions_list.html",
                    "openstack_dashboard/dashboards/project/routers/templates/routers/_detail_overview.html",
                    "openstack_dashboard/templates/header/_user_menu.html",
                    "openstack_dashboard/themes/material/templates/horizon/_sidebar.html",
                ),
                ExcludeReason.TEMPLATE_TAG_AS_ATTRIBUTE: (
                    # <td{{ cell.attr_string|safe }}>, an interpolation standing for attributes
                    "horizon/templates/horizon/common/_data_table_cell.html",
                    "horizon/templates/horizon/common/_data_table_row.html",
                ),
                ExcludeReason.MISSING_END_TAG: (
                    "openstack_dashboard/dashboards/project/key_pairs/templates/key_pairs/detail.html",  # </dt>
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "horizon/test/templates/base.html",  # Stray quote in a script src
                    "openstack_dashboard/templates/angular.html",  # <ngdetails> never closed
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="rafalp", name="Misago", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    # One element per {% if %}/{% elif %} branch, closed once afterwards
                    "misago/admin/templates/misago/admin/login.html",
                    "misago/admin/templates/misago/admin/messages.html",
                    "misago/templates/misago/account/settings/download_data_form.html",
                    "misago/templates/misago/messages.html",
                    "misago/templates/misago/post_edits/edit_diff.html",
                    "misago/templates/misago/post_feed/post.html",
                    "misago/templates/misago/profile/feed.html",
                    "misago/templates/misago/profile/header.html",
                    "misago/templates/misago/snackbars.html",
                    "misago/templates/misago/userslists/usercard.html",
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "misago/admin/templates/misago/admin/attachments/list.html",  # Doubled quote
                    "misago/templates/misago/admin/users/ban.html",  # Missing closing div
                    "misago/templates/misago/mark_as_read/page.html",  # Doubled quote
                    "misago/templates/misago/profile/follows.html",  # </div> closing an <h3>
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="bookwyrm-social", name="bookwyrm", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    # <form> opened in {% block modal-form-open %}, closed in a later block
                    "bookwyrm/templates/author/sync_modal.html",
                    "bookwyrm/templates/book/cover_add_modal.html",
                    "bookwyrm/templates/book/file_links/add_link_modal.html",
                    "bookwyrm/templates/book/sync_modal.html",
                    "bookwyrm/templates/confirm_email/resend_modal.html",
                    "bookwyrm/templates/lists/add_item_modal.html",
                    "bookwyrm/templates/lists/create_list_modal.html",
                    "bookwyrm/templates/readthrough/readthrough_modal.html",
                    "bookwyrm/templates/settings/link_domains/edit_domain_modal.html",
                    "bookwyrm/templates/snippets/create_status/layout.html",
                    "bookwyrm/templates/snippets/reading_modals/finish_reading_modal.html",
                    "bookwyrm/templates/snippets/reading_modals/progress_update_modal.html",
                    "bookwyrm/templates/snippets/reading_modals/start_reading_modal.html",
                    "bookwyrm/templates/snippets/reading_modals/stop_reading_modal.html",
                    "bookwyrm/templates/snippets/reading_modals/want_to_read_modal.html",
                    "bookwyrm/templates/snippets/report_modal.html",
                    "bookwyrm/templates/snippets/toggle/toggle_button.html",
                    # <a>/<optgroup> opened inside an {% if %}
                    "bookwyrm/templates/book/edit/edit_book.html",
                    "bookwyrm/templates/book/sections/description.html",
                    "bookwyrm/templates/widgets/select.html",
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="DjangoCRM", name="django-crm", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "crm/templates/admin/crm/crmemail/includes/fieldset.html",
                    "crm/templates/admin/crm/crmemail/includes/fieldset_for_inline.html",
                    "crm/templates/admin/crm/crmemail/stacked.html",
                    "crm/templates/admin/crm/inline_emails.html",
                    "crm/templates/admin/crm/payment/change_list.html",
                    "quality/templates/admin/quality/transactionqualityevent/stacked.html",
                    "tasks/templates/admin/tasks/memo/change_list_object_tools.html",
                    # <ul> opened before the {% with %} that holds its </ul>
                    "templates/admin/crm_date_filter.html",
                ),
                ExcludeReason.MISSING_END_TAG: (
                    # <p> where </p> was intended
                    "analytics/templates/admin/analytics/closingreasonstat/closingreasons_summary_change_list.html",
                    "analytics/templates/admin/analytics/leadsourcestat/leadsource_summary_change_list.html",
                    "analytics/templates/analytics/bar_chart.html",
                    "analytics/templates/analytics/bar_chart_.html",
                    "common/templates/common/select_emails.html",  # </tr>
                    "crm/templates/admin/crm/contact/change_form_object_tools.html",  # </li>
                ),
                ExcludeReason.INVALID_ENCODING: (
                    # Truncated UTF-8 sequence in a class attribute
                    "analytics/templates/analytics/data_table.html",
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="sehmaschine", name="django-grappelli", ref="master"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "grappelli/dashboard/templates/grappelli/dashboard/module.html",
                    "grappelli/templates/admin/includes/fieldset.html",
                    "grappelli/templates/admin/includes/fieldset_inline.html",
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "grappelli/templates/grp_doc/change_list.html",  # Unclosed <section>
                    "grappelli/templates/grp_doc/fieldsets.html",  # Unclosed <small>
                    "grappelli/templates/grp_doc/filter.html",  # Stray </div>
                    "grappelli/templates/grp_doc/groups.html",  # Stray </div>
                    "grappelli/templates/grp_doc/tables.html",  # <th> closed by </td>
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="django-helpdesk", name="django-helpdesk", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    # <ul>/<a> opened inside an {% if %}, closed in a later one
                    "src/helpdesk/templates/helpdesk/public_view_ticket.html",
                    "src/helpdesk/templates/helpdesk/report_index.html",
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "src/helpdesk/templates/helpdesk/debug.html",  # </col> on a void element
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="readthedocs", name="readthedocs.org", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.FOREIGN_TEMPLATE_ENGINE: (
                    "docs/_templates/breadcrumbs.html",  # Sphinx template using {{ super() }}
                ),
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "readthedocs/templates/search/elastic_search.html",
                ),
                ExcludeReason.MISSING_END_TAG: (
                    "readthedocs/templates/projects/project_notifications.html",  # </p>
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    # <b> where </b> was intended
                    "readthedocs/subscriptions/templates/subscriptions/notifications/organization_disabled_email.html",
                    "readthedocs/subscriptions/templates/subscriptions/notifications/subscription_ended_email.html",
                    "readthedocs/subscriptions/templates/subscriptions/notifications/subscription_required_email.html",
                    "readthedocs/templates/projects/legend.html",  # Stray </em>
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="djangopackages", name="djangopackages", ref="main"),
        cli_options=CliOptions(custom_blocks="flag,switch"),
    ),
    Project(
        repo=Repository(owner="wger-project", name="wger", ref="master"),
        cli_options=CliOptions(
            custom_blocks="slot",
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    # {% endwith %} placed inside the <p> the {% with %} opened before
                    "wger/core/templates/user/delete_account.html",
                    # <s> opened inside an {% if %}, closed in a later one
                    "wger/trophies/templates/trophies/overview.html",
                ),
                ExcludeReason.DYNAMIC_TAG_NAME: (
                    "wger/core/templates/allauth/elements/button.html",
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    "wger/core/templates/template.html",  # </hr> on a void element
                    "wger/nutrition/templates/ingredient/view.html",  # Dangling </a>
                    "wger/software/templates/features.html",  # </hr> on a void element
                ),
            },
        ),
    ),
    Project(
        repo=Repository(owner="farridav", name="django-jazzmin", ref="main"),
        cli_options=CliOptions(
            exclude={
                ExcludeReason.TAG_SPANS_TEMPLATE_BLOCK: (
                    "jazzmin/templates/admin/includes/fieldset.html",
                    "jazzmin/templates/admin/index.html",
                    "jazzmin/templates/jazzmin/widgets/select.html",
                ),
                ExcludeReason.MISSING_END_TAG: (
                    "jazzmin/templates/admin/filer/breadcrumbs.html",  # </li>
                ),
                ExcludeReason.INVALID_SOURCE_HTML: (
                    # Attribute quote left open across a conditional placeholder
                    "jazzmin/templates/admin/change_list.html",
                    "jazzmin/templates/admin_doc/view_index.html",  # Missing closing div
                ),
            },
        ),
    ),
]
