# Phase 0 baseline — 2026-08-31, commit `b06202b`

Baseline for the runtime-evaluation performance work (see PERFORMANCE_PLAN.md
on branch `perf/runtime-eval`). All numbers measured after the Phase 0
benchmark repairs, before any optimization.

Host: Apple Silicon macOS, release profile (opt-level 3, LTO, 1 CGU).
QEMU: mps2-an500 / Cortex-M7 model, CMSDK timer ticks. QEMU is not
cycle-accurate; its ticks track host wall clock at 25 MHz. Use QEMU numbers
only to compare builds against each other. Ground truth for cycles is DWT
measurement on real hardware in the consumer project.

## Rust — `cargo bench --bench eval_benchmark`

Per `Expression::eval` call (set_param + eval + get_result), criterion medians:

| Benchmark | f64 | f32 | Native closure (f64) |
|---|---|---|---|
| `a+5` | 150.7 ns | 143.6 ns | 2.7 ns |
| `(a+5)*2` | 217.2 ns | 188.3 ns | 2.7 ns |
| `a*a + 2*a + 1` | 317.5 ns | 286.9 ns | 2.7 ns |
| `sin(a)*cos(a) + sqrt(a+1)` | 406.5 ns | 377.3 ns | 7.6 ns |
| `1/(a+1)+2/(a+2)+3/(a+3)` | 447.4 ns | 433.7 ns | 3.7 ns |
| batch: update 10 params + eval 7 exprs | 5.00 µs | 4.81 µs | — |

Note (corrected during Phase A): an earlier version of this file claimed the
Rust batch row and the native C batch row below measure "the same shape" and
attributed the 5.0 vs 1.78 µs gap to the per-call BatchParamMap rebuild. That
was wrong — the C benchmark's 7 expressions are much smaller (2–3 operators
each) than this bench's 7 (10–15 operator nodes each), so the two rows are
not comparable. The Phase A controlled A/B put the map rebuild at ~4–5% of
the heavy batch; per-node cost dominates.

## Rust — `cargo bench --bench arena_consolidated_benchmark` (f64)

- `arena_batch_evaluation` (100 ticks x 7 exprs): 517.8 µs → 5.18 µs/tick
- `individual_evaluation` (context clone + interp): 3.48 ms per 100 ticks
- CPU utilization test: arena path 5.2 µs/iteration, 0.7 µs/expression

## Native C — `test_performance` (meson release, host)

| Metric | f64 | f32 |
|---|---|---|
| Batch eval, 7 expressions | 1.776 µs | 1.754 µs |
| Per expression | 0.254 µs | 0.251 µs |
| Param update, 10 params | 0.041 µs | 0.042 µs |
| Full update + eval cycle | 1.894 µs | 1.784 µs |
| Full setup (10 params + 7 exprs parse) | 6.80 µs | — |

## QEMU — `batch_performance_test` (CMSDK ticks, comparison only)

| Metric | f64 | f32 |
|---|---|---|
| Setup per batch (new + 10 vars + 6 exprs) | 196 | 632 |
| Eval per batch (6 expressions) | 25 | 19 |
| Eval per expression | 4 | 3 |
| Param update per cycle (10 params) | 22 | 16 |
| Full cycle (update + eval) | 65 | 65 |

All QEMU runs verify results against C reference math (18/18 checks pass in
both modes).

## How these were produced

- Rust: `cargo bench --bench eval_benchmark` (add `--features f32` for f32);
  `cargo bench --bench arena_consolidated_benchmark`
- Native C: `./run_tests.sh --native [-m f32]`, then run the binary directly:
  `./target/meson/tests_native_c/test_performance`
- QEMU: `./run_tests.sh --qemu [-m f32]`, then run the kernel directly:
  `qemu-system-arm -M mps2-an500 -cpu cortex-m7 -semihosting
  -semihosting-config enable=on,target=native -nographic -monitor none
  -serial stdio -kernel target/meson/qemu_test/batch_performance_test_f64`

Raw outputs are in this directory.
