"""
Default projects for ecosystem checks
"""

from ecosystem_check.projects import (
    Project,
    Repository,
)

# TODO(zanieb): Consider exporting this as JSON and loading from there instead
DEFAULT_TARGETS = [
    Project(repo=Repository(owner="django", name="django", ref="main")),
    Project(repo=Repository(owner="zulip", name="zulip", ref="main")),
    Project(repo=Repository(owner="sissbruecker", name="linkding", ref="master")),
]
