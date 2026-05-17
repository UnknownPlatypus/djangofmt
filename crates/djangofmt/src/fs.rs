use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Return the current working directory, cached after the first call.
///
/// On WASM (where `std::env::current_dir` is unsupported) and on the unlikely
/// event of a read failure, falls back to `.`.
pub fn get_cwd() -> &'static Path {
    static CWD: OnceLock<PathBuf> = OnceLock::new();
    CWD.get_or_init(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .as_path()
}

/// Convert an absolute path to be relative to the current working directory.
pub fn relativize_path<P: AsRef<Path>>(path: P) -> String {
    let path = path.as_ref();
    let cwd = get_cwd();
    if let Ok(stripped) = path.strip_prefix(cwd) {
        return stripped.display().to_string();
    }
    path.display().to_string()
}
