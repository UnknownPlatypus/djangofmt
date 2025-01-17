"""
Default projects for ecosystem checks
"""

from ecosystem_check.projects import (
    Project,
    Repository,
    FormatOptions,
)

# TODO(zanieb): Consider exporting this as JSON and loading from there instead
DEFAULT_TARGETS = [
    # Jinja templates
    Project(repo=Repository(owner="zulip", name="zulip", ref="main")),
    # Django templates
    Project(repo=Repository(owner="django", name="django", ref="main")),
    Project(repo=Repository(owner="sissbruecker", name="linkding", ref="master")),
    Project(repo=Repository(owner="wagtail", name="wagtail", ref="main")),
    Project(repo=Repository(owner="django-cms", name="django-cms", ref="develop-4")),
    Project(repo=Repository(owner="django-oscar", name="django-oscar", ref="master")),
    Project(repo=Repository(owner="saleor", name="saleor", ref="main")),
    Project(
        repo=Repository(owner="django-commons", name="django-debug-toolbar", ref="main")
    ),
    Project(
        repo=Repository(owner="cookiecutter", name="cookiecutter-django", ref="master")
    ),
    Project(repo=Repository(owner="pennersr", name="django-allauth", ref="main")),
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
        ),
        format_options=FormatOptions(
            exclude=(
                # https://github.com/g-plane/markup_fmt/pull/98
                "import_export/templates/admin/import_export/export.html",
            )
        ),
    ),
]
