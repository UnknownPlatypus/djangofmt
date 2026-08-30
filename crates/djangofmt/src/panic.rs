use std::any::Any;
use std::backtrace::BacktraceStatus;
use std::cell::Cell;
use std::panic::Location;
use std::sync::OnceLock;

/// The cause of a panic caught by [`catch_unwind`], with the payload rendered to
/// a string right away so the error stays `Send + Sync` for miette.
#[derive(Debug)]
pub struct PanicError {
    pub location: Option<String>,
    pub payload: String,
    pub backtrace: Option<std::backtrace::Backtrace>,
}

fn payload_to_string(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(ToString::to_string))
        .unwrap_or_else(|| "Box<dyn Any>".to_string())
}

impl std::fmt::Display for PanicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "panicked at")?;
        if let Some(location) = &self.location {
            write!(f, " {location}")?;
        }
        write!(f, ":\n{payload}", payload = self.payload)?;

        if let Some(backtrace) = &self.backtrace {
            match backtrace.status() {
                BacktraceStatus::Disabled => {
                    writeln!(
                        f,
                        "\nrun with `RUST_BACKTRACE=1` environment variable to display a backtrace"
                    )?;
                }
                BacktraceStatus::Captured => {
                    writeln!(f, "\nBacktrace: {backtrace}")?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct CapturedPanicInfo {
    backtrace: Option<std::backtrace::Backtrace>,
    location: Option<String>,
}

thread_local! {
    static CAPTURE_PANIC_INFO: Cell<bool> = const { Cell::new(false) };
    static LAST_PANIC_INFO: Cell<CapturedPanicInfo> = const {
        Cell::new(CapturedPanicInfo { backtrace: None, location: None })
    };
}

/// Install the global hook once; threads outside a [`catch_unwind`] call keep the default one.
fn install_hook() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !CAPTURE_PANIC_INFO.with(Cell::get) {
                return (*prev)(info);
            }
            LAST_PANIC_INFO.set(CapturedPanicInfo {
                backtrace: Some(std::backtrace::Backtrace::capture()),
                location: info.location().map(Location::to_string),
            });
        }));
    });
}

/// Invoke a closure, capturing the cause of a panic without letting the default hook print it.
pub fn catch_unwind<F, R>(f: F) -> Result<R, PanicError>
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
{
    install_hook();
    let prev_should_capture = CAPTURE_PANIC_INFO.replace(true);
    let result = std::panic::catch_unwind(f).map_err(|payload| {
        let CapturedPanicInfo {
            location,
            backtrace,
        } = LAST_PANIC_INFO.take();
        PanicError {
            location,
            payload: payload_to_string(payload.as_ref()),
            backtrace,
        }
    });
    CAPTURE_PANIC_INFO.set(prev_should_capture);
    result
}
