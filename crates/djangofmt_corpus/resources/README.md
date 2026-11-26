This directory vendors some files from actual projects.
This is to benchmark Djangofmt's performance against real-world code instead of synthetic benchmarks.

The following files are included, under their original licenses:

- [`zulip/templates/corporate/comparison_table_integrated.html`](https://github.com/zulip/zulip/blob/1e7c4f43891c449a28e53edbef9320cef6b25b6a/templates/corporate/comparison_table_integrated.html) (Apache-2.0)
- [`zulip/templates/corporate/billing/upgrade.html`](https://github.com/zulip/zulip/blob/1e7c4f43891c449a28e53edbef9320cef6b25b6a/templates/corporate/billing/upgrade.html) (Apache-2.0)
- [`django/tests/utils_tests/files/strip_tags1.html`](https://github.com/django/django/blob/922c4cf972e04b1ce7ecee592231106724dcfd09/tests/utils_tests/files/strip_tags1.html) (BSD-3-Clause)
- [`django/django/contrib/admin/templates/admin/404.html`](https://github.com/django/django/blob/922c4cf972e04b1ce7ecee592231106724dcfd09/django/contrib/admin/templates/admin/404.html) (BSD-3-Clause)
- [`django/django/technical_500.html`](https://github.com/django/django/blob/922c4cf972e04b1ce7ecee592231106724dcfd09/django/views/templates/technical_500.html) (BSD-3-Clause)
- [`django/django/contrib/admin/templates/admin/change_form.html`](https://github.com/django/django/blob/35dab0ad9ee2ed23101420cb0f253deda2818191/django/contrib/admin/templates/admin/change_form.html) (BSD-3-Clause)
- [`wagtail/admin/templates/wagtailadmin/login.html`](https://github.com/wagtail/wagtail/blob/6e809784bda871df1bd5987833870e22cb1af940/wagtail/admin/templates/wagtailadmin/login.html) (BSD-3-Clause)
- [`wagtail/admin/templates/wagtailadmin/shared/headers/slim_header.html`](https://github.com/wagtail/wagtail/blob/6e809784bda871df1bd5987833870e22cb1af940/wagtail/admin/templates/wagtailadmin/shared/headers/slim_header.html) (BSD-3-Clause)
- [`django-cotton/docs/docs_project/docs_project/templates/ui_docs/home.html`](https://github.com/wrabit/django-cotton/blob/982671f2eba62340264c0ee7084033c1c76b0d57/docs/docs_project/docs_project/templates/ui_docs/home.html) (MIT)
- [`django-cotton/docs/docs_project/docs_project/templates/components.html`](https://github.com/wrabit/django-cotton/blob/d3e1ccdd1754ea5682a3af260bccfc6ba84ea5c3/docs/docs_project/docs_project/templates/components.html) (MIT)
- [`django-oscar/src/oscar/templates/oscar/dashboard/orders/order_detail.html`](https://github.com/django-oscar/django-oscar/blob/9bc531349519c4448eaec5d4c402f9bdcebd85bb/src/oscar/templates/oscar/dashboard/orders/order_detail.html) (BSD-3-Clause)

The files are included in the `resources` directory to allow running benchmarks offline and for simplicity.

Other projects pin this corpus by git tag. After changing it, bump `version` in `crates/djangofmt_corpus/Cargo.toml` and push a `corpus-v<version>` tag on the merged commit.
