# Benchmarks

<p align="center">
  <picture align="center">
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/user-attachments/assets/806ad1c2-a2c7-4177-9651-1857b43aff0d">
    <source media="(prefers-color-scheme: light)" srcset="https://github.com/user-attachments/assets/7e132145-7738-4cd1-8cdc-dab6e87175b4">
    <img alt="Shows a bar chart with benchmark results." src="https://github.com/user-attachments/assets/7e132145-7738-4cd1-8cdc-dab6e87175b4">
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
  <summary>Benchmark details (2026-08-29)</summary>

This was run on my AMD Ryzen 9 7950X (32) @ 5.881GHz.

Tools versions:

- djangofmt: v0.2.12
- prettier: v3.9.6
- djlint: v1.44.2
- djade: v1.9.0
- djhtml: v3.0.11

<pre><code>Benchmark 1: cat /tmp/files-list | xargs --max-procs=0 ../../target/release/djangofmt --profile django --line-length 120 --quiet
  Time (mean ± σ):      19.4 ms ±   1.3 ms    [User: 118.5 ms, System: 49.0 ms]
  Range (min … max):    16.5 ms …  23.1 ms    70 runs

  Warning: Ignoring non-zero exit code.

Benchmark 2: cat /tmp/files-list | xargs --max-procs=0 djade --target-version 5.1
  Time (mean ± σ):      74.1 ms ±   1.2 ms    [User: 64.2 ms, System: 10.6 ms]
  Range (min … max):    72.7 ms …  76.9 ms    17 runs

Benchmark 3: cat /tmp/files-list | xargs --max-procs=0 djhtml
  Time (mean ± σ):      1.468 s ±  0.013 s    [User: 1.366 s, System: 0.102 s]
  Range (min … max):    1.454 s …  1.496 s    10 runs

Benchmark 4: cat /tmp/files-list | xargs --max-procs=0 djlint --reformat --profile=django --max-line-length 120
  Time (mean ± σ):      2.155 s ±  0.056 s    [User: 54.558 s, System: 1.608 s]
  Range (min … max):    2.084 s …  2.277 s    10 runs

  Warning: Ignoring non-zero exit code.

Benchmark 5: cat /tmp/files-list | xargs --max-procs=0 ./node_modules/.bin/prettier --ignore-unknown --write --print-width 120 --log-level silent
  Time (mean ± σ):      3.625 s ±  0.101 s    [User: 5.044 s, System: 0.264 s]
  Range (min … max):    3.515 s …  3.850 s    10 runs

  Warning: Ignoring non-zero exit code.

Summary
  cat /tmp/files-list | xargs --max-procs=0 ../../target/release/djangofmt --profile django --line-length 120 --quiet ran
    3.82 ± 0.26 times faster than cat /tmp/files-list | xargs --max-procs=0 djade --target-version 5.1
   75.55 ± 5.06 times faster than cat /tmp/files-list | xargs --max-procs=0 djhtml
  110.96 ± 7.90 times faster than cat /tmp/files-list | xargs --max-procs=0 djlint --reformat --profile=django --max-line-length 120
  186.64 ± 13.43 times faster than cat /tmp/files-list | xargs --max-procs=0 ./node_modules/.bin/prettier --ignore-unknown --write --print-width 120 --log-level silent
</code></pre>
</details>

## Running the benchmarks yourself

See [`python/benchmarks/README.md`](https://github.com/UnknownPlatypus/djangofmt/blob/main/python/benchmarks/README.md) for the hyperfine recipe and how to reproduce these numbers on your own templates.
