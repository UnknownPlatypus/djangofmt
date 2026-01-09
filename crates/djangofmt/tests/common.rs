#![allow(clippy::missing_panics_doc)]
use std::path::Path;

use insta::Settings;

#[must_use]
pub fn build_settings(path: &Path) -> Settings {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path(path.parent().expect("Unable to find parent path"));
    settings.remove_snapshot_suffix();
    settings.set_prepend_module_to_snapshot(false);
    settings.remove_input_file();
    settings.set_omit_expression(true);
    settings.remove_info();
    settings
}
