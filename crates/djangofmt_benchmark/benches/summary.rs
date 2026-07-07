use djangofmt::commands::format::{FormatResult, build_summary};

fn main() {
    divan::main();
}

fn make_results(formatted: usize, unchanged: usize) -> Vec<FormatResult> {
    let mut results = Vec::with_capacity(formatted + unchanged);
    for _ in 0..formatted {
        results.push(FormatResult::Formatted);
    }
    for _ in 0..unchanged {
        results.push(FormatResult::Unchanged);
    }
    results
}

#[divan::bench(args = [10, 100, 1000, 10_000])]
fn build_summary_mixed(bencher: divan::Bencher, n: usize) {
    let results = make_results(n / 2, n / 3);

    bencher.bench(|| build_summary(divan::black_box(&results)));
}
