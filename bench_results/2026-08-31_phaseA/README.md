# Phase A results — 2026-08-31, commit `356a070`

Comparison against the Phase 0 baseline (`bench_results/2026-08-31_phase0_baseline/`,
commit `b06202b`). Same machine, same commands.

## Rust — `cargo bench --bench eval_benchmark`

Per `Expression::eval` call (set_param + eval + get_result), criterion medians, f64:

| Benchmark | Phase 0 | Phase A | Change |
|---|---|---|---|
| `a+5` | 150.7 ns | 90.7 ns | −40% |
| `(a+5)*2` | 217.2 ns | 127.1 ns | −41% |
| `a*a + 2*a + 1` | 317.5 ns | 227–238 ns | −27% |
| `sin(a)*cos(a) + sqrt(a+1)` | 406.5 ns | 316–322 ns | −21% |
| `1/(a+1)+2/(a+2)+3/(a+3)` | 447.4 ns | 386–390 ns | −13% |
| batch: update 10 + eval 7 | 4.93–4.96 µs * | 4.75–4.76 µs * | −4% |

\* The batch row comes from a controlled A/B: isolated runs, alternating
between a worktree at the baseline commit and the Phase A tree, because
end-of-suite runs read up to 15% high on this machine (thermal / scheduling).
The single-expression rows are criterion's own before/after on identical
conditions.

f32 tracks f64 within a few percent (see raw files).

## Native C — `test_performance` (warm run; first run after build reads high)

| Metric | f64 Phase 0 | f64 Phase A | f32 Phase 0 | f32 Phase A |
|---|---|---|---|---|
| Batch eval, 7 exprs | 1.776 µs | 1.38–1.43 µs (−20%) | 1.754 µs | 1.42 µs (−19%) |
| Param update, 10 params | 0.041 µs | 0.125–0.15 µs | 0.042 µs | 0.143 µs |
| Full update + eval cycle | 1.894 µs | 1.51–1.57 µs (−19%) | 1.784 µs | 1.60 µs (−11%) |

The parameter-update increase is the A3 design working as intended: values now
write through to the engine's override map (one hash lookup per update), so
`eval` no longer rebuilds the map. Net effect on the update+eval cycle is
−19% (f64). Phase B's slot compilation makes `set_param` a plain array write
again.

## QEMU — `batch_performance_test` (CMSDK ticks, wall-clock proxy, coarse)

| Metric | f64 P0 | f64 PA | f32 P0 | f32 PA |
|---|---|---|---|---|
| Eval per batch of 6 | 25 | 24 | 19 | 23 |
| Param update per cycle | 22 | 32 | 16 | 21 |
| Full cycle | 65 | 65 | 65 | 50 |

At this tick resolution (1 tick = 40 ns of host wall time) these are within
noise; the native C table is the meaningful host-side comparison. Ground
truth for the target remains the consumer project's DWT measurement.

## Correction to a Phase 0 note

The Phase 0 baseline README claimed the Rust batch path (5.0 µs) was "the
same shape" as the native C 1.78 µs batch and attributed the gap to the
BatchParamMap rebuild. That was wrong: the C benchmark's 7 expressions are
much smaller (2–3 operators each) than the Rust bench's 7 (10–15 operator
nodes each). The workloads are not comparable. The map rebuild was worth
~4–5% on the heavy batch, as this phase's controlled A/B shows; per-node
evaluation cost dominates the rest, which is what Phase B/C target.

## Also fixed in this phase

`expr_context_add_function` never validated a NULL function pointer; it
"rejected" NULL only because the engine kept a clone of the last-used context
alive, making `Rc::get_mut` fail with -4 for ANY registration after an
evaluate. Registration after an evaluate now works, and NULL is rejected
explicitly. `exp_rs.h` is byte-identical.
