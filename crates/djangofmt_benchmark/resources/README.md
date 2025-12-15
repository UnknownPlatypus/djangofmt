This directory vendors some files from actual projects.
This is to benchmark Djangofmt's performance against real-world code instead of synthetic benchmarks.

The following files are included:

- [`templates/corporate/comparison_table_integrated.html`](https://github.com/zulip/zulip/blob/1e7c4f43891c449a28e53edbef9320cef6b25b6a/templates/corporate/comparison_table_integrated.html)
- [`apps/api/templates/emails/invitations/project_invitation.html`](https://github.com/makeplane/plane/blob/22339b9786e98e12be803ca33eeaafe56fb5134a/apps/api/templates/emails/invitations/project_invitation.html)
- [`tests/utils_tests/files/strip_tags1.html`](https://github.com/django/django/blob/922c4cf972e04b1ce7ecee592231106724dcfd09/tests/utils_tests/files/strip_tags1.html)

The files are included in the `resources` directory to allow running benchmarks offline and for simplicity.
They're licensed according to their original licenses (see link).
