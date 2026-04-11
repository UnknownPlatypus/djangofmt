/// Returns true if the value contains Jinja/Django interpolation markers.
///
/// Values with `{{` or `{%` are dynamic and should be skipped by most rules.
#[inline]
pub fn contains_interpolation(value: &str) -> bool {
    value.contains("{{") || value.contains("{%")
}
