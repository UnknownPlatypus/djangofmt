//! Helpers and the allocator shared by the divan micro-benchmarks.

use djangofmt_corpus::{Syntax, Template};
use djangofmt_syntax::Profile;

// Benchmark the allocator the CLI ships with, minus its deferred work: page purging, and the
// tcache GC, whose flush lands inside a random measurement window. 1TiB, not 0, disables the GC:
// jemalloc clips `tcache_gc_incr_bytes` up to a 1KiB floor, so 0 would fire it 64x more often.
#[cfg(all(
    not(target_os = "macos"),
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "aix"),
    not(target_os = "android"),
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64",
        target_arch = "riscv64"
    )
))]
mod allocator {
    #[global_allocator]
    static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

    // jemalloc reads this symbol as a `const char *`, so it must be a thin pointer, not a `&[u8]`.
    #[expect(
        unsafe_code,
        reason = "jemalloc reads its config from this exported symbol"
    )]
    #[unsafe(export_name = "_rjem_malloc_conf")]
    pub static MALLOC_CONF: &u8 =
        &b"dirty_decay_ms:-1,muzzy_decay_ms:-1,tcache_gc_incr_bytes:1099511627776\0"[0];
}

/// Directives no corpus template carries: the bare *formatter* one, which the linter must
/// leave free, and a bracketed lint code list, which it must resolve to a guarded node.
pub const FORMATTER_DIRECTIVE: &str = "{# djangofmt:ignore #}";
pub const LINT_DIRECTIVE: &str = "{# djangofmt: ignore[empty-attr-value] #}";

/// A corpus template with `directive` in front of its body, guarding the node that follows.
#[must_use]
pub fn with_directive(template: &Template, directive: &str) -> String {
    format!("<p>head</p>\n{directive}\n{}", template.code)
}

/// Unmeasured runs before the measured one, mirroring `codspeed::codspeed::WARMUP_RUNS`.
/// `codspeed-criterion-compat` applies these natively; `codspeed-divan-compat` does not.
const WARMUP_RUNS: usize = 5;

/// Fills process-global caches (rustywind's class-sort LRU, `LazyLock` tables) before the
/// measured call, which `codspeed-divan-compat` instruments as a single cold iteration.
pub fn warmup<T>(f: impl Fn() -> T) {
    for _ in 0..WARMUP_RUNS {
        std::hint::black_box(f());
    }
}

/// The profile to parse a corpus template with.
#[must_use]
pub const fn profile(template: &Template) -> Profile {
    match template.syntax {
        Syntax::Django => Profile::Django,
        Syntax::Jinja => Profile::Jinja,
    }
}
