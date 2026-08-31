# Phase B results — 2026-08-31, commit `4cbc0cc`

Compile-to-slots. Comparison against Phase 0 (`b06202b`) and Phase A
(`356a070`). Same machine, same commands, warm runs.

## Rust — `cargo bench --bench eval_benchmark` (f64 medians)

| Benchmark | Phase 0 | Phase A | Phase B | Total change |
|---|---|---|---|---|
| `a+5` | 150.7 ns | 90.7 ns | 23.7 ns | −84% (6.4x) |
| `(a+5)*2` | 217.2 ns | 127.1 ns | 27.7 ns | −87% (7.8x) |
| `a*a + 2*a + 1` | 317.5 ns | 227.4 ns | 38.3 ns | −88% (8.3x) |
| `sin(a)*cos(a) + sqrt(a+1)` | 406.5 ns | 316.3 ns | 62.8 ns | −85% (6.5x) |
| `1/(a+1)+2/(a+2)+3/(a+3)` | 447.4 ns | 386.4 ns | 62.6 ns | −86% (7.1x) |
| batch: update 10 + eval 7 (heavy) | 4.94 µs | 4.76 µs | 0.95 µs | −81% (5.2x) |

Native closure baselines for the same formulas: 2.7 ns (arithmetic),
7.6 ns (trig). The engine-to-native ratio fell from ~56–165x to ~9–23x.
f32 tracks f64 (batch 0.85 µs).

## Native C — `test_performance` (the FFI path the firmware uses)

| Metric | f64 P0 | f64 PA | f64 PB | f32 PB |
|---|---|---|---|---|
| Batch eval, 7 exprs | 1.776 µs | 1.41 µs | 0.269 µs | 0.277 µs |
| Per expression | 0.254 µs | 0.203 µs | 0.038 µs | 0.040 µs |
| Param update, 10 params | 0.041 µs | 0.13 µs | 0.042 µs | 0.044 µs |
| Full update + eval cycle | 1.894 µs | 1.53 µs | 0.309 µs | 0.308 µs |

The full cycle is 6.1x faster than the Phase 0 baseline. The Phase A
parameter-update cost is gone: with no iterative fallback in the batch,
`set_param` is a plain array write again.

## QEMU — `batch_performance_test` (ticks, wall-clock proxy, coarse)

| Metric | f64 P0 | f64 PB | f32 P0 | f32 PB |
|---|---|---|---|---|
| Full cycle | 65 | 22 | 65 | 38 |

Directionally consistent; DWT on real hardware is the ground truth.

## Iterative path after Phase B (Phase C input)

`arena_consolidated_benchmark`, f64:

- `individual_evaluation` (context clone + `interp` per expression):
  3.48 ms (P0) → 3.02 ms — essentially unchanged, as expected; Phases A/B
  did not touch the per-node iterative machinery.
- `arena_batch_evaluation` (100 ticks x 7 heavy exprs through
  `Expression::eval`): 517.8 µs → 114.1 µs (4.5x).

The iterative evaluator now runs only for: `interp()`/`eval_ast` calls,
expressions with arrays or attributes, and recursive expression functions.
The consumer firmware uses none of these per tick.

## Verification

- `tests/compile_differential_test.rs`: 512-case proptest, compiled result
  == iterative result bit-for-bit (both-NaN equal), plus 13 targeted
  semantics tests. All pass.
- cargo test 15/15 suites in f64 and f32; native C 17/17 both modes;
  QEMU 4/4 both modes (18/18 in-test verification checks).
- `exp_rs.h` byte-identical.
