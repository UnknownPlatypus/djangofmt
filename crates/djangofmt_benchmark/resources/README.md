This directory vendors some files from actual projects.
This is to benchmark Djangofmt's performance against real-world code instead of synthetic benchmarks.

The following files are included:

- [`zulip/templates/corporate/comparison_table_integrated.html`](https://github.com/zulip/zulip/blob/1e7c4f43891c449a28e53edbef9320cef6b25b6a/templates/corporate/comparison_table_integrated.html)
- [`plane/apps/api/templates/emails/invitations/project_invitation.html`](https://github.com/makeplane/plane/blob/22339b9786e98e12be803ca33eeaafe56fb5134a/apps/api/templates/emails/invitations/project_invitation.html)
- [`django/tests/utils_tests/files/strip_tags1.html`](https://github.com/django/django/blob/922c4cf972e04b1ce7ecee592231106724dcfd09/tests/utils_tests/files/strip_tags1.html)
- [`django/django/contrib/admin/templates/admin/404.html`](https://github.com/django/django/blob/922c4cf972e04b1ce7ecee592231106724dcfd09/django/contrib/admin/templates/admin/404.html)
- [`django/django/technical_500.html`](https://github.com/django/django/blob/922c4cf972e04b1ce7ecee592231106724dcfd09/django/views/templates/technical_500.html)
- [`django/django/contrib/admin/templates/admin/change_form.html`](https://github.com/django/django/blob/35dab0ad9ee2ed23101420cb0f253deda2818191/django/contrib/admin/templates/admin/change_form.html)
- [`wagtail/admin/templates/wagtailadmin/login.html`](https://github.com/wagtail/wagtail/blob/6e809784bda871df1bd5987833870e22cb1af940/wagtail/admin/templates/wagtailadmin/login.html)
- [`wagtail/admin/templates/wagtailadmin/shared/headers/slim_header.html`](https://github.com/wagtail/wagtail/blob/6e809784bda871df1bd5987833870e22cb1af940/wagtail/admin/templates/wagtailadmin/shared/headers/slim_header.html)

The files are included in the `resources` directory to allow running benchmarks offline and for simplicity.
They're licensed according to their original licenses (see links).
