# Benchmarks

<p align="center">
  <picture align="center">
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/user-attachments/assets/3b09a8a2-b5cb-4f1b-a0bc-5f4e3ca169db">
    <source media="(prefers-color-scheme: light)" srcset="https://github.com/user-attachments/assets/88dda91e-cfdd-45a7-a3b4-1f3cc2d0fe95">
    <img alt="Shows a bar chart with benchmark results." src="https://github.com/user-attachments/assets/88dda91e-cfdd-45a7-a3b4-1f3cc2d0fe95">
  </picture>
</p>

<p align="center">
  <i>Formatting 100k+ lines of HTML across 1.7k+ files from scratch.</i>
</p>

This is important to note that only `djlint` covers the same scope in terms of formatting capabilities.
`djade` only alter django templating, `djhtml` only fix indentation and `prettier` only understand html (and **will** break templates)

As always, these results should be taken with a grain of salt.
Results on my machine will differ from yours, especially if you have many CPU cores because some tools take better advantage of parallelization than others.

But at least it was fun to build thanks to the wonderful [hyperfine](https://github.com/sharkdp/hyperfine) tool.

<details>
  <summary>Benchmark details (2025-02-28)</summary>

This was run on my AMD Ryzen 9 7950X (32) @ 5.881GHz.

Tools versions:

- djangofmt: v0.1.0
- prettier: v3.5.2
- djlint: v1.36.4
- djade: v1.3.2
- djhtml: v3.0.7

<pre><code>Benchmark 1: cat /tmp/test-files | xargs --max-procs=0 ../../target/release/djangofmt format --profile django --line-length 120 --quiet
  Time (mean ± σ):      19.8 ms ±   0.9 ms    [User: 179.6 ms, System: 73.7 ms]
  Range (min … max):    18.3 ms …  23.3 ms    73 runs

  Warning: Ignoring non-zero exit code.

Benchmark 2: cat /tmp/test-files | xargs --max-procs=0 djade --target-version 5.1
  Time (mean ± σ):      72.0 ms ±   1.0 ms    [User: 63.2 ms, System: 9.3 ms]
  Range (min … max):    70.5 ms …  73.4 ms    18 runs

Benchmark 3: cat /tmp/test-files | xargs --max-procs=0 djhtml
  Time (mean ± σ):      1.401 s ±  0.026 s    [User: 1.322 s, System: 0.079 s]
  Range (min … max):    1.373 s …  1.453 s    10 runs

Benchmark 4: cat /tmp/test-files | xargs --max-procs=0 djlint --reformat --profile=django --max-line-length 120
  Time (mean ± σ):      2.343 s ±  0.026 s    [User: 64.944 s, System: 1.176 s]
  Range (min … max):    2.297 s …  2.377 s    10 runs

  Warning: Ignoring non-zero exit code.

Benchmark 5: cat /tmp/test-files | xargs --max-procs=0 ./node_modules/.bin/prettier --ignore-unknown --write --print-width 120 --log-level silent
  Time (mean ± σ):      3.226 s ±  0.062 s    [User: 4.481 s, System: 0.261 s]
  Range (min … max):    3.092 s …  3.292 s    10 runs

  Warning: Ignoring non-zero exit code.

Summary
  cat /tmp/test-files | xargs --max-procs=0 ../../target/release/djangofmt format --profile django --line-length 120 --quiet ran
    3.63 ± 0.17 times faster than cat /tmp/test-files | xargs --max-procs=0 djade --target-version 5.1
   70.71 ± 3.45 times faster than cat /tmp/test-files | xargs --max-procs=0 djhtml
  118.28 ± 5.48 times faster than cat /tmp/test-files | xargs --max-procs=0 djlint --reformat --profile=django --max-line-length 120
  162.80 ± 7.96 times faster than cat /tmp/test-files | xargs --max-procs=0 ./node_modules/.bin/prettier --ignore-unknown --write --print-width 120 --log-level silent
</code></pre>
</details>

## Running the benchmarks yourself

See [`python/benchmarks/README.md`](../python/benchmarks/README.md) for the hyperfine recipe and how to reproduce these numbers on your own templates.
